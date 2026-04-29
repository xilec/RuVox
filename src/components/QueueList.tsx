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
import type { TextEntry, EntryStatus, EntryId, UnlistenFn } from '../lib/tauri';
import { useSelectedEntry } from '../stores/selectedEntry';
import { useSearchQuery } from '../stores/searchQuery';
import { IconPlay, IconLocate } from './icons';
import classes from './QueueList.module.css';

function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}

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
  }
}

function statusLabel(status: EntryStatus): string {
  switch (status) {
    case 'pending':
      return 'Ожидание';
    case 'processing':
      return 'Обработка';
    case 'ready':
      return 'Готово';
    case 'playing':
      return 'Играет';
    case 'error':
      return 'Ошибка';
  }
}

interface QueueItemProps {
  entry: TextEntry;
  isSelected: boolean;
  isPlaying: boolean;
  onSelect: (entry: TextEntry) => void;
  onPlay: (id: string) => void;
  onContextMenu: (entry: TextEntry, x: number, y: number) => void;
}

function QueueItem({ entry, isSelected, isPlaying, onSelect, onPlay, onContextMenu }: QueueItemProps) {
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
              {statusLabel(entry.status)}
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
            title="Воспроизвести"
            onClick={(e) => {
              e.stopPropagation();
              onPlay(entry.id);
            }}
            aria-label="Воспроизвести"
          >
            <IconPlay />
          </ActionIcon>
        </Group>
      </Group>
    </div>
  );
}

export function QueueList() {
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
  const [menu, setMenu] = useState<
    { entry: TextEntry; x: number; y: number } | null
  >(null);

  const loadEntries = useCallback(async () => {
    const result = await commands.getEntries();
    // Sort by created_at desc (newest first). Backend guarantees this order
    // per the IPC contract, but local sort guards against inconsistent updates.
    result.sort((a, b) => b.created_at.localeCompare(a.created_at));
    setEntries(result);
  }, []);

  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];

    loadEntries();

    unlisteners.push(
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
    );

    unlisteners.push(
      events.entryRemoved((payload) => {
        setEntries((prev) => prev.filter((e) => e.id !== payload.id));
        useSelectedEntry.setState((state) =>
          state.selectedId === payload.id
            ? { selectedId: null, selectedEntry: null }
            : {},
        );
      }),
    );

    // Highlight the currently-playing entry.  Paused playback keeps the
    // highlight (user may resume); only stop/finish clears it.
    unlisteners.push(events.playbackStarted((p) => setPlayingId(p.entry_id)));
    unlisteners.push(events.playbackStopped(() => setPlayingId(null)));
    unlisteners.push(events.playbackFinished(() => setPlayingId(null)));

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
  }, [loadEntries]);

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
    await commands.playEntry(id);
  }, []);

  const handleRegenerate = useCallback(async (id: string) => {
    try {
      await commands.regenerateEntry(id);
      notifications.show({
        title: 'Перегенерация',
        message: 'Запущена перегенерация аудио',
        color: 'blue',
      });
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      notifications.show({
        title: 'Ошибка',
        message: `Не удалось запустить перегенерацию: ${message}`,
        color: 'red',
      });
    }
  }, []);

  const handleDelete = useCallback(
    (id: string) => {
      modals.openConfirmModal({
        title: 'Удалить запись?',
        children: (
          <Text size="sm">
            Запись и аудиофайл будут удалены без возможности восстановления.
          </Text>
        ),
        labels: { confirm: 'Удалить', cancel: 'Отмена' },
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
    [selectedId, setSelectedEntry],
  );

  return (
    <div className={classes.container}>
      {entries.length === 0 ? (
        <Text c="dimmed" size="sm" ta="center" mt="md">
          Скопируйте текст и нажмите Add
        </Text>
      ) : filteredEntries.length === 0 ? (
        <Text c="dimmed" size="sm" ta="center" mt="md">
          Ничего не найдено
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
                onContextMenu={(e, x, y) => setMenu({ entry: e, x, y })}
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
          title="К читаемому"
          aria-label="К читаемому"
        >
          К читаемому
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
          <Menu.Item
            disabled={
              menu === null ||
              (menu.entry.status !== 'ready' && menu.entry.status !== 'playing')
            }
            onClick={() => menu && handlePlay(menu.entry.id)}
          >
            Воспроизвести
          </Menu.Item>
          <Menu.Item
            disabled={menu === null || menu.entry.status === 'processing'}
            onClick={() => menu && handleRegenerate(menu.entry.id)}
          >
            Перегенерировать аудио
          </Menu.Item>
          <Menu.Divider />
          <Menu.Item
            color="red"
            onClick={() => menu && handleDelete(menu.entry.id)}
          >
            Удалить
          </Menu.Item>
        </Menu.Dropdown>
      </Menu>
    </div>
  );
}
