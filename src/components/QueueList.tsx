import { useEffect, useState, useCallback } from 'react';
import {
  Stack,
  Group,
  Text,
  Badge,
  ActionIcon,
  ScrollArea,
  Loader,
  Menu,
} from '@mantine/core';
import { modals } from '@mantine/modals';
import { commands, events } from '../lib/tauri';
import type { TextEntry, EntryStatus, UnlistenFn } from '../lib/tauri';
import { useSelectedEntry } from '../stores/selectedEntry';
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
  onSelect: (entry: TextEntry) => void;
  onPlay: (id: string) => void;
  onDelete: (id: string) => void;
}

function QueueItem({ entry, isSelected, onSelect, onPlay, onDelete }: QueueItemProps) {
  const preview = entry.original_text.slice(0, 60);
  const isProcessing = entry.status === 'processing';
  const canPlay = entry.status === 'ready' || entry.status === 'playing';

  // Mantine Menu opens on trigger click; we open it on right-click (contextmenu)
  // and position it at the cursor.  Controlled state so the trigger is a
  // dummy anchor sized 0.
  const [menuOpen, setMenuOpen] = useState(false);
  const [menuPos, setMenuPos] = useState<{ x: number; y: number }>({ x: 0, y: 0 });

  return (
    <>
      <div
        className={`${classes.item} ${isSelected ? classes.itemSelected : ''}`}
        onClick={() => onSelect(entry)}
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          setMenuPos({ x: e.clientX, y: e.clientY });
          setMenuOpen(true);
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
              &#x25B6;
            </ActionIcon>
          </Group>
        </Group>
      </div>

      <Menu
        opened={menuOpen}
        onChange={setMenuOpen}
        position="bottom-start"
        withinPortal
      >
        <Menu.Target>
          <div
            style={{
              position: 'fixed',
              left: menuPos.x,
              top: menuPos.y,
              width: 0,
              height: 0,
              pointerEvents: 'none',
            }}
          />
        </Menu.Target>
        <Menu.Dropdown>
          <Menu.Item
            disabled={!canPlay}
            onClick={() => onPlay(entry.id)}
          >
            Воспроизвести
          </Menu.Item>
          <Menu.Divider />
          <Menu.Item
            color="red"
            onClick={() => onDelete(entry.id)}
          >
            Удалить
          </Menu.Item>
        </Menu.Dropdown>
      </Menu>
    </>
  );
}

export function QueueList() {
  const [entries, setEntries] = useState<TextEntry[]>([]);
  const { selectedId, setSelectedEntry } = useSelectedEntry();

  const loadEntries = useCallback(async () => {
    const result = await commands.getEntries();
    // Sort by created_at desc (newest first). Backend guarantees this order
    // per the IPC contract, but local sort guards against inconsistent updates.
    result.sort((a, b) => b.created_at.localeCompare(a.created_at));
    setEntries(result);
  }, []);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    loadEntries();

    events
      .entryUpdated((payload) => {
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
      })
      .then((fn) => {
        unlisten = fn;
      });

    return () => {
      unlisten?.();
    };
  }, [loadEntries]);

  const handlePlay = useCallback(async (id: string) => {
    await commands.playEntry(id);
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

  if (entries.length === 0) {
    return (
      <Text c="dimmed" size="sm" ta="center" mt="md">
        Скопируйте текст и нажмите Read Now
      </Text>
    );
  }

  return (
    <ScrollArea className={classes.scrollArea}>
      <Stack gap="xs">
        {entries.map((entry) => (
          <QueueItem
            key={entry.id}
            entry={entry}
            isSelected={selectedId === entry.id}
            onSelect={setSelectedEntry}
            onPlay={handlePlay}
            onDelete={handleDelete}
          />
        ))}
      </Stack>
    </ScrollArea>
  );
}
