import {
  AppShell as MantineAppShell,
  Title,
  Group,
  Button,
  ActionIcon,
  TextInput,
  CloseButton,
  useMantineColorScheme,
} from '@mantine/core';
import { Menu } from '@mantine/core';
import { useHotkeys } from '@mantine/hooks';
import { notifications } from '@mantine/notifications';
import { useState, useEffect, useRef } from 'react';
import { readText as readClipboardText } from '@tauri-apps/plugin-clipboard-manager';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { commands } from '../lib/tauri';
import type { EntryFormat, EntrySource, UIConfig, UnlistenFn } from '../lib/tauri';
import type { ReadTextFileResult, TextEntry } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { useT } from '../lib/i18n';
import { setLocale, toLocale } from '../stores/locale';
import { resolveIngest } from '../lib/ingest';
import { resolveAddAction } from '../lib/addFlow';
import { resolveImport } from '../lib/importFlow';
import { IMPORTABLE_FILE_EXTENSIONS } from '../lib/importFlow';
import type { ImportSource } from '../lib/importFlow';
import { shouldOfferBundleDownload } from '../lib/bundlePrompt';
import { TextViewer } from './TextViewer';
import { Player } from './Player';
import { QueueList } from './QueueList';
import { useSelectedEntry } from '../stores/selectedEntry';
import { useSearchQuery } from '../stores/searchQuery';
import { PreviewDialog } from '../dialogs/PreviewDialog';
import { SettingsModal } from '../dialogs/Settings';
import { SileroBundlePrompt } from '../dialogs/SileroBundlePrompt';
import { EncodingDialog } from '../dialogs/EncodingDialog';
import { UrlImportDialog } from '../dialogs/UrlImportDialog';
import { IconSearch, IconChevronDown } from './icons';
import classes from './AppShell.module.css';

/** File whose lowercased extension is importable — the silent pre-filter for
 *  drops (unsupported extensions are ignored without an error per spec). */
function hasImportableExtension(fileName: string): boolean {
  const lower = fileName.toLowerCase();
  return IMPORTABLE_FILE_EXTENSIONS.some((ext) => lower.endsWith(`.${ext}`));
}

interface EncodingDialogState {
  path: string;
  detected: ReadTextFileResult;
}

export function AppShell() {
  const tt = useT();
  const { selectedEntry } = useSelectedEntry();
  const { setColorScheme } = useMantineColorScheme();
  const [pending, setPending] = useState(false);
  const [settingsOpened, setSettingsOpened] = useState(false);
  const [previewOpen, setPreviewOpen] = useState(false);
  const [previewText, setPreviewText] = useState('');
  // Format preselected in the dialog for imported sources (text-import spec,
  // "Import format routing": extension decides for files). Clipboard openings
  // leave it null so the dialog starts in the auto-detect mode.
  const [previewFormat, setPreviewFormat] = useState<EntryFormat | null>(null);
  // Plain flavor carried alongside an auto-detected HTML opening of the
  // preview dialog: when the markup yields no readable text, synthesis falls
  // back to it (same rule as the ungated direct path). Null when the dialog
  // was opened with plain text — an explicit `html` selector choice then
  // keeps the red error on failed extraction (preview-dialog spec).
  const [previewPlainFallback, setPreviewPlainFallback] = useState<string | null>(null);
  // Where the previewed text came from (clipboard or an import): recorded on
  // the entry when synthesis is confirmed from the dialog.
  const [previewSource, setPreviewSource] = useState<EntrySource>('clipboard');
  // Regeneration preview (preview-dialog spec): the queue context menu hands
  // the entry here instead of synthesizing right away. The dialog shows the
  // stored original_text; only a confirmed «Перегенерировать» reaches the
  // backend, so cancelling never touches the existing audio.
  const [regenEntry, setRegenEntry] = useState<TextEntry | null>(null);
  const [config, setConfig] = useState<UIConfig | null>(null);
  const configLoaded = useRef(false);
  const [bundlePromptOpen, setBundlePromptOpen] = useState(false);
  const [navWidth, setNavWidth] = useState(280);
  const navResizeRef = useRef<{
    pointerId: number;
    startX: number;
    originW: number;
  } | null>(null);
  const { query, setQuery } = useSearchQuery();
  const searchInputRef = useRef<HTMLInputElement>(null);
  // Import entry points (text-import spec): URL modal, manual-encoding step,
  // and the full-window drag-over overlay driven by enter/leave transitions.
  const [urlDialogOpen, setUrlDialogOpen] = useState(false);
  const [encodingDialog, setEncodingDialog] = useState<EncodingDialogState | null>(null);
  const [dropDepth, setDropDepth] = useState(0);

  // Ctrl+F / Cmd+F focuses the queue search field. preventDefault stops the
  // webview's built-in "find in page" behaviour so the hotkey is consistent
  // across platforms.
  useHotkeys([
    [
      'mod+F',
      () => searchInputRef.current?.focus(),
      { preventDefault: true },
    ],
  ]);

  function onNavResizeDown(e: React.PointerEvent<HTMLDivElement>) {
    e.preventDefault();
    navResizeRef.current = {
      pointerId: e.pointerId,
      startX: e.clientX,
      originW: navWidth,
    };
    e.currentTarget.setPointerCapture(e.pointerId);
  }
  function onNavResizeMove(e: React.PointerEvent<HTMLDivElement>) {
    const s = navResizeRef.current;
    if (!s || s.pointerId !== e.pointerId) return;
    const next = Math.min(
      Math.floor(window.innerWidth * 0.7),
      Math.max(180, s.originW + (e.clientX - s.startX)),
    );
    setNavWidth(next);
  }
  function onNavResizeUp(e: React.PointerEvent<HTMLDivElement>) {
    const s = navResizeRef.current;
    if (!s || s.pointerId !== e.pointerId) return;
    navResizeRef.current = null;
    e.currentTarget.releasePointerCapture(e.pointerId);
  }

  useEffect(() => {
    if (configLoaded.current) return;
    configLoaded.current = true;
    commands.getConfig().then((cfg) => {
      setConfig(cfg);
      // Mantine's color-scheme manager is the source of truth for the UI;
      // sync it to the persisted backend theme on first load so the saved
      // choice survives across launches.
      setColorScheme(cfg.theme);
      // Same pattern for the UI language: seed the localization store from
      // the persisted config so every catalog-driven string starts in the
      // saved locale.
      setLocale(toLocale(cfg.language));
      // First-run bundle prompt (ui spec): when the persisted engine is
      // silero_native but the bundle probe reports it missing, offer the
      // one-time download. A failed probe is non-fatal — no prompt.
      commands
        .getAvailableEngines()
        .then((availability) => {
          if (shouldOfferBundleDownload(cfg, availability)) {
            setBundlePromptOpen(true);
          }
        })
        .catch(() => {});
    }).catch(() => {
      // Config load failure is non-fatal; preview will be skipped
    });
  }, [setColorScheme]);

  // Paste anywhere in the window ingests clipboard content. The paste event
  // carries the text/html flavor natively in the WKWebView (no permission
  // prompts, unlike navigator.clipboard.read), so this is the primary
  // HTML-detection path (html-ingestion spec).
  useEffect(() => {
    function handlePaste(e: ClipboardEvent) {
      // Don't hijack paste into text inputs (search field, preview editor).
      if (!(e.target instanceof Element)) return;
      if (e.target.closest('input, textarea, [contenteditable="true"]')) return;
      if (pending) return;

      const rawHtml = e.clipboardData?.getData('text/html') ?? '';
      const plain = e.clipboardData?.getData('text/plain') ?? '';
      if (!rawHtml.trim() && !plain.trim()) return;
      e.preventDefault();
      setPending(true);
      // Paste shares the Add-flow decision with the preview gate always off
      // (the dialog gates the Add button only — preview-dialog spec) and
      // stays silent on an empty extraction: a passive paste event must not
      // nag with the empty-clipboard hint, unlike the explicit Add click.
      const action = resolveAddAction({
        html: rawHtml,
        plain,
        previewEnabled: false,
      });
      switch (action.kind) {
        case 'empty':
          setPending(false);
          return;
        case 'direct-html':
          void runDirectHtml(action.html, action.plainFallback, false).catch((err) => {
            notifications.show({ title: tt('errors.title'), message: formatError(err), color: 'red' });
            setPending(false);
          });
          return;
        case 'direct-plain':
          void doAddEntry(action.text, true);
          return;
        case 'preview':
          // Unreachable: previewEnabled is hard-coded false above.
          setPending(false);
          return;
      }
    }
    window.addEventListener('paste', handlePaste);
    return () => window.removeEventListener('paste', handlePaste);
    // Re-subscribing on `pending` is enough: addHtmlEntry/doAddEntry are
    // state-independent (they only touch commands, notifications, stores);
    // `tt` re-subscribes on locale switch so toasts use the active catalog.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pending, tt]);

  // Neutral hint for an explicit Add click that found nothing ingestible
  // (#194: an empty clipboard must not look like an app failure). Shared by
  // the `empty` and the no-plain-fallback `direct-html` arms.
  function showEmptyClipboardHint() {
    notifications.show({
      title: tt('app.clipboard.empty.title'),
      message: tt('app.clipboard.empty.message'),
      color: 'blue',
    });
  }

  // ── Import flow glue (text-import spec, #224) ─────────────────────────────

  // Shared red-error notification for import/fetch failures (coded errors
  // localize through formatError's catalog lookup).
  function notifyError(err: unknown) {
    notifications.show({
      title: tt('errors.title'),
      message: formatError(err),
      color: 'red',
    });
  }

  /**
   * Runs a resolved import decision. Imports share the Add flow's preview
   * gate, but their direct arm differs from the clipboard one in error
   * semantics: HTML whose extraction yields nothing is an explicit red
   * error for an explicitly chosen source — never the neutral
   * «Буфер обмена пуст» hint.
   */
  async function runImportSource(source: ImportSource) {
    setPending(true);
    try {
      const action = resolveImport(source, {
        previewEnabled: config?.preview_dialog_enabled ?? false,
      });
      switch (action.kind) {
        case 'preview':
          setRegenEntry(null); // only one preview open at a time
          setPreviewText(action.text);
          setPreviewFormat(action.format ?? null);
          setPreviewPlainFallback(null);
          setPreviewSource(source.kind === 'url' ? 'url' : 'file');
          setPreviewOpen(true);
          setPending(false);
          return;
        case 'direct-plain':
          await doAddEntry(action.text, true, action.format, undefined, source.kind === 'url' ? 'url' : 'file');
          return;
        case 'direct-html': {
          const ingested = await addHtmlEntry(action.html, true, source.kind === 'url' ? 'url' : 'file');
          if (!ingested) {
            notifications.show({
              title: tt('errors.title'),
              message: tt('app.html.extract_failed'),
              color: 'red',
            });
            setPending(false);
          }
          return;
        }
        case 'empty':
          // Unreachable for imports: a decoded source is never empty here,
          // backend rejects empty payloads with input.empty first.
          setPending(false);
          return;
        default: {
          const exhaustive: never = action;
          throw new Error(`unknown import action: ${JSON.stringify(exhaustive)}`);
        }
      }
    } catch (err) {
      // Coded import errors (SPA shell, empty page, decode failure) surface
      // as notifications; no dialog opens (preview-dialog spec).
      notifyError(err);
      setPending(false);
    }
  }

  /** Reads a picked/dropped file and continues the flow; the optional
   *  encoding-dialog branch pauses before routing so the user can correct
   *  auto-detection. */
  async function startFileImport(path: string, encodingDialogFirst = false) {
    if (pending) return;
    setPending(true);
    try {
      const detected = await commands.readTextFile(path);
      if (encodingDialogFirst) {
        setPending(false);
        setEncodingDialog({ path, detected });
        return;
      }
      await runImportSource({ kind: 'file', fileName: path, text: detected.text });
    } catch (err) {
      notifyError(err);
      setPending(false);
    }
  }

  async function pickAndImport(encodingDialogFirst: boolean) {
    if (pending) return;
    try {
      const picked = await commands.pickImportFile();
      if (!picked) return;
      await startFileImport(picked, encodingDialogFirst);
    } catch (err) {
      notifyError(err);
      setPending(false);
    }
  }

  async function importFromUrl(url: string) {
    if (pending) return;
    setPending(true);
    try {
      const fetched = await commands.fetchUrlText(url);
      await runImportSource({ kind: 'url', body: fetched.text, contentType: fetched.content_type });
    } catch (err) {
      notifyError(err);
      setPending(false);
    }
  }

  function confirmEncodingDialog(result: ReadTextFileResult) {
    const state = encodingDialog;
    setEncodingDialog(null);
    if (state) void runImportSource({ kind: 'file', fileName: state.path, text: result.text });
  }

  // Drag & drop: subscribe once via the webview-level API (HTML5 DnD events
  // are suppressed by Tauri's native handler); the latest committed handlers
  // are reached through a ref so locale/pending state never goes stale.
  const dropHandlerRef = useRef<(paths: string[]) => void>(() => {});
  useEffect(() => {
    dropHandlerRef.current = (paths) => {
      if (pending || paths.length !== 1) return; // zero/several items ignored silently
      const dropped = paths[0].trim();
      if (/^https?:\/\//i.test(dropped)) {
        void importFromUrl(dropped);
        return;
      }
      // Unsupported extensions are ignored without an error (spec scenario
      // "Unsupported drop is ignored"); paths that are neither URLs nor
      // importable files stay silent too.
      if (!hasImportableExtension(dropped)) return;
      void startFileImport(dropped);
    };
  });

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload;
        // Overlay state derives from enter/leave transitions; some window
        // managers skip `over`, none of the logic needs it.
        if (payload.type === 'enter') {
          setDropDepth((d) => Math.min(d + 1, 4));
        } else if (payload.type === 'leave' || payload.type === 'drop') {
          setDropDepth(0);
        }
        if (payload.type === 'drop') dropHandlerRef.current(payload.paths);
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // Subscription failed (pre-webview teardown): drops stay inert, the
        // menu-driven import paths are unaffected. Logged for diagnosis.
        console.warn('drag-drop subscription failed');
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  // Shared direct-HTML executor for the Add button and the paste listener.
  // HTML whose markup yields no readable text (e.g. pure nav/button chrome)
  // falls back to the plain flavor; with no plain flavor there is nothing
  // to ingest — `notifyOnEmpty` decides between the explicit-click hint and
  // a silent no-op (paste stays silent by design).
  async function runDirectHtml(
    html: string,
    plainFallback: string | null,
    notifyOnEmpty: boolean,
  ): Promise<void> {
    // Only the clipboard paths (Add button, paste) go through here; imports
    // call addHtmlEntry with their own source.
    if (await addHtmlEntry(html, true, 'clipboard')) return;
    if (plainFallback) {
      await doAddEntry(plainFallback, true, undefined, undefined, 'clipboard');
      return;
    }
    if (notifyOnEmpty) showEmptyClipboardHint();
    setPending(false);
  }

  async function addEntry() {
    if (pending) return;
    setPending(true);

    try {
      // Best-effort HTML detection: navigator.clipboard.read() may be
      // unavailable or blocked in the WKWebView — any failure just falls
      // through to the plain-text path below. On WebView2 (Windows) the
      // read succeeds after a one-time permission grant.
      let clipboardHtml: string | null = null;
      try {
        const items = await navigator.clipboard.read();
        for (const item of items) {
          if (item.types.includes('text/html')) {
            const rawHtml = await (await item.getType('text/html')).text();
            if (rawHtml.trim()) clipboardHtml = rawHtml;
            break;
          }
        }
      } catch (err) {
        // No HTML clipboard access — continue with plain text. Logged so a
        // genuinely broken read stays diagnosable instead of looking like
        // "no HTML flavor" forever.
        console.warn('clipboard HTML read failed:', err);
      }

      // Read via tauri-plugin-clipboard-manager: the plugin goes through
      // the Tauri webview's native clipboard bridge, which handles Wayland
      // / KDE data reliably — unlike the Rust-side `arboard` crate which
      // silently fails with `ContentNotAvailable` on KDE Plasma 6, and
      // unlike `navigator.clipboard.readText` which is gated by WebKit
      // permission policies in the WKWebView.
      // An empty clipboard must not look like an app failure (#194): the Add
      // button doubles as "read what I just copied", so both an empty result
      // and a read failure (Windows reports an empty clipboard as an error)
      // surface as a neutral hint instead of a red error.
      let clipboardText: string;
      try {
        clipboardText = (await readClipboardText()) ?? '';
      } catch (err) {
        // Windows reports an *empty* clipboard as an error, so the failure
        // still maps to the neutral hint — but the error is logged to tell
        // a real read failure apart from an empty clipboard.
        console.warn('clipboard text read failed:', err);
        clipboardText = '';
      }

      const action = resolveAddAction({
        html: clipboardHtml,
        plain: clipboardText,
        previewEnabled: config?.preview_dialog_enabled ?? false,
      });

      switch (action.kind) {
        case 'empty':
          showEmptyClipboardHint();
          setPending(false);
          return;
        case 'preview':
          // The preview gate applies to the HTML flavor too (#195): the raw
          // markup goes into the dialog, which detects the format itself,
          // instead of being ingested directly behind the user's back.
          setRegenEntry(null); // only one preview open at a time
          setPreviewText(action.text);
          setPreviewFormat(null);
          setPreviewPlainFallback(action.plainFallback);
          setPreviewSource('clipboard');
          setPreviewOpen(true);
          setPending(false);
          return;
        case 'direct-html':
          await runDirectHtml(action.html, action.plainFallback, true);
          return;
        case 'direct-plain':
          await doAddEntry(action.text, true);
          return;
        default: {
          // Exhaustiveness guard: a new AddAction variant must break the
          // build here instead of leaving the Add button stuck in pending.
          const exhaustive: never = action;
          throw new Error(`unknown add action: ${JSON.stringify(exhaustive)}`);
        }
      }
    } catch (err) {
      const message = formatError(err);
      notifications.show({ title: tt('errors.title'), message, color: 'red' });
      setPending(false);
    }
  }

  // HTML ingestion path: sanitize → extract → add an entry with format
  // "html" (extracted text goes to synthesis, sanitized markup is kept for
  // rendering). Returns false when the HTML yields no readable text, so the
  // caller can fall back to the plain-text flavor.
  async function addHtmlEntry(
    rawHtml: string,
    playWhenReady: boolean,
    source: EntrySource,
  ): Promise<boolean> {
    const action = resolveIngest(rawHtml, 'html');
    if (action.kind !== 'html') return false;
    await doAddEntry(action.text, playWhenReady, 'html', action.htmlSource, source);
    return true;
  }

  async function doAddEntry(
    text: string,
    playWhenReady: boolean,
    format?: EntryFormat,
    htmlSource?: string,
    source: EntrySource = 'clipboard',
  ) {
    try {
      const entryId = await commands.addTextEntry(
        text,
        playWhenReady,
        format,
        htmlSource,
        source,
      );
      // Select the new entry so TextViewer swaps to its content; entry_updated
      // events from the backend will populate the full TextEntry shortly.
      useSelectedEntry.getState().setSelectedId(entryId);
      notifications.show({
        title: tt('app.added.title'),
        message: playWhenReady
          ? tt('app.added.now')
          : tt('app.added.later'),
        color: 'green',
      });
    } catch (err) {
      const message = formatError(err);
      notifications.show({ title: tt('errors.title'), message, color: 'red' });
    } finally {
      setPending(false);
    }
  }

  function handlePreviewSynthesize(
    finalText: string,
    skipShortTexts: boolean,
    playWhenReady: boolean,
    sourceFormat: EntryFormat,
  ) {
    setPreviewOpen(false);
    setPreviewFormat(null);
    const plainFallback = previewPlainFallback;
    setPreviewPlainFallback(null);
    if (skipShortTexts && config) {
      // Persist user preference: disable preview dialog
      commands.updateConfig({ preview_dialog_enabled: false }).catch(() => {});
      setConfig({ ...config, preview_dialog_enabled: false });
    }
    setPending(true);
    // finalText reflects user edits from the preview dialog; fall back to the
    // captured clipboard text if the user didn't edit or cleared the field.
    // An explicit `html` choice has no plain fallback (preview-dialog spec):
    // an empty extraction is an error, not a silent plain ingest.
    const action = resolveIngest(finalText || previewText, sourceFormat);
    switch (action.kind) {
      case 'reject':
        // Auto-detected HTML flavor (the dialog itself opened with the raw
        // markup and the plain flavor was carried along) falls back to the
        // plain text, exactly like the ungated direct path. With no carried
        // fallback the `html` selection was explicit — keep the red error.
        if (sourceFormat === 'html' && plainFallback) {
          void doAddEntry(plainFallback, playWhenReady, undefined, undefined, previewSource);
          return;
        }
        notifications.show({
          title: tt('errors.title'),
          message: tt('app.html.extract_failed'),
          color: 'red',
        });
        setPending(false);
        return;
      case 'html':
        void doAddEntry(action.text, playWhenReady, 'html', action.htmlSource, previewSource);
        return;
      case 'direct':
        void doAddEntry(action.text, playWhenReady, action.format, undefined, previewSource);
        return;
    }
  }

  function handlePreviewCancel() {
    setPreviewOpen(false);
    setPreviewFormat(null);
    setPreviewPlainFallback(null);
    setPending(false);
  }

  // Regeneration confirm: the delete-then-synthesize sequence lives entirely
  // behind the backend command, so the old audio is only dropped now that the
  // user has seen the preview and confirmed (preview-dialog spec, "Regeneration
  // preview"). Cancel never reaches here.
  function handleRegenConfirm(playWhenReady: boolean) {
    const entry = regenEntry;
    setRegenEntry(null);
    if (!entry) return;
    commands
      .regenerateEntry(entry.id, playWhenReady)
      .then(() => {
        notifications.show({
          title: tt('queue.notify.regenerating.title'),
          message: tt('queue.notify.regenerating.message'),
          color: 'blue',
        });
      })
      .catch((e) => {
        notifications.show({
          title: tt('errors.title'),
          message: tt('queue.notify.regenerate_failed', [formatError(e)]),
          color: 'red',
        });
      });
  }

  return (
    <MantineAppShell
      header={{ height: 74 }}
      navbar={{ width: navWidth, breakpoint: 'sm' }}
      padding="md"
    >
      <MantineAppShell.Header>
        <Player onOpenSettings={() => setSettingsOpened(true)} />
      </MantineAppShell.Header>

      <SettingsModal
        opened={settingsOpened}
        onClose={() => setSettingsOpened(false)}
        onSaved={() => {
          commands.getConfig().then(setConfig).catch(() => {});
        }}
      />

      <SileroBundlePrompt
        opened={bundlePromptOpen}
        onClose={() => setBundlePromptOpen(false)}
      />

      <MantineAppShell.Navbar p="md">
        {/* Inner relative wrapper for the absolute resize handle.  Anchoring
            the handle on the Navbar itself (via `position: relative` on
            Navbar) breaks Mantine's `position: fixed` styling for the
            overlay, which then takes block-flow space and pushes Main
            below the viewport.  Keep the wrapper inside the fixed Navbar
            instead. */}
        <div
          style={{
            position: 'relative',
            height: '100%',
            display: 'flex',
            flexDirection: 'column',
            minHeight: 0,
          }}
        >
          <Group justify="space-between" align="center" mb="xs" wrap="nowrap">
            <Title order={6} c="dimmed">{tt('app.queue.title')}</Title>
            {/* Split-button (ui spec): primary click keeps the clipboard Add
                flow; the dropdown shares one import flow with drag & drop. */}
            <Group gap={4} wrap="nowrap">
              <Button
                size="xs"
                color="blue"
                loading={pending}
                disabled={pending}
                onClick={() => addEntry()}
                style={{
                  borderTopRightRadius: 0,
                  borderBottomRightRadius: 0,
                }}
              >
                {tt('app.add')}
              </Button>
              <Menu withinPortal position="bottom-end" disabled={pending}>
                <Menu.Target>
                  <ActionIcon
                    size="xs"
                    color="blue"
                    variant="filled"
                    disabled={pending}
                    aria-label={tt('app.import.menu.file')}
                    style={{
                      borderTopLeftRadius: 0,
                      borderBottomLeftRadius: 0,
                      height: 'auto',
                      minHeight: 'var(--button-height-xs)',
                    }}
                  >
                    <IconChevronDown />
                  </ActionIcon>
                </Menu.Target>
                <Menu.Dropdown>
                  <Menu.Item onClick={() => void pickAndImport(false)}>
                    {tt('app.import.menu.file')}
                  </Menu.Item>
                  <Menu.Item onClick={() => void pickAndImport(true)}>
                    {tt('app.import.menu.file_encoding')}
                  </Menu.Item>
                  <Menu.Item onClick={() => setUrlDialogOpen(true)}>
                    {tt('app.import.menu.url')}
                  </Menu.Item>
                </Menu.Dropdown>
              </Menu>
            </Group>
          </Group>
          <TextInput
            ref={searchInputRef}
            placeholder={tt('app.search.placeholder')}
            value={query}
            onChange={(e) => setQuery(e.currentTarget.value)}
            leftSection={<IconSearch />}
            rightSection={
              query ? (
                <CloseButton
                  size="sm"
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => setQuery('')}
                  aria-label={tt('app.search.clear')}
                />
              ) : null
            }
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                e.preventDefault();
                setQuery('');
                e.currentTarget.blur();
              }
            }}
            size="xs"
            mb="xs"
          />
          <QueueList
            onRegenerate={(entry) => {
              // The dialogs are non-modal, so both could stack centered on
              // top of each other with clashing window-level ESC handlers;
              // only one preview may be open at a time.
              setPreviewOpen(false);
              setRegenEntry(entry);
            }}
          />
          <div
            onPointerDown={onNavResizeDown}
            onPointerMove={onNavResizeMove}
            onPointerUp={onNavResizeUp}
            onPointerCancel={onNavResizeUp}
            style={{
              position: 'absolute',
              top: 0,
              right: 'calc(-1 * var(--mantine-spacing-md) - 3px)',
              bottom: 0,
              width: 6,
              cursor: 'col-resize',
              zIndex: 10,
              touchAction: 'none',
            }}
            aria-label={tt('app.nav.resize')}
          />
        </div>
      </MantineAppShell.Navbar>

      <MantineAppShell.Main
        style={{
          // Mantine's Main has min-height: 100dvh and a top padding equal
          // to the fixed header's height, so its content box already maps
          // to the viewport area below the header.  Just turn it into a
          // flex column so TextViewer's flex:1 child can fill the
          // available height.
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        <TextViewer entry={selectedEntry} />
      </MantineAppShell.Main>

      <PreviewDialog
        opened={previewOpen}
        text={previewText}
        initialFormat={previewFormat ?? undefined}
        onSynthesize={handlePreviewSynthesize}
        onCancel={handlePreviewCancel}
      />

      {/* Regeneration instance (preview-dialog spec, "Regeneration preview"):
          mounted separately from the Add-flow dialog so the two flows keep
          independent state; the component renders null while closed. Of the
          dialog's confirmation decision only «Read Now» is consumed — the
          text and format are the entry's own immutable values. */}
      <PreviewDialog
        opened={regenEntry !== null}
        mode="regenerate"
        text={regenEntry?.original_text ?? ''}
        onSynthesize={(_finalText, _skipShortTexts, playWhenReady) =>
          handleRegenConfirm(playWhenReady)
        }
        onCancel={() => setRegenEntry(null)}
      />

      <UrlImportDialog
        opened={urlDialogOpen}
        onConfirm={(url) => {
          setUrlDialogOpen(false);
          void importFromUrl(url);
        }}
        onClose={() => setUrlDialogOpen(false)}
      />

      {encodingDialog && (
        <EncodingDialog
          opened
          path={encodingDialog.path}
          initial={encodingDialog.detected}
          onConfirm={confirmEncodingDialog}
          onCancel={() => setEncodingDialog(null)}
        />
      )}

      {/* Full-window drop overlay (ui spec): visible only while a drag is
          over the window, never intercepts pointer events. */}
      {dropDepth > 0 && (
        <div className={classes.dropOverlay} aria-hidden>
          <div className={classes.dropCard}>{tt('app.import.drop_overlay')}</div>
        </div>
      )}
    </MantineAppShell>
  );
}
