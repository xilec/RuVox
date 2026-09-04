import { useCallback, useEffect, useRef, useState } from 'react';
import {
  Badge,
  Button,
  Divider,
  Group,
  Modal,
  Stack,
  Table,
  Text,
  TextInput,
} from '@mantine/core';
import { modals } from '@mantine/modals';
import { notifications } from '@mantine/notifications';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import type { UnlistenFn } from '@tauri-apps/api/event';

import { commands } from '../lib/tauri';
import type { DictionaryEntryDto, DictionaryImportMode } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { useT } from '../lib/i18n';
import {
  entryKey,
  sortAndFilterEntries,
  validateEntryInput,
  type DictionaryValidationReason,
} from '../lib/userDictionary';

/** Quiet footer status line states: every mutation saves immediately, the
 *  line only ever reflects save progress/failure — no success toasts. */
type SaveStatus = 'saved' | 'saving' | 'error';

interface DictionaryModalProps {
  opened: boolean;
  onClose: () => void;
  /** Prefilled `from` for the add form (preview quick-add); consumed once
   *  per open. */
  initialFrom?: string | null;
  /** Called when the prefilled `from` has been applied to the form. */
  onInitialFromConsumed?: () => void;
}

interface EditorState {
  mode: 'add' | 'edit';
  /** Key of the entry being edited (edit mode). */
  originalKey?: string;
  from: string;
  to: string;
}

const VALIDATION_KEY: Record<DictionaryValidationReason, Parameters<ReturnType<typeof useT>>[0]> = {
  'from.required': 'dictionary.form.error.from.required',
  'from.charset': 'dictionary.form.error.from.charset',
  'from.too_long': 'dictionary.form.error.from.too_long',
  'to.required': 'dictionary.form.error.to.required',
  'to.too_long': 'dictionary.form.error.to.too_long',
  'to.latin_warning': 'dictionary.form.warning.latin',
};

/** True when the dropped file looks like a dictionary TOML (the AppShell
 *  global drop handler only claims importable text files, so a .toml lands
 *  here; anything else is ignored silently, same contract as AppShell). */
function isDictionaryFile(path: string): boolean {
  return path.toLowerCase().endsWith('.toml');
}

export function DictionaryModal({
  opened,
  onClose,
  initialFrom,
  onInitialFromConsumed,
}: DictionaryModalProps) {
  const tt = useT();
  const [entries, setEntries] = useState<DictionaryEntryDto[]>([]);
  const [search, setSearch] = useState('');
  const [editor, setEditor] = useState<EditorState | null>(null);
  const [status, setStatus] = useState<SaveStatus>('saved');
  const [pendingImport, setPendingImport] = useState<string | null>(null);
  const statusRef = useRef<SaveStatus>('saved');

  const setStatusSafe = useCallback((next: SaveStatus) => {
    statusRef.current = next;
    setStatus(next);
  }, []);

  const refreshEntries = useCallback(async () => {
    const fresh = await commands.getUserDictionary();
    setEntries(fresh);
  }, []);

  // Load on every open; reset transient editor state so a stale form from a
  // previous session never survives.
  useEffect(() => {
    if (!opened) return;
    setSearch('');
    setEditor(null);
    setPendingImport(null);
    setStatusSafe('saved');
    commands
      .getUserDictionary()
      .then((list) => {
        setEntries(list);
        if (initialFrom) {
          setEditor({ mode: 'add', from: initialFrom, to: '' });
          onInitialFromConsumed?.();
        }
      })
      .catch((e) => {
        notifications.show({
          title: tt('common.error'),
          message: formatError(e),
          color: 'red',
        });
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [opened]);

  /** Persist the full list (immediate save, no save button). The optimistic
   *  update lands before the request so consecutive actions always read the
   *  fresh list — a save-in-flight must never overwrite a newer edit. On
   *  success the entries are re-read only to refresh override badges; a
   *  failed re-read must not fake a save failure (the file is written). */
  const persist = useCallback(
    async (next: DictionaryEntryDto[]) => {
      setEntries(next);
      setStatusSafe('saving');
      try {
        await commands.saveUserDictionary(next.map(({ from, to }) => ({ from, to })));
        setStatusSafe('saved');
        commands
          .getUserDictionary()
          .then((fresh) => setEntries(fresh))
          .catch(() => {});
      } catch (e) {
        setStatusSafe('error');
        notifications.show({
          title: tt('dictionary.save.failed.title'),
          message: formatError(e),
          color: 'red',
        });
      }
    },
    [setStatusSafe, tt],
  );

  const retrySave = useCallback(() => {
    void persist(entries);
  }, [entries, persist]);

  const submitEditor = useCallback(() => {
    if (!editor) return;
    const input = { from: editor.from.trim(), to: editor.to.trim() };

    // One entry per word: submitting a from that collides with an existing
    // entry (a re-add, or a rename in edit mode) updates that entry's spoken
    // form with what the user typed instead of creating a second entry.
    const collision = entries.find(
      (e) => entryKey(e.from) === entryKey(input.from) && entryKey(e.from) !== editor.originalKey,
    );
    if (collision) {
      const next = entries.map((e) => (e === collision ? { ...e, to: input.to } : e));
      setEditor(null);
      void persist(next);
      return;
    }

    const next =
      editor.mode === 'add'
        ? [...entries, { from: input.from, to: input.to, overridesBuiltin: false }]
        : entries.map((e) =>
            entryKey(e.from) === editor.originalKey
              ? { from: input.from, to: input.to, overridesBuiltin: e.overridesBuiltin }
              : e,
          );
    setEditor(null);
    void persist(next);
  }, [editor, entries, persist]);

  const handleDelete = useCallback(
    (target: DictionaryEntryDto) => {
      modals.openConfirmModal({
        title: tt('dictionary.delete.title'),
        children: <Text size="sm">{tt('dictionary.delete.message', [target.from])}</Text>,
        labels: { confirm: tt('common.delete'), cancel: tt('common.cancel') },
        confirmProps: { color: 'red' },
        // Above the dictionary editor (400): the ModalsProvider default
        // (200) renders the confirmation behind it.
        zIndex: 500,
        onConfirm: () => {
          const next = entries.filter((e) => entryKey(e.from) !== entryKey(target.from));
          void persist(next);
        },
      });
    },
    [entries, persist, tt],
  );

  // Drop-to-import: the same webview-level event AppShell uses. A .toml drop
  // is ignored there (not an importable text extension), so the editor owns
  // it while open; the file dialog remains the parallel path.
  const dropRef = useRef<(path: string) => void>(() => {});
  useEffect(() => {
    dropRef.current = (path) => {
      if (isDictionaryFile(path)) setPendingImport(path);
    };
  });
  useEffect(() => {
    if (!opened) return;
    let disposed = false;
    let unlisten: UnlistenFn | undefined;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === 'drop' && event.payload.paths.length === 1) {
          dropRef.current(event.payload.paths[0].trim());
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        console.warn('dictionary drop subscription failed');
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [opened]);

  const applyImport = useCallback(
    async (mode: DictionaryImportMode) => {
      const path = pendingImport;
      setPendingImport(null);
      if (!path) return;
      try {
        const report = await commands.importUserDictionary(path, mode);
        await refreshEntries();
        setStatusSafe('saved');
        notifications.show({
          title: tt('dictionary.import.done.title'),
          message: tt('dictionary.import.done.message', [
            report.added,
            report.updated,
            report.skipped,
          ]),
          color: 'green',
        });
      } catch (e) {
        notifications.show({
          title: tt('dictionary.import.failed.title'),
          message: formatError(e),
          color: 'red',
        });
      }
    },
    [pendingImport, refreshEntries, setStatusSafe, tt],
  );

  const handleExport = useCallback(async () => {
    const path = await commands.pickDictionaryExportPath();
    if (!path) return;
    try {
      await commands.exportUserDictionary(path);
      notifications.show({
        title: tt('dictionary.export.done.title'),
        message: path,
        color: 'green',
      });
    } catch (e) {
      notifications.show({
        title: tt('dictionary.export.failed.title'),
        message: formatError(e),
        color: 'red',
      });
    }
  }, [tt]);

  const validation = editor ? validateEntryInput({ from: editor.from.trim(), to: editor.to.trim() }) : null;
  const hardError = validation?.blocking ? validation.reason : null;
  const softWarning = validation && !validation.blocking ? validation.reason : null;
  const visible = sortAndFilterEntries(entries, search);

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title={tt('dictionary.title')}
      size="lg"
      withinPortal
      /* Above the Settings modal (z 200) and the floating preview window
       * (--ruvox-preview-z: 300): the editor is opened from both. */
      zIndex={400}
    >
      <Stack gap="sm">
        <Group justify="space-between" wrap="nowrap">
          <TextInput
            placeholder={tt('dictionary.search.placeholder')}
            value={search}
            onChange={(e) => setSearch(e.currentTarget.value)}
            flex={1}
          />
          <Button variant="default" onClick={() => setEditor({ mode: 'add', from: '', to: '' })}>
            {tt('dictionary.add')}
          </Button>
        </Group>

        {editor !== null && (
          <Stack gap="xs" p="sm" bd="1px solid var(--mantine-color-default-border)">
            <TextInput
              label={tt('dictionary.form.from')}
              placeholder={tt('dictionary.form.from.placeholder')}
              value={editor.from}
              data-autofocus
              onChange={(e) => setEditor({ ...editor, from: e.currentTarget.value })}
              error={
                hardError !== null && hardError.startsWith('from.')
                  ? tt(VALIDATION_KEY[hardError])
                  : undefined
              }
            />
            <TextInput
              label={tt('dictionary.form.to')}
              placeholder={tt('dictionary.form.to.placeholder')}
              value={editor.to}
              onChange={(e) => setEditor({ ...editor, to: e.currentTarget.value })}
              error={
                hardError !== null && hardError.startsWith('to.')
                  ? tt(VALIDATION_KEY[hardError])
                  : undefined
              }
              description={
                softWarning !== null ? tt(VALIDATION_KEY[softWarning]) : undefined
              }
            />
            <Group justify="flex-end">
              <Button variant="default" onClick={() => setEditor(null)}>
                {tt('common.cancel')}
              </Button>
              <Button onClick={submitEditor} disabled={hardError !== null}>
                {tt('common.save')}
              </Button>
            </Group>
          </Stack>
        )}

        {entries.length === 0 ? (
          <Text c="dimmed" size="sm">
            {tt('dictionary.empty')}
          </Text>
        ) : visible.length === 0 ? (
          <Text c="dimmed" size="sm">
            {tt('dictionary.no_results')}
          </Text>
        ) : (
          <Table.ScrollContainer maxHeight={360} minWidth={320}>
            <Table verticalSpacing="xs" highlightOnHover>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>{tt('dictionary.column.from')}</Table.Th>
                  <Table.Th>{tt('dictionary.column.to')}</Table.Th>
                  <Table.Th />
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {visible.map((entry) => (
                  <Table.Tr key={entryKey(entry.from)}>
                    <Table.Td>
                      {entry.from}
                      {entry.overridesBuiltin && (
                        <Badge variant="light" size="xs" ml="xs">
                          {tt('dictionary.badge.overrides')}
                        </Badge>
                      )}
                    </Table.Td>
                    <Table.Td>{entry.to}</Table.Td>
                    <Table.Td>
                      <Group gap="xs" justify="flex-end" wrap="nowrap">
                        <Button
                          variant="subtle"
                          size="compact-xs"
                          onClick={() =>
                            setEditor({
                              mode: 'edit',
                              originalKey: entryKey(entry.from),
                              from: entry.from,
                              to: entry.to,
                            })
                          }
                        >
                          {tt('dictionary.edit')}
                        </Button>
                        <Button
                          variant="subtle"
                          color="red"
                          size="compact-xs"
                          onClick={() => handleDelete(entry)}
                        >
                          {tt('common.delete')}
                        </Button>
                      </Group>
                    </Table.Td>
                  </Table.Tr>
                ))}
              </Table.Tbody>
            </Table>
          </Table.ScrollContainer>
        )}

        <Divider />
        <Group justify="space-between" align="center" wrap="nowrap">
          <Group gap="xs">
            <Button variant="default" size="xs" onClick={() => void commands.pickDictionaryImportPath().then((p) => p && setPendingImport(p))}>
              {tt('dictionary.import')}
            </Button>
            <Button variant="default" size="xs" onClick={() => void handleExport()}>
              {tt('dictionary.export')}
            </Button>
            <Text size="xs" c="dimmed">
              {tt('dictionary.drop.hint')}
            </Text>
          </Group>
          {status === 'saving' && (
            <Text size="xs" c="dimmed">
              {tt('dictionary.status.saving')}
            </Text>
          )}
          {status === 'saved' && (
            <Text size="xs" c="dimmed">
              {tt('dictionary.status.saved')}
            </Text>
          )}
          {status === 'error' && (
            <Button size="compact-xs" variant="light" color="red" onClick={retrySave}>
              {tt('dictionary.status.error')}
            </Button>
          )}
        </Group>
      </Stack>

      <Modal
        opened={pendingImport !== null}
        onClose={() => setPendingImport(null)}
        title={tt('dictionary.import.choose.title')}
        size="md"
        withinPortal
        zIndex={500}
      >
        <Stack gap="sm">
          <Text size="sm">{tt('dictionary.import.choose.message', [pendingImport ?? ''])}</Text>
          <Stack gap={4}>
            <Button onClick={() => void applyImport('merge')}>{tt('dictionary.import.merge')}</Button>
            <Text size="xs" c="dimmed">
              {tt('dictionary.import.merge.hint')}
            </Text>
          </Stack>
          <Stack gap={4}>
            <Button color="red" variant="light" onClick={() => void applyImport('replace')}>
              {tt('dictionary.import.replace')}
            </Button>
            <Text size="xs" c="dimmed">
              {tt('dictionary.import.replace.hint')}
            </Text>
          </Stack>
        </Stack>
      </Modal>
    </Modal>
  );
}
