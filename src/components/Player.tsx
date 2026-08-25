import { useEffect, useState, useCallback, useRef } from 'react';
import { ActionIcon, Group, Slider, Text, NumberInput, Tooltip } from '@mantine/core';
import { useHotkeys } from '@mantine/hooks';
import { notifications } from '@mantine/notifications';
import { commands, events, clampSpeed, MAX_SPEED, MIN_SPEED } from '../lib/tauri';
import type { EntryId } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { useTauriEvents } from '../lib/useTauriEvents';
import { IconPlay, IconPause, IconSettings, IconAppLogo } from './icons';
import classes from './Player.module.css';

function showPlayerError(action: string, err: unknown): void {
  notifications.show({
    title: 'Ошибка',
    message: `Не удалось ${action}: ${formatError(err)}`,
    color: 'red',
  });
}

interface PlayerState {
  isPlaying: boolean;
  isPaused: boolean;
  position: number;
  duration: number;
  speed: number;
  volume: number;
  currentEntryId: EntryId | null;
}

const INITIAL_STATE: PlayerState = {
  isPlaying: false,
  isPaused: false,
  position: 0,
  duration: 0,
  speed: 1.0,
  volume: 1.0,
  currentEntryId: null,
};

function formatTime(sec: number): string {
  const total = Math.max(0, Math.floor(sec));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const mm = String(m).padStart(2, '0');
  const ss = String(s).padStart(2, '0');
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
}

interface PlayerProps {
  onOpenSettings?: () => void;
}

export function Player({ onOpenSettings }: PlayerProps = {}) {
  const [state, setState] = useState<PlayerState>(INITIAL_STATE);
  // Ref (not state) because the playback_position listener closes over it
  // once on mount; a ref avoids resubscribing on every drag.
  const draggingRef = useRef(false);
  const speedWrapperRef = useRef<HTMLDivElement>(null);
  const speedRef = useRef(state.speed);
  const volumeRef = useRef(state.volume);
  useEffect(() => { speedRef.current = state.speed; }, [state.speed]);

  useTauriEvents([
    () =>
      events.playbackStarted((p) => {
        setState((prev) => ({
          ...prev,
          isPlaying: true,
          isPaused: false,
          // Preserve position on resume (playback_started now fires on resume
          // too); only reset when the entry actually changes.
          position: prev.currentEntryId === p.entry_id ? prev.position : 0,
          currentEntryId: p.entry_id,
          duration: p.duration_sec ?? prev.duration,
        }));
      }),
    () =>
      events.playbackPaused((p) => {
        setState((prev) => ({
          ...prev,
          isPlaying: false,
          isPaused: true,
          position: p.position_sec,
        }));
      }),
    () =>
      events.playbackStopped(() => {
        setState((prev) => ({
          ...prev,
          isPlaying: false,
          isPaused: false,
          position: 0,
          currentEntryId: null,
        }));
      }),
    () =>
      events.playbackFinished(() => {
        setState((prev) => ({
          ...prev,
          isPlaying: false,
          isPaused: false,
          position: 0,
        }));
      }),
    () =>
      events.playbackPosition((p) => {
        setState((prev) => {
          const next: PlayerState = { ...prev, currentEntryId: p.entry_id };
          if (!draggingRef.current) {
            next.position = p.position_sec;
          }
          // mpv needs a few ticks to parse the WAV header; duration comes
          // through here as soon as it is available.
          if (p.duration_sec != null && prev.duration !== p.duration_sec) {
            next.duration = p.duration_sec;
          }
          return next;
        });
      }),
    () =>
      events.entryUpdated((p) => {
        setState((prev) => {
          if (
            prev.currentEntryId === p.entry.id &&
            p.entry.duration_sec !== null
          ) {
            return { ...prev, duration: p.entry.duration_sec };
          }
          return prev;
        });
      }),
  ]);

  // Restore the persisted playback speed (#227): set_speed persists to
  // UIConfig.speech_rate, but nothing applied it on start, so every launch
  // began at 1.0x.  The restored value must also reach the backend — a fresh
  // mpv process starts at 1.0x, so updating only the UI would desync the
  // slider from actual playback.  A config read failure is non-fatal — keep
  // 1.0x; a user who already touched the speed during startup wins over the
  // late-arriving config.
  useEffect(() => {
    let cancelled = false;
    commands
      .getConfig()
      .then((config) => {
        if (cancelled || speedRef.current !== INITIAL_STATE.speed) return;
        const clamped = clampSpeed(config.speech_rate);
        setState((prev) => ({ ...prev, speed: clamped }));
        void commands.setSpeed(clamped).catch((err) =>
          console.warn('failed to apply restored speed:', err),
        );
      })
      .catch((err) => console.warn('failed to load config for speed restore:', err));
    return () => { cancelled = true; };
  }, []);

  const handlePlayPause = useCallback(async () => {
    try {
      if (state.isPlaying) {
        await commands.pausePlayback();
      } else if (state.isPaused) {
        await commands.resumePlayback();
      } else if (state.currentEntryId) {
        await commands.playEntry(state.currentEntryId);
      }
    } catch (err) {
      showPlayerError('управлять воспроизведением', err);
    }
  }, [state.isPlaying, state.isPaused, state.currentEntryId]);

  const handleSeek = useCallback(
    async (positionSec: number) => {
      if (state.isPlaying || state.isPaused) {
        try {
          await commands.seekTo(positionSec);
        } catch (err) {
          showPlayerError('перемотать', err);
        }
      }
    },
    [state.isPlaying, state.isPaused],
  );

  const handleSpeedChange = useCallback(async (value: number | string) => {
    const speed = typeof value === 'string' ? parseFloat(value) : value;
    if (isNaN(speed)) return;
    const clamped = clampSpeed(speed);
    const prevSpeed = speedRef.current;
    setState((prev) => ({ ...prev, speed: clamped }));
    try {
      await commands.setSpeed(clamped);
    } catch (err) {
      setState((prev) => ({ ...prev, speed: prevSpeed }));
      showPlayerError('изменить скорость', err);
    }
  }, []);

  // Native (non-passive) wheel listener: React's synthetic onWheel binds at
  // the root with passive=true, so e.preventDefault() inside it is a no-op
  // and the page still scrolls.  addEventListener with passive:false fixes it.
  useEffect(() => {
    const el = speedWrapperRef.current;
    if (!el) return;
    const handler = (e: WheelEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const delta = e.deltaY > 0 ? -0.1 : 0.1;
      void handleSpeedChange(
        Math.round((speedRef.current + delta) * 10) / 10,
      );
    };
    el.addEventListener('wheel', handler, { passive: false });
    return () => el.removeEventListener('wheel', handler);
  }, [handleSpeedChange]);

  const handleVolumeChange = useCallback(async (volume: number) => {
    // Rollback baseline is the last successfully committed volume (ref),
    // not the current state: during a drag the slider already wrote the
    // new value into state before onChangeEnd fires, so state cannot serve
    // as the "previous" value if the IPC call fails.
    const prevVolume = volumeRef.current;
    setState((prev) => ({ ...prev, volume }));
    try {
      await commands.setVolume(volume);
      volumeRef.current = volume;
    } catch (err) {
      setState((prev) => ({ ...prev, volume: prevVolume }));
      showPlayerError('изменить громкость', err);
    }
  }, []);

  // Keyboard shortcuts
  useHotkeys([
    ['Space', (e) => { e.preventDefault(); void handlePlayPause(); }],
    ['ArrowLeft', (e) => { e.preventDefault(); void handleSeek(Math.max(0, state.position - 5)); }],
    ['ArrowRight', (e) => { e.preventDefault(); void handleSeek(Math.min(state.duration || 0, state.position + 5)); }],
  ]);

  const maxProgress = state.duration > 0 ? state.duration : 1;

  return (
    <Group className={classes.root} gap="xs" wrap="nowrap">
      <IconAppLogo size={48} className={classes.appLogo} />

      <ActionIcon
        className={classes.playButton}
        variant="filled"
        onClick={() => { void handlePlayPause(); }}
        aria-label={state.isPlaying ? 'Пауза' : 'Воспроизвести'}
        size="md"
        disabled={!state.currentEntryId}
      >
        {state.isPlaying ? <IconPause /> : <IconPlay />}
      </ActionIcon>

      <Slider
        className={classes.progressSlider}
        min={0}
        max={maxProgress}
        value={state.position}
        onChange={(v) => {
          // Optimistic update while dragging; playback_position listener
          // skips overwriting state.position while draggingRef.current=true.
          draggingRef.current = true;
          setState((prev) => ({ ...prev, position: v }));
        }}
        onChangeEnd={(v) => {
          // Keep draggingRef=true across the seekTo IPC roundtrip so the
          // next position-emitter tick cannot slip in with the pre-seek
          // time-pos before Player::seek has set up its own suppress
          // window on the Rust side.
          setState((prev) => ({ ...prev, position: v }));
          void handleSeek(v).finally(() => {
            draggingRef.current = false;
          });
        }}
        label={formatTime}
        step={0.1}
        aria-label="Позиция воспроизведения"
        size="sm"
        disabled={state.duration === 0}
      />

      <Text className={classes.timeDisplay}>
        {formatTime(state.position)} / {formatTime(state.duration)}
      </Text>

      <Tooltip label={`Скорость (${MIN_SPEED}x–${MAX_SPEED}x)`}>
        <div ref={speedWrapperRef} className={classes.speedInputWrapper}>
        <NumberInput
          className={classes.speedInput}
          value={state.speed}
          onChange={(v) => { void handleSpeedChange(v); }}
          min={MIN_SPEED}
          max={MAX_SPEED}
          step={0.1}
          decimalScale={1}
          fixedDecimalScale
          size="xs"
          aria-label="Скорость воспроизведения"
          suffix="x"
          hideControls={false}
        />
        </div>
      </Tooltip>

      <Tooltip label="Громкость">
        <Slider
          className={classes.volumeSlider}
          min={0}
          max={1}
          step={0.05}
          value={state.volume}
          onChange={(v) => {
            setState((prev) => ({ ...prev, volume: v }));
          }}
          onChangeEnd={(v) => { void handleVolumeChange(v); }}
          label={(v) => `${Math.round(v * 100)}%`}
          aria-label="Громкость"
          size="sm"
        />
      </Tooltip>

      {onOpenSettings && (
        <Tooltip label="Настройки">
          <ActionIcon
            variant="subtle"
            aria-label="Открыть настройки"
            onClick={onOpenSettings}
          >
            <IconSettings size={18} />
          </ActionIcon>
        </Tooltip>
      )}
    </Group>
  );
}
