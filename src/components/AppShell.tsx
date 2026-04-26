import { AppShell as MantineAppShell, Title, Group, Stack, Button, ActionIcon, Tooltip } from '@mantine/core';
import { notifications } from '@mantine/notifications';
import { useState, useEffect, useRef } from 'react';
import { readText as readClipboardText } from '@tauri-apps/plugin-clipboard-manager';
import { commands } from '../lib/tauri';
import type { UIConfig } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { ThemeSwitcher } from './ThemeSwitcher';
import { TextViewer } from './TextViewer';
import { Player } from './Player';
import { QueueList } from './QueueList';
import { useSelectedEntry } from '../stores/selectedEntry';
import { PreviewDialog } from '../dialogs/PreviewDialog';
import { SettingsModal } from '../dialogs/Settings';

function IconSettings({ size = 18 }: { size?: number }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  );
}

export function AppShell() {
  const { selectedEntry } = useSelectedEntry();
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
    commands.getConfig().then(setConfig).catch(() => {
      // Config load failure is non-fatal; preview will be skipped
    });
  }, []);

  async function addEntry() {
    if (pending) return;
    setPending(true);

    try {
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

  async function doAddEntry(text: string, playWhenReady: boolean) {
    try {
      const entryId = await commands.addTextEntry(text, playWhenReady);
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
    doAddEntry(finalText || previewText, playWhenReady);
  }

  function handlePreviewCancel() {
    setPreviewOpen(false);
    setPending(false);
  }

  return (
    <MantineAppShell
      header={{ height: 108 }}
      navbar={{ width: navWidth, breakpoint: 'sm' }}
      padding="md"
    >
      <MantineAppShell.Header>
        <Stack gap={0}>
          <Group h={56} px="md" justify="space-between">
            <Title order={3}>RuVox 2</Title>
            <Group>
              <Button
                color="blue"
                loading={pending}
                disabled={pending}
                onClick={() => addEntry()}
              >
                Add
              </Button>
              <ThemeSwitcher />
              <Tooltip label="Настройки">
                <ActionIcon
                  variant="subtle"
                  aria-label="Открыть настройки"
                  onClick={() => setSettingsOpened(true)}
                >
                  <IconSettings size={18} />
                </ActionIcon>
              </Tooltip>
            </Group>
          </Group>
          <Player />
        </Stack>
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
          <Title order={6} c="dimmed" mb="xs">Очередь</Title>
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
