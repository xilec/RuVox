import { useEffect, useState, useCallback, useMemo, useRef } from 'react';
import {
  Stack,
  Group,
  Text,
  Badge,
  ActionIcon,
  ScrollArea,
  Loader,
  Menu,
  Button,
} from '@mantine/core';
import { modals } from '@mantine/modals';
import { notifications } from '@mantine/notifications';
import { commands, events } from '../lib/tauri';
import type { TextEntry, EntryStatus, EntryId } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { useT } from '../lib/i18n';
import type { MessageKey } from '../i18n/ru';
import { useTauriEvents } from '../lib/useTauriEvents';
import { useSelectedEntry } from '../stores/selectedEntry';
import { useSearchQuery } from '../stores/searchQuery';
import { formatDuration } from '../lib/format';
import { GenerationParamsDialog } from '../dialogs/GenerationParamsDialog';
import { IconPlay, IconLocate } from './icons';
import classes from './QueueList.module.css';

function statusBadgeColor(status: EntryStatus): string {
  switch (status) {
    case 'pending':
      return 'gray';
    case 'processing':
      return 'blue';
    case 'ready':
      return 'green';
    case 'playing':
      return 'teal';
    case 'error':
      return 'red';
    case 'cancelled':
      return 'gray';
  }
}

const STATUS_KEY: Record<EntryStatus, MessageKey> = {
  pending: 'queue.status.pending',
  processing: 'queue.status.processing',
  ready: 'queue.status.ready',
  playing: 'queue.status.playing',
  error: 'queue.status.error',
  cancelled: 'queue.status.cancelled',
};

interface QueueItemProps {
  entry: TextEntry;
  isSelected: boolean;
  isPlaying: boolean;
  onSelect: (entry: TextEntry) => void;
  onPlay: (id: string) => void;
  onContextMenu: (entry: TextEntry, x: number, y: number) => void;
  onOpenParams: (entry: TextEntry) => void;
}

function QueueItem({ entry, isSelected, isPlaying, onSelect, onPlay, onContextMenu, onOpenParams }: QueueItemProps) {
  const tt = useT();
  const preview = entry.original_text.slice(0, 60);
  const isProcessing = entry.status === 'processing';
  const canPlay = entry.status === 'ready' || entry.status === 'playing';

  const itemClass = [
    classes.item,
    isSelected ? classes.itemSelected : '',
    isPlaying ? classes.itemPlaying : '',
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div
      className={itemClass}
      data-entry-id={entry.id}
      onClick={() => onSelect(entry)}
      onDoubleClick={() => onOpenParams(entry)}
      onContextMenu={(e) => {
        e.preventDefault();
        e.stopPropagation();
        onContextMenu(entry, e.clientX, e.clientY);
      }}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onSelect(entry);
        }
      }}
    >
      <Group justify="space-between" gap="xs" wrap="nowrap">
        <Stack gap={2} style={{ minWidth: 0, flex: 1 }}>
          <Text className={classes.preview} title={entry.original_text}>
            {preview}
            {entry.original_text.length > 60 ? '\u2026' : ''}
          </Text>
          <Group gap="xs" align="center">
            <Badge
              size="xs"
              color={statusBadgeColor(entry.status)}
              leftSection={isProcessing ? <Loader size={8} color="blue" /> : null}
            >
              {tt(STATUS_KEY[entry.status])}
            </Badge>
            {entry.duration_sec != null && (
              <Text className={classes.meta}>{formatDuration(entry.duration_sec)}</Text>
            )}
          </Group>
        </Stack>

        <Group gap="xs" className={classes.actions} wrap="nowrap">
          <ActionIcon
            size="sm"
            variant="subtle"
            color="green"
            disabled={!canPlay}
            title={tt('queue.play')}
            onClick={(e) => {
              e.stopPropagation();
              onPlay(entry.id);
            }}
            aria-label={tt('queue.play')}
          >
            <IconPlay />
          </ActionIcon>
        </Group>
      </Group>
    </div>
  );
}

interface QueueListProps {
  /** Called by «Перегенерировать аудио»: hands the entry to AppShell, which
   * opens the regeneration preview instead of synthesizing right away
   * (preview-dialog spec, "Regeneration preview"). */
  onRegenerate: (entry: TextEntry) => void;
}

export function QueueList({ onRegenerate }: QueueListProps) {
  const tt = useT();
  const [entries, setEntries] = useState<TextEntry[]>([]);
  const [playingId, setPlayingId] = useState<EntryId | null>(null);
  const [playingVisible, setPlayingVisible] = useState(true);
  const viewportRef = useRef<HTMLDivElement>(null);
  const { selectedId, setSelectedEntry } = useSelectedEntry();
  const { query } = useSearchQuery();
  const filteredEntries = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return entries;
    return entries.filter((e) => e.original_text.toLowerCase().includes(q));
  }, [entries, query]);
  // Single Menu instance shared by all queue items — cheaper than one per item
  // and avoids stacking many hidden Menu portals that can interfere with other
  // popovers (e.g. the theme dropdown in the header).
  const [menu, setMenu] = useState<{ id: string; x: number; y: number } | null>(
    null,
  );
  // Entry whose generation snapshot the params dialog shows (id, so the
  // dialog sees live entry updates while open).
  const [paramsEntryId, setParamsEntryId] = useState<EntryId | null>(null);
  const paramsEntry = paramsEntryId
    ? (entries.find((e) => e.id === paramsEntryId) ?? null)
    : null;
  // Resolve the live entry at render time: the status may change while the
  // menu is open (e.g. synthesis finishes), and actions must see the current
  // state, not the snapshot taken at right-click time.
  const menuEntry = menu
    ? (entries.find((e) => e.id === menu.id) ?? null)
    : null;

  const loadEntries = useCallback(async () => {
    const result = await commands.getEntries();
    // Sort by created_at desc (newest first). Backend guarantees this order
    // per the IPC contract, but local sort guards against inconsistent updates.
    result.sort((a, b) => b.created_at.localeCompare(a.created_at));
    setEntries(result);
  }, []);

  useEffect(() => {
    void loadEntries();
  }, [loadEntries]);

  useTauriEvents([
    () =>
      events.entryUpdated((payload) => {
        setEntries((prev) => {
          const idx = prev.findIndex((e) => e.id === payload.entry.id);
          let next: TextEntry[];
          if (idx === -1) {
            // New entry — prepend and re-sort to maintain desc order.
            next = [payload.entry, ...prev];
            next.sort((a, b) => b.created_at.localeCompare(a.created_at));
          } else {
            next = [...prev];
            next[idx] = payload.entry;
          }
          return next;
        });

        // Keep the selected entry in sync so TextViewer reflects latest status
        // without a separate get_entry invoke.
        useSelectedEntry.setState((state) => {
          if (state.selectedId === payload.entry.id) {
            return { selectedEntry: payload.entry };
          }
          return {};
        });
      }),
    () =>
      events.entryRemoved((payload) => {
        setEntries((prev) => prev.filter((e) => e.id !== payload.id));
        useSelectedEntry.setState((state) =>
          state.selectedId === payload.id
            ? { selectedId: null, selectedEntry: null }
            : {},
        );
      }),
    // Highlight the currently-playing entry.  Paused playback keeps the
    // highlight (user may resume); only stop/finish clears it.
    () => events.playbackStarted((p) => setPlayingId(p.entry_id)),
    () => events.playbackStopped(() => setPlayingId(null)),
    () => events.playbackFinished(() => setPlayingId(null)),
  ]);

  // Track whether the playing entry is currently visible in the viewport so we
  // only surface the "jump to current" button when the user has scrolled away.
  useEffect(() => {
    if (!playingId || !viewportRef.current) {
      setPlayingVisible(true);
      return;
    }
    const target = viewportRef.current.querySelector<HTMLElement>(
      `[data-entry-id="${CSS.escape(playingId)}"]`,
    );
    if (!target) {
      setPlayingVisible(true);
      return;
    }
    const observer = new IntersectionObserver(
      ([entry]) => setPlayingVisible(entry.intersectionRatio >= 1),
      { root: viewportRef.current, threshold: [0, 1] },
    );
    observer.observe(target);
    return () => observer.disconnect();
  }, [playingId, filteredEntries]);

  const handleJumpToPlaying = useCallback(() => {
    if (!playingId || !viewportRef.current) return;
    if (selectedId !== playingId) {
      const playingEntry = entries.find((e) => e.id === playingId);
      if (playingEntry) setSelectedEntry(playingEntry);
    }
    const target = viewportRef.current.querySelector<HTMLElement>(
      `[data-entry-id="${CSS.escape(playingId)}"]`,
    );
    target?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }, [playingId, selectedId, entries, setSelectedEntry]);

  const handlePlay = useCallback(async (id: string) => {
    try {
      await commands.playEntry(id);
    } catch (err) {
      notifications.show({
        title: tt('errors.title'),
        message: tt('queue.notify.play_failed', [formatError(err)]),
        color: 'red',
      });
    }
  }, [tt]);

  const handleCancelSynthesis = useCallback(async (id: string) => {
    try {
      await commands.cancelSynthesis(id);
      notifications.show({
        title: tt('queue.notify.cancelled.title'),
        message: tt('queue.notify.cancelled.message'),
        color: 'green',
      });
    } catch (e) {
      notifications.show({
        title: tt('errors.title'),
        message: tt('queue.notify.cancel_failed', [formatError(e)]),
        color: 'red',
      });
    }
  }, [tt]);

  // "Save audio as…": pick a target with the native save dialog, then copy
  // the cached audio there. A cancelled dialog is a silent no-op; any
  // rejection (dialog or copy) surfaces the localized red error.
  const handleExportAudio = useCallback(
    async (id: string) => {
      try {
        const path = await commands.pickExportAudioPath(id);
        if (path === null) return;
        await commands.exportAudio(id, path);
        notifications.show({
          message: tt('notify.export.ok', [path]),
          color: 'green',
        });
      } catch (e) {
        notifications.show({
          title: tt('errors.title'),
          message: formatError(e),
          color: 'red',
        });
      }
    },
    [tt],
  );

  // Play and "Save audio as…" share one gate: audio exists only on
  // ready/playing entries.
  const menuCanPlay =
    menuEntry !== null &&
    (menuEntry.status === 'ready' || menuEntry.status === 'playing');
  // The params dialog needs audio context: a snapshot, or at least a
  // generation timestamp (legacy entry synthesized by an older build).
  const menuHasGeneration =
    menuEntry !== null &&
    (menuEntry.generation !== null || menuEntry.audio_generated_at !== null);
  // Double-click opens the same dialog under the same gate as the menu item;
  // a dblclick always fires, so the gate lives here, not on the DOM.
  const handleOpenParams = useCallback((entry: TextEntry) => {
    if (entry.generation !== null || entry.audio_generated_at !== null) {
      setParamsEntryId(entry.id);
    }
  }, []);

  const handleDelete = useCallback(
    (id: string) => {
      modals.openConfirmModal({
        title: tt('queue.delete.title'),
        children: (
          <Text size="sm">
            {tt('queue.delete.body')}
          </Text>
        ),
        labels: { confirm: tt('common.delete'), cancel: tt('common.cancel') },
        confirmProps: { color: 'red' },
        onConfirm: async () => {
          await commands.deleteEntry(id);
          setEntries((prev) => prev.filter((e) => e.id !== id));
          if (selectedId === id) {
            setSelectedEntry(null);
          }
        },
      });
    },
    [selectedId, setSelectedEntry, tt],
  );

  return (
    <div className={classes.container}>
      {entries.length === 0 ? (
        <Text c="dimmed" size="sm" ta="center" mt="md">
          {tt('queue.empty')}
        </Text>
      ) : filteredEntries.length === 0 ? (
        <Text c="dimmed" size="sm" ta="center" mt="md">
          {tt('queue.no_results')}
        </Text>
      ) : (
        <ScrollArea className={classes.scrollArea} viewportRef={viewportRef}>
          <Stack gap={4}>
            {filteredEntries.map((entry) => (
              <QueueItem
                key={entry.id}
                entry={entry}
                isSelected={selectedId === entry.id}
                isPlaying={playingId === entry.id}
                onSelect={setSelectedEntry}
                onPlay={handlePlay}
                onContextMenu={(e, x, y) => setMenu({ id: e.id, x, y })}
                onOpenParams={handleOpenParams}
              />
            ))}
          </Stack>
        </ScrollArea>
      )}

      {playingId !== null && !playingVisible && (
        <Button
          className={classes.jumpToPlaying}
          size="compact-xs"
          radius="xl"
          variant="filled"
          color="teal"
          leftSection={<IconLocate />}
          onClick={handleJumpToPlaying}
          title={tt('queue.jump_to_playing')}
          aria-label={tt('queue.jump_to_playing')}
        >
          {tt('queue.jump_to_playing')}
        </Button>
      )}

      <Menu
        opened={menu !== null}
        onChange={(open) => { if (!open) setMenu(null); }}
        position="bottom-start"
        withinPortal
        closeOnItemClick
        // Default 'mousedown' closes the menu the instant it opens, because
        // the right-click mousedown that opened us bubbles to the window
        // after we've set `opened=true`.  `click` fires on mouseup, not
        // mousedown, so a right-click no longer self-closes.
        clickOutsideEvents={['click']}
      >
        <Menu.Target>
          <div
            style={{
              position: 'fixed',
              left: menu?.x ?? -9999,
              top: menu?.y ?? -9999,
              width: 0,
              height: 0,
              pointerEvents: 'none',
            }}
          />
        </Menu.Target>
        <Menu.Dropdown>
          {/* Play and "Save audio as…" share one gate (audio-export spec:
              "the same gate as Play"). */}
          <Menu.Item
            disabled={!menuCanPlay}
            onClick={() => menuEntry && handlePlay(menuEntry.id)}
          >
            {tt('queue.play')}
          </Menu.Item>
          <Menu.Item
            disabled={!menuCanPlay}
            onClick={() => menuEntry && void handleExportAudio(menuEntry.id)}
          >
            {tt('queue.menu.export_audio')}
          </Menu.Item>
          <Menu.Item
            disabled={menuEntry === null || menuEntry.status === 'processing'}
            onClick={() => menuEntry && onRegenerate(menuEntry)}
          >
            {tt('queue.menu.regenerate')}
          </Menu.Item>
          <Menu.Item
            disabled={!menuHasGeneration}
            onClick={() => menuEntry && setParamsEntryId(menuEntry.id)}
          >
            {tt('queue.menu.generation_params')}
          </Menu.Item>
          <Menu.Item
            disabled={
              menuEntry === null ||
              (menuEntry.status !== 'processing' && menuEntry.status !== 'pending')
            }
            onClick={() => menuEntry && handleCancelSynthesis(menuEntry.id)}
          >
            {tt('queue.menu.cancel_synthesis')}
          </Menu.Item>
          <Menu.Divider />
          <Menu.Item
            color="red"
            onClick={() => menuEntry && handleDelete(menuEntry.id)}
          >
            {tt('common.delete')}
          </Menu.Item>
        </Menu.Dropdown>
      </Menu>

      <GenerationParamsDialog
        entry={paramsEntry}
        opened={paramsEntryId !== null && paramsEntry !== null}
        onClose={() => setParamsEntryId(null)}
      />
    </div>
  );
}
