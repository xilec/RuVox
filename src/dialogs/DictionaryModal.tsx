import { useCallback, useEffect, useRef, useState } from 'react';
import {
  ActionIcon,
  Badge,
  Box,
  Button,
  Divider,
  Group,
  Modal,
  Popover,
  Select,
  Stack,
  Table,
  Text,
  TextInput,
  Tooltip,
} from '@mantine/core';
import { modals } from '@mantine/modals';
import { notifications } from '@mantine/notifications';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import type { UnlistenFn } from '@tauri-apps/api/event';

import { IconPlay, IconStop } from '../components/icons';
import { commands } from '../lib/tauri';
import type {
  DictionaryCollectionDto,
  DictionaryEntryDto,
  DictionaryImportMode,
  DictionaryScope,
  EngineKind,
  PreviewPronunciationResult,
} from '../lib/tauri';
import { computeEngineFormState, voiceForEngine } from '../lib/engineSelection';
import { formatError } from '../lib/errors';
import { useT } from '../lib/i18n';
import {
  entryKey,
  findFormCollision,
  isSingleSourceToken,
  isSpokenFormSynthesizable,
  parseFormList,
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
  /** Called after every successful dictionary write (add, edit, delete or
   *  import), so the caller can offer a regeneration for stale audio. */
  onSaved?: () => void;
}

interface EditorState {
  mode: 'add' | 'edit';
  /** Key of the entry being edited (edit mode). */
  originalKey?: string;
  from: string;
  to: string;
  /** Comma-separated additional exact forms. */
  forms: string;
}

const VALIDATION_KEY: Record<DictionaryValidationReason, Parameters<ReturnType<typeof useT>>[0]> = {
  'from.required': 'dictionary.form.error.from.required',
  'from.charset': 'dictionary.form.error.from.charset',
  'from.too_long': 'dictionary.form.error.from.too_long',
  'to.required': 'dictionary.form.error.to.required',
  'to.stress_malformed': 'dictionary.form.error.to.stress_malformed',
  'to.too_long': 'dictionary.form.error.to.too_long',
  'to.latin_warning': 'dictionary.form.warning.latin',
  'forms.requires_cyrillic': 'dictionary.form.error.forms.requires_cyrillic',
  'forms.charset': 'dictionary.form.error.forms.charset',
  'forms.too_long': 'dictionary.form.error.forms.too_long',
  'forms.duplicate': 'dictionary.form.error.forms.duplicate',
  'forms.collision': 'dictionary.form.error.forms.collision',
};

/** The dialog sits above Settings (z 200) and the floating preview window
 *  (--ruvox-preview-z: 300); its tooltips have to clear the dialog itself,
 *  because the Mantine default (popover, z 300) would land behind it. */
const DIALOG_Z = 400;
const NESTED_DIALOG_Z = 500;
const TOOLTIP_Z = 600;

/** True when the dropped file looks like a dictionary TOML (the AppShell
 *  global drop handler only claims importable text files, so a .toml lands
 *  here; anything else is ignored silently, same contract as AppShell). */
function isDictionaryFile(path: string): boolean {
  return path.toLowerCase().endsWith('.toml');
}

/** Which field's preview is in flight or playing. */
type PreviewField = 'from' | 'to';

/** Scope → catalog key of its display name (scope selector and engine hint). */
const SCOPE_LABEL_KEY: Record<DictionaryScope, Parameters<ReturnType<typeof useT>>[0]> = {
  global: 'dictionary.scope.global',
  silero_native: 'dictionary.scope.silero_native',
  silero: 'dictionary.scope.silero',
  piper: 'dictionary.scope.piper',
  teratts: 'dictionary.scope.teratts',
};

/** Preview cache size limit: every entry holds a whole WAV, and the editor
 *  only replays a handful of words before a save retires the cache. */
const PREVIEW_CACHE_MAX = 12;

/** Extra time after the reported clip length before the dialog stops the
 *  preview player: covers mpv startup and the audio device's latency. */
const PREVIEW_END_MARGIN_MS = 500;

/** Read a cached preview and mark it most recently used. `Map` iterates in
 *  insertion order, so re-inserting moves the entry to the end. */
function takeCachedPreview(
  cache: Map<string, PreviewPronunciationResult>,
  key: string,
): PreviewPronunciationResult | undefined {
  const cached = cache.get(key);
  if (cached === undefined) return undefined;
  cache.delete(key);
  cache.set(key, cached);
  return cached;
}

/** Insert a fresh preview past the LRU limit: the least recently used
 *  readings are dropped first (issue #349 review). */
function cachePreview(
  cache: Map<string, PreviewPronunciationResult>,
  key: string,
  result: PreviewPronunciationResult,
) {
  cache.delete(key);
  cache.set(key, result);
  while (cache.size > PREVIEW_CACHE_MAX) {
    const oldest = cache.keys().next().value;
    if (oldest === undefined) break;
    cache.delete(oldest);
  }
}

export function DictionaryModal({
  opened,
  onClose,
  initialFrom,
  onInitialFromConsumed,
  onSaved,
}: DictionaryModalProps) {
  const tt = useT();
  const [collections, setCollections] = useState<DictionaryCollectionDto[]>([]);
  const [scope, setScope] = useState<DictionaryScope>('global');
  const [entries, setEntries] = useState<DictionaryEntryDto[]>([]);
  const [search, setSearch] = useState('');
  const [editor, setEditor] = useState<EditorState | null>(null);
  const [status, setStatus] = useState<SaveStatus>('saved');
  const [pendingImport, setPendingImport] = useState<string | null>(null);
  const statusRef = useRef<SaveStatus>('saved');

  // ── Pronunciation preview (issue #349) ─────────────────────────────────
  const [previewBusy, setPreviewBusy] = useState<PreviewField | null>(null);
  const [previewPlaying, setPreviewPlaying] = useState<PreviewField | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewEngine, setPreviewEngine] = useState<EngineKind | null>(null);
  const [previewVoice, setPreviewVoice] = useState('');
  const previewTimerRef = useRef<number | null>(null);
  const previewCacheRef = useRef(new Map<string, PreviewPronunciationResult>());
  const previewSeqRef = useRef(0);
  /** Bumped on every dictionary write: a cached «Слово» reading must not
   *  outlive the save that changed it. */
  const previewRevisionRef = useRef(0);

  const setStatusSafe = useCallback((next: SaveStatus) => {
    statusRef.current = next;
    setStatus(next);
  }, []);

  /** Drop the pending playback-end timer without touching the player. */
  const clearPreviewTimer = useCallback(() => {
    if (previewTimerRef.current !== null) {
      window.clearTimeout(previewTimerRef.current);
      previewTimerRef.current = null;
    }
  }, []);

  /** Abort the current preview (playing or in flight) and clear its state. */
  const stopPreview = useCallback(() => {
    clearPreviewTimer();
    // Invalidate an in-flight response: it must not start playing after the
    // editor moved on.
    previewSeqRef.current += 1;
    // The backend player is idempotent; a stop failure (already gone, app
    // exiting) must not break the form.
    void commands.previewStop().catch((e) => {
      console.warn('dictionary preview: stop failed', e);
    });
    setPreviewBusy(null);
    setPreviewPlaying(null);
  }, [clearPreviewTimer]);

  /** Pause narration, never stop it: the preview speaks over the pause and
   *  the user resumes where they left off. A failed pause is reported without
   *  failing the preview itself — the audio still plays. */
  const pauseNarration = useCallback(async () => {
    try {
      await commands.pausePlayback();
    } catch (e) {
      notifications.show({
        title: tt('dictionary.preview.pause_failed.title'),
        message: formatError(e),
        color: 'yellow',
      });
    }
  }, [tt]);

  /** Retire every cached reading: after a dictionary write (save or import)
   *  the saved dictionary no longer matches what the cache holds. */
  const bumpPreviewRevision = useCallback(() => {
    previewRevisionRef.current += 1;
    previewCacheRef.current.clear();
  }, []);

  /** Play (or stop) one field's pronunciation preview. */
  const startPreview = useCallback(
    (field: PreviewField) => {
      if (!editor) return;
      if (previewPlaying === field) {
        stopPreview();
        return;
      }
      const value = (field === 'from' ? editor.from : editor.to).trim();
      const playable =
        field === 'from' ? isSingleSourceToken(value) : isSpokenFormSynthesizable(value);
      if (!playable) return;

      const mode = field === 'from' ? 'normalize' : 'spoken';
      // Cache identity: the request inputs plus the dictionary revision. The
      // response's `spoken_text` is only known after the round trip, so it
      // cannot be the lookup key; a save bumps the revision and retires the
      // cached reading.
      const key = `${previewEngine ?? 'unknown'}|${previewVoice}|${mode}|${value}|${
        previewRevisionRef.current
      }`;

      void (async () => {
        stopPreview();
        setPreviewError(null);
        const seq = previewSeqRef.current;
        // Pause narration as the preview starts, not when synthesis returns:
        // the click is the user asking to listen now, and a cache miss can
        // take seconds. Narration is never resumed afterwards.
        await pauseNarration();
        if (seq !== previewSeqRef.current) return;

        let result = takeCachedPreview(previewCacheRef.current, key);
        if (!result) {
          setPreviewBusy(field);
          try {
            result = await commands.previewPronunciation(value, mode);
          } catch (e) {
            if (seq !== previewSeqRef.current) return;
            setPreviewBusy(null);
            // No resume on failure: the paused narration is the user's own
            // state, not a placeholder for the preview.
            setPreviewError(tt('dictionary.preview.error', [formatError(e)]));
            return;
          }
          if (seq !== previewSeqRef.current) return;
          cachePreview(previewCacheRef.current, key, result);
        }

        // Second pause, right before playback starts: a cache hit skips the
        // synthesis wait above, and a narration started in between must still
        // yield to the preview.
        await pauseNarration();
        if (seq !== previewSeqRef.current) return;

        // The backend owns the mpv instance and the temporary WAV; a failure
        // is the same inline line the webview player used to produce.
        setPreviewBusy(field);
        try {
          await commands.previewPlay(result.wav_base64);
        } catch (e) {
          if (seq !== previewSeqRef.current) return;
          stopPreview();
          console.warn('dictionary preview: play failed', e);
          setPreviewError(tt('dictionary.preview.error.play_failed'));
          return;
        }
        if (seq !== previewSeqRef.current) {
          // The editor closed (or a newer preview won) while play was in
          // flight: stop the clip the backend may have just started.
          void commands.previewStop().catch(() => {});
          return;
        }

        setPreviewBusy(null);
        setPreviewPlaying(field);
        // mpv stays alive after EOF (`--idle=yes`); stop it once the reported
        // clip length elapses, with a margin for startup and the audio device.
        // A second click, another preview, or the editor closing clears this
        // timer first.
        clearPreviewTimer();
        previewTimerRef.current = window.setTimeout(
          () => {
            if (seq !== previewSeqRef.current) return;
            stopPreview();
          },
          result.duration_sec * 1000 + PREVIEW_END_MARGIN_MS,
        );
      })();
    },
    [
      editor,
      previewPlaying,
      stopPreview,
      pauseNarration,
      previewEngine,
      previewVoice,
      clearPreviewTimer,
      tt,
    ],
  );

  /** One play/stop control, rendered in a field's `rightSection`. */
  const renderPreviewControl = (field: PreviewField) => {
    const value = (field === 'from' ? editor?.from : editor?.to)?.trim() ?? '';
    const playable =
      field === 'from' ? isSingleSourceToken(value) : isSpokenFormSynthesizable(value);
    const busy = previewBusy === field;
    const playing = previewPlaying === field;
    // Both controls are blocked while either preview synthesizes.
    const disabled = !playable || previewBusy !== null;
    const label = playing
      ? tt('dictionary.preview.stop')
      : busy
        ? tt('dictionary.preview.busy')
        : tt(field === 'from' ? 'dictionary.preview.from.aria' : 'dictionary.preview.to.aria');
    const hint = tt(
      field === 'from' ? 'dictionary.preview.from.hint' : 'dictionary.preview.to.hint',
    );
    return (
      <Tooltip
        label={playing ? tt('dictionary.preview.stop') : hint}
        multiline
        w={260}
        withArrow
        zIndex={TOOLTIP_Z}
      >
        {/* The tooltip has to open while the field is empty, when the
            control is disabled: a disabled Mantine button swallows pointer
            events, so the span wrapper carries them. */}
        <Box component="span" display="flex">
          <ActionIcon
            variant="subtle"
            size="sm"
            radius="xl"
            aria-label={label}
            disabled={disabled}
            loading={busy}
            data-preview={field}
            onClick={() => startPreview(field)}
          >
            {playing ? <IconStop /> : <IconPlay />}
          </ActionIcon>
        </Box>
      </Tooltip>
    );
  };

  const applyCollections = useCallback(
    (fresh: DictionaryCollectionDto[], target: DictionaryScope) => {
      setCollections(fresh);
      const selected = fresh.find((c) => c.scope === target);
      setEntries(selected?.entries ?? []);
    },
    [],
  );

  const refreshCollections = useCallback(
    async (target: DictionaryScope) => {
      const fresh = await commands.getDictionaries();
      applyCollections(fresh, target);
    },
    [applyCollections],
  );

  // Load on every open; reset transient editor state so a stale form from a
  // previous session never survives.
  useEffect(() => {
    if (!opened) return;
    setSearch('');
    setEditor(null);
    setPendingImport(null);
    setStatusSafe('saved');
    stopPreview();
    setPreviewError(null);
    setPreviewEngine(null);
    setPreviewVoice('');
    previewCacheRef.current.clear();
    setScope('global');
    commands
      .getDictionaries()
      .then((list) => {
        applyCollections(list, 'global');
        if (initialFrom) {
          setEditor({ mode: 'add', from: initialFrom, to: '', forms: '' });
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

  // The preview speaks with the engine actually serving this session — the
  // same fallback reducer Settings uses, so a startup fallback (e.g. a missing
  // Silero bundle serving Piper) names the engine that will really speak.
  useEffect(() => {
    if (!opened) return;
    let cancelled = false;
    Promise.all([commands.getConfig(), commands.getAvailableEngines()])
      .then(([config, availability]) => {
        if (cancelled) return;
        const engine = computeEngineFormState(config, availability).engine;
        setPreviewEngine(engine);
        setPreviewVoice(voiceForEngine(engine, config));
      })
      .catch((e) => console.warn('dictionary preview: engine probe failed', e));
    return () => {
      cancelled = true;
    };
  }, [opened]);

  // Closing the editor (or the dialog) stops the preview player; a preview
  // never outlives the form it belongs to.
  const editorOpen = editor !== null;
  useEffect(() => {
    if (!opened || !editorOpen) stopPreview();
  }, [opened, editorOpen, stopPreview]);

  // Unmount: never leave the preview mpv process or its timer running.
  useEffect(
    () => () => {
      // Drop any in-flight response too: nothing may start playing after the
      // dialog is gone.
      previewSeqRef.current += 1;
      if (previewTimerRef.current !== null) window.clearTimeout(previewTimerRef.current);
      void commands.previewStop().catch(() => {});
    },
    [],
  );
  /** Persist the full list (immediate save, no save button). The optimistic
   *  update lands before the request so consecutive actions always read the
   *  fresh list — a save-in-flight must never overwrite a newer edit. On
   *  success the entries are re-read only to refresh override badges; a
   *  failed re-read must not fake a save failure (the file is written). */
  const persist = useCallback(
    async (next: DictionaryEntryDto[]) => {
      setEntries(next);
      setCollections((prev) =>
        prev.some((c) => c.scope === scope)
          ? prev.map((c) => (c.scope === scope ? { ...c, entries: next } : c))
          : [...prev, { scope, name: scope, entries: next }],
      );
      setStatusSafe('saving');
      try {
        await commands.saveDictionary(
          scope,
          next.map(({ from, to, forms }) => ({ from, to, forms })),
        );
        bumpPreviewRevision();
        setStatusSafe('saved');
        onSaved?.();
        refreshCollections(scope).catch(() => {});
      } catch (e) {
        setStatusSafe('error');
        notifications.show({
          title: tt('dictionary.save.failed.title'),
          message: formatError(e),
          color: 'red',
        });
      }
    },
    [bumpPreviewRevision, onSaved, refreshCollections, scope, setStatusSafe, tt],
  );

  const retrySave = useCallback(() => {
    void persist(entries);
  }, [entries, persist]);

  const submitEditor = useCallback(() => {
    if (!editor) return;
    const input = {
      from: editor.from.trim(),
      to: editor.to.trim(),
      forms: parseFormList(editor.forms),
    };

    // One entry per word: submitting a from that collides with an existing
    // entry (a re-add, or a rename in edit mode) updates that entry's spoken
    // form with what the user typed instead of creating a second entry.
    const collision = entries.find(
      (e) => entryKey(e.from) === entryKey(input.from) && entryKey(e.from) !== editor.originalKey,
    );
    if (collision) {
      const next = entries.map((e) =>
        e === collision ? { ...e, to: input.to, forms: input.forms } : e,
      );
      setEditor(null);
      void persist(next);
      return;
    }

    const next =
      editor.mode === 'add'
        ? [...entries, { ...input, overridesBuiltin: false }]
        : entries.map((e) =>
            entryKey(e.from) === editor.originalKey
              ? { ...input, overridesBuiltin: e.overridesBuiltin }
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
        const report = await commands.importDictionary(path, mode, scope);
        await refreshCollections(scope);
        bumpPreviewRevision();
        setStatusSafe('saved');
        onSaved?.();
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
    [bumpPreviewRevision, onSaved, pendingImport, refreshCollections, scope, setStatusSafe, tt],
  );

  const handleExport = useCallback(async () => {
    const path = await commands.pickDictionaryExportPath();
    if (!path) return;
    try {
      await commands.exportDictionary(path, scope);
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
  }, [scope, tt]);

  const changeScope = useCallback(
    (next: DictionaryScope | null) => {
      if (!next) return;
      setScope(next);
      setEntries(collections.find((c) => c.scope === next)?.entries ?? []);
      setEditor(null);
      setSearch('');
      setPendingImport(null);
    },
    [collections],
  );

  const editorInput = editor
    ? {
        from: editor.from.trim(),
        to: editor.to.trim(),
        forms: parseFormList(editor.forms),
      }
    : null;
  const validation = editorInput ? validateEntryInput(editorInput) : null;
  const formCollision =
    editorInput && editor
      ? findFormCollision(
          entries,
          editorInput.from,
          editorInput.forms,
          editor.originalKey,
        )
      : null;
  const hardError = validation?.blocking
    ? validation.reason
    : formCollision !== null
      ? 'forms.collision'
      : null;
  const softWarning = validation && !validation.blocking ? validation.reason : null;
  const previewHint =
    previewEngine !== null && scope !== 'global' && scope !== previewEngine
      ? tt('dictionary.preview.engine_hint', [
          tt(SCOPE_LABEL_KEY[previewEngine]),
          tt(SCOPE_LABEL_KEY[scope]),
        ])
      : null;
  const visible = sortAndFilterEntries(entries, search);

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title={tt('dictionary.title')}
      size="lg"
      withinPortal
      zIndex={DIALOG_Z}
    >
      <Stack gap="sm">
        <Select
          label={tt('dictionary.scope.label')}
          description={tt('dictionary.scope.description')}
          value={scope}
          onChange={(value) => changeScope(value as DictionaryScope | null)}
          data={(Object.keys(SCOPE_LABEL_KEY) as DictionaryScope[]).map((value) => ({
            value,
            label: tt(SCOPE_LABEL_KEY[value]),
          }))}
          allowDeselect={false}
          // The modal sits at DIALOG_Z; Mantine's default popover z-index
          // (300) would put the dropdown behind the overlay, making the
          // select look unresponsive.
          comboboxProps={{ zIndex: NESTED_DIALOG_Z }}
        />
        <Group justify="space-between" wrap="nowrap">
          <TextInput
            placeholder={tt('dictionary.search.placeholder')}
            value={search}
            onChange={(e) => setSearch(e.currentTarget.value)}
            flex={1}
          />
          <Button
            variant="default"
            onClick={() => setEditor({ mode: 'add', from: '', to: '', forms: '' })}
          >
            {tt('dictionary.add')}
          </Button>
        </Group>

        {editor !== null && (
          <Stack gap="xs" p="sm" bd="1px solid var(--mantine-color-default-border)">
            <TextInput
              label={tt('dictionary.form.from')}
              rightSectionWidth={76}
              rightSectionPointerEvents="all"
              rightSection={
                <Group gap={2} wrap="nowrap">
                  {renderPreviewControl('from')}
                  <Tooltip
                  label={tt('dictionary.form.from.hint')}
                  multiline
                  w={260}
                  withArrow
                  zIndex={TOOLTIP_Z}
                >
                  <ActionIcon
                    variant="subtle"
                    size="sm"
                    radius="xl"
                    aria-label={tt('dictionary.form.from.help')}
                  >
                    ?
                  </ActionIcon>
                  </Tooltip>
                </Group>
              }
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
              rightSectionWidth={76}
              rightSectionPointerEvents="all"
              rightSection={
                <Group gap={2} wrap="nowrap">
                  {renderPreviewControl('to')}
                  <Tooltip
                  label={tt('dictionary.form.stress.hint')}
                  multiline
                  w={260}
                  withArrow
                  zIndex={TOOLTIP_Z}
                >
                  <ActionIcon
                    variant="subtle"
                    size="sm"
                    radius="xl"
                    aria-label={tt('dictionary.form.stress.help')}
                  >
                    ?
                  </ActionIcon>
                  </Tooltip>
                </Group>
              }
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
            {/* Directly under the field they belong to: the hint explains what
                the preview will say, the error replaces it on failure. */}
            {previewHint !== null && (
              <Text size="xs" c="dimmed" data-preview-hint>
                {previewHint}
              </Text>
            )}
            {previewError !== null && (
              <Text size="xs" c="red" data-preview-error>
                {previewError}
              </Text>
            )}
            <TextInput
              label={tt('dictionary.form.forms')}
              rightSection={
                <Tooltip
                  label={tt('dictionary.form.forms.hint')}
                  multiline
                  w={320}
                  withArrow
                  zIndex={TOOLTIP_Z}
                >
                  <ActionIcon
                    variant="subtle"
                    size="sm"
                    radius="xl"
                    aria-label={tt('dictionary.form.forms.help')}
                  >
                    ?
                  </ActionIcon>
                </Tooltip>
              }
              placeholder={tt('dictionary.form.forms.placeholder')}
              value={editor.forms}
              onChange={(e) => setEditor({ ...editor, forms: e.currentTarget.value })}
              description={tt('dictionary.form.forms.description')}
              error={
                hardError !== null && hardError.startsWith('forms.')
                  ? tt(VALIDATION_KEY[hardError])
                  : undefined
              }
            />
            <Popover
              width={380}
              position="bottom-start"
              withArrow
              shadow="md"
              withinPortal
              zIndex={TOOLTIP_Z}
            >
              <Popover.Target>
                <Button variant="subtle" size="compact-xs" w="fit-content">
                  {tt('dictionary.form.forms.details.open')}
                </Button>
              </Popover.Target>
              <Popover.Dropdown>
                <Stack gap={6}>
                  <Text size="sm" fw={600}>
                    {tt('dictionary.form.forms.details.title')}
                  </Text>
                  <Text size="xs">{tt('dictionary.form.forms.details.intro')}</Text>
                  <Text size="xs">{tt('dictionary.form.forms.details.example')}</Text>
                  <Text size="xs">{tt('dictionary.form.forms.details.rules')}</Text>
                  <Text size="xs">{tt('dictionary.form.forms.details.stress')}</Text>
                </Stack>
              </Popover.Dropdown>
            </Popover>
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
                      {(entry.forms ?? []).length > 0 && (
                        <Text size="xs" c="dimmed">
                          {(entry.forms ?? []).join(', ')}
                        </Text>
                      )}
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
                              forms: (entry.forms ?? []).join(', '),
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
        zIndex={NESTED_DIALOG_Z}
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
