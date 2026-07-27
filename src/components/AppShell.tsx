import {
  AppShell as MantineAppShell,
  Title,
  Group,
  Button,
  TextInput,
  CloseButton,
  useMantineColorScheme,
} from '@mantine/core';
import { useHotkeys } from '@mantine/hooks';
import { notifications } from '@mantine/notifications';
import { useState, useEffect, useRef } from 'react';
import { readText as readClipboardText } from '@tauri-apps/plugin-clipboard-manager';
import { commands } from '../lib/tauri';
import type { UIConfig } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { sanitizeHtml } from '../lib/html';
import { extractTextForTts } from '../lib/htmlText';
import { TextViewer } from './TextViewer';
import { Player } from './Player';
import { QueueList } from './QueueList';
import { useSelectedEntry } from '../stores/selectedEntry';
import { useSearchQuery } from '../stores/searchQuery';
import { PreviewDialog } from '../dialogs/PreviewDialog';
import { SettingsModal } from '../dialogs/Settings';
import { IconSearch } from './icons';

export function AppShell() {
  const { selectedEntry } = useSelectedEntry();
  const { setColorScheme } = useMantineColorScheme();
  const [pending, setPending] = useState(false);
  const [settingsOpened, setSettingsOpened] = useState(false);
  const [previewOpen, setPreviewOpen] = useState(false);
  const [previewText, setPreviewText] = useState('');
  const [config, setConfig] = useState<UIConfig | null>(null);
  const configLoaded = useRef(false);
  const [navWidth, setNavWidth] = useState(280);
  const navResizeRef = useRef<{
    pointerId: number;
    startX: number;
    originW: number;
  } | null>(null);
  const { query, setQuery } = useSearchQuery();
  const searchInputRef = useRef<HTMLInputElement>(null);

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
      const target = e.target as HTMLElement | null;
      if (target?.closest('input, textarea, [contenteditable="true"]')) return;
      if (pending) return;

      const rawHtml = e.clipboardData?.getData('text/html') ?? '';
      const plain = e.clipboardData?.getData('text/plain') ?? '';
      if (!rawHtml.trim() && !plain.trim()) return;
      e.preventDefault();
      setPending(true);
      if (rawHtml.trim()) {
        void addHtmlEntry(rawHtml, true)
          .then((added) => (added ? undefined : doAddEntry(plain, true)))
          .catch((err) => {
            notifications.show({ title: 'Ошибка', message: formatError(err), color: 'red' });
            setPending(false);
          });
        return;
      }
      void doAddEntry(plain, true);
    }
    window.addEventListener('paste', handlePaste);
    return () => window.removeEventListener('paste', handlePaste);
    // Re-subscribing on `pending` is enough: addHtmlEntry/doAddEntry are
    // state-independent (they only touch commands, notifications, stores).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pending]);

  async function addEntry() {
    if (pending) return;
    setPending(true);

    try {
      // Best-effort HTML detection: navigator.clipboard.read() may be
      // unavailable or blocked in the WKWebView — any failure just falls
      // through to the plain-text path below.
      try {
        const items = await navigator.clipboard.read();
        for (const item of items) {
          if (item.types.includes('text/html')) {
            const rawHtml = await (await item.getType('text/html')).text();
            if (rawHtml.trim() && (await addHtmlEntry(rawHtml, true))) return;
            break;
          }
        }
      } catch {
        // No HTML clipboard access — continue with plain text.
      }

      // Read via tauri-plugin-clipboard-manager: the plugin goes through
      // the Tauri webview's native clipboard bridge, which handles Wayland
      // / KDE data reliably — unlike the Rust-side `arboard` crate which
      // silently fails with `ContentNotAvailable` on KDE Plasma 6, and
      // unlike `navigator.clipboard.readText` which is gated by WebKit
      // permission policies in the WKWebView.
      let clipboardText: string;
      try {
        clipboardText = (await readClipboardText()) ?? '';
      } catch {
        notifications.show({
          title: 'Ошибка',
          message: 'Не удалось прочитать буфер обмена',
          color: 'red',
        });
        setPending(false);
        return;
      }

      const previewEnabled = config?.preview_dialog_enabled ?? false;

      if (previewEnabled) {
        setPreviewText(clipboardText);
        setPreviewOpen(true);
        setPending(false);
        return;
      }

      await doAddEntry(clipboardText, true);
    } catch (err) {
      const message = formatError(err);
      notifications.show({ title: 'Ошибка', message, color: 'red' });
      setPending(false);
    }
  }

  // HTML ingestion path: sanitize → extract → add an entry with format
  // "html" (extracted text goes to synthesis, sanitized markup is kept for
  // rendering). Returns false when the HTML yields no readable text, so the
  // caller can fall back to the plain-text flavor.
  async function addHtmlEntry(rawHtml: string, playWhenReady: boolean): Promise<boolean> {
    const sanitized = sanitizeHtml(rawHtml);
    const extracted = extractTextForTts(sanitized);
    if (!extracted.trim()) return false;
    await doAddEntry(extracted, playWhenReady, sanitized);
    return true;
  }

  async function doAddEntry(text: string, playWhenReady: boolean, htmlSource?: string) {
    try {
      const entryId = await commands.addTextEntry(
        text,
        playWhenReady,
        htmlSource ? 'html' : undefined,
        htmlSource,
      );
      // Select the new entry so TextViewer swaps to its content; entry_updated
      // events from the backend will populate the full TextEntry shortly.
      useSelectedEntry.getState().setSelectedId(entryId);
      notifications.show({
        title: 'Добавлено в очередь',
        message: playWhenReady
          ? 'Текст будет воспроизведён сразу'
          : 'Текст добавлен для прослушивания позже',
        color: 'green',
      });
    } catch (err) {
      const message = formatError(err);
      notifications.show({ title: 'Ошибка', message, color: 'red' });
    } finally {
      setPending(false);
    }
  }

  function handlePreviewSynthesize(
    finalText: string,
    skipShortTexts: boolean,
    playWhenReady: boolean,
  ) {
    setPreviewOpen(false);
    if (skipShortTexts && config) {
      // Persist user preference: disable preview dialog
      commands.updateConfig({ preview_dialog_enabled: false }).catch(() => {});
      setConfig({ ...config, preview_dialog_enabled: false });
    }
    setPending(true);
    // finalText reflects user edits from the preview dialog; fall back to the
    // captured clipboard text if the user didn't edit or cleared the field.
    void doAddEntry(finalText || previewText, playWhenReady);
  }

  function handlePreviewCancel() {
    setPreviewOpen(false);
    setPending(false);
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
            <Title order={6} c="dimmed">Очередь</Title>
            <Button
              size="xs"
              color="blue"
              loading={pending}
              disabled={pending}
              onClick={() => addEntry()}
            >
              Add
            </Button>
          </Group>
          <TextInput
            ref={searchInputRef}
            placeholder="Поиск по записям"
            value={query}
            onChange={(e) => setQuery(e.currentTarget.value)}
            leftSection={<IconSearch />}
            rightSection={
              query ? (
                <CloseButton
                  size="sm"
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => setQuery('')}
                  aria-label="Очистить поиск"
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
          <QueueList />
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
            aria-label="Изменить ширину списка"
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
        onSynthesize={handlePreviewSynthesize}
        onCancel={handlePreviewCancel}
      />
    </MantineAppShell>
  );
}
