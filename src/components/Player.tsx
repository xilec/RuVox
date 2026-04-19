import { useEffect, useState, useCallback } from 'react';
import { ActionIcon, Group, Slider, Text, NumberInput, Tooltip } from '@mantine/core';
import { useHotkeys } from '@mantine/hooks';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { commands, events } from '../lib/tauri';
import type { EntryId } from '../lib/tauri';
import classes from './Player.module.css';

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
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

interface PlayerProps {
  // entryIds is provided by U2 QueueList when available; allows Prev/Next navigation.
  entryIds?: EntryId[];
}

export function Player({ entryIds }: PlayerProps) {
  const [state, setState] = useState<PlayerState>(INITIAL_STATE);
  // Tracks whether the user is actively dragging the progress slider so we
  // don't override the drag value with incoming playback_position events.
  const [isDragging, setIsDragging] = useState(false);
  const [dragValue, setDragValue] = useState(0);

  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];

    unlisteners.push(
      events.playbackStarted((p) => {
        setState((prev) => ({
          ...prev,
          isPlaying: true,
          isPaused: false,
          position: 0,
          currentEntryId: p.entry_id,
        }));
      }),
    );

    unlisteners.push(
      events.playbackPaused((p) => {
        setState((prev) => ({
          ...prev,
          isPlaying: false,
          isPaused: true,
          position: p.position_sec,
        }));
      }),
    );

    unlisteners.push(
      events.playbackStopped(() => {
        setState((prev) => ({
          ...prev,
          isPlaying: false,
          isPaused: false,
          position: 0,
          currentEntryId: null,
        }));
      }),
    );

    unlisteners.push(
      events.playbackFinished(() => {
        setState((prev) => ({
          ...prev,
          isPlaying: false,
          isPaused: false,
          position: 0,
        }));
      }),
    );

    unlisteners.push(
      events.playbackPosition((p) => {
        setState((prev) => {
          // Keep duration updated from position events when entry changes
          const next: PlayerState = { ...prev, currentEntryId: p.entry_id };
          if (!isDragging) {
            next.position = p.position_sec;
          }
          return next;
        });
      }),
    );

    // Sync duration from entry_updated events
    unlisteners.push(
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
    );

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
    // isDragging is intentionally excluded: the closure only reads it via
    // setState callback (captures latest via closure-over-ref pattern would
    // require a ref; using a state callback avoids stale closure issues).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handlePlayPause = useCallback(async () => {
    if (state.isPlaying) {
      await commands.pausePlayback();
    } else if (state.isPaused) {
      await commands.resumePlayback();
    } else if (state.currentEntryId) {
      await commands.playEntry(state.currentEntryId);
    }
  }, [state.isPlaying, state.isPaused, state.currentEntryId]);

  const handleSeek = useCallback(
    async (positionSec: number) => {
      if (state.isPlaying || state.isPaused) {
        await commands.seekTo(positionSec);
      }
    },
    [state.isPlaying, state.isPaused],
  );

  const handleSpeedChange = useCallback(async (value: number | string) => {
    const speed = typeof value === 'string' ? parseFloat(value) : value;
    if (isNaN(speed)) return;
    const clamped = Math.min(2.0, Math.max(0.5, speed));
    setState((prev) => ({ ...prev, speed: clamped }));
    await commands.setSpeed(clamped);
  }, []);

  const handleVolumeChange = useCallback(async (volume: number) => {
    setState((prev) => ({ ...prev, volume }));
    await commands.setVolume(volume);
  }, []);

  const handlePrev = useCallback(async () => {
    if (!entryIds || entryIds.length === 0) return;
    const idx = state.currentEntryId ? entryIds.indexOf(state.currentEntryId) : -1;
    const prevIdx = idx > 0 ? idx - 1 : entryIds.length - 1;
    const prevId = entryIds[prevIdx];
    if (prevId) {
      await commands.playEntry(prevId);
    }
  }, [entryIds, state.currentEntryId]);

  const handleNext = useCallback(async () => {
    if (!entryIds || entryIds.length === 0) return;
    const idx = state.currentEntryId ? entryIds.indexOf(state.currentEntryId) : -1;
    const nextIdx = idx >= 0 && idx < entryIds.length - 1 ? idx + 1 : 0;
    const nextId = entryIds[nextIdx];
    if (nextId) {
      await commands.playEntry(nextId);
    }
  }, [entryIds, state.currentEntryId]);

  // Keyboard shortcuts
  useHotkeys([
    ['Space', (e) => { e.preventDefault(); void handlePlayPause(); }],
    ['ArrowLeft', (e) => { e.preventDefault(); void handleSeek(Math.max(0, state.position - 5)); }],
    ['ArrowRight', (e) => { e.preventDefault(); void handleSeek(Math.min(state.duration || 0, state.position + 5)); }],
  ]);

  const playIcon = state.isPlaying
    ? '\u23F8' // pause
    : '\u25B6'; // play

  const progressValue = isDragging ? dragValue : state.position;
  const maxProgress = state.duration > 0 ? state.duration : 1;

  return (
    <Group className={classes.root} gap="xs" wrap="nowrap">
      {entryIds && entryIds.length > 0 && (
        <Tooltip label="Предыдущий">
          <ActionIcon
            className={classes.navButton}
            variant="subtle"
            onClick={() => { void handlePrev(); }}
            aria-label="Предыдущий"
            size="sm"
          >
            &#x23EE;
          </ActionIcon>
        </Tooltip>
      )}

      <ActionIcon
        className={classes.playButton}
        variant="filled"
        onClick={() => { void handlePlayPause(); }}
        aria-label={state.isPlaying ? 'Пауза' : 'Воспроизвести'}
        size="md"
        disabled={!state.currentEntryId && (!entryIds || entryIds.length === 0)}
      >
        {playIcon}
      </ActionIcon>

      {entryIds && entryIds.length > 0 && (
        <Tooltip label="Следующий">
          <ActionIcon
            className={classes.navButton}
            variant="subtle"
            onClick={() => { void handleNext(); }}
            aria-label="Следующий"
            size="sm"
          >
            &#x23ED;
          </ActionIcon>
        </Tooltip>
      )}

      <Slider
        className={classes.progressSlider}
        min={0}
        max={maxProgress}
        value={progressValue}
        onChange={(v) => {
          setIsDragging(true);
          setDragValue(v);
        }}
        onChangeEnd={(v) => {
          setIsDragging(false);
          setState((prev) => ({ ...prev, position: v }));
          void handleSeek(v);
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

      <Tooltip label="Скорость (0.5x–2.0x)">
        <NumberInput
          className={classes.speedInput}
          value={state.speed}
          onChange={(v) => { void handleSpeedChange(v); }}
          min={0.5}
          max={2.0}
          step={0.1}
          decimalScale={1}
          fixedDecimalScale
          size="xs"
          aria-label="Скорость воспроизведения"
          suffix="x"
          hideControls={false}
        />
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
    </Group>
  );
}
