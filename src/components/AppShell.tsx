import { AppShell as MantineAppShell, Title, Group, Stack, Button, ActionIcon, Tooltip } from '@mantine/core';
import { useHotkeys } from '@mantine/hooks';
import { notifications } from '@mantine/notifications';
import { useState, useEffect, useRef } from 'react';
import { commands } from '../lib/tauri';
import type { UIConfig } from '../lib/tauri';
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
  const [pendingPlayWhenReady, setPendingPlayWhenReady] = useState(false);
  const [config, setConfig] = useState<UIConfig | null>(null);
  const configLoaded = useRef(false);

  useEffect(() => {
    if (configLoaded.current) return;
    configLoaded.current = true;
    commands.getConfig().then(setConfig).catch(() => {
      // Config load failure is non-fatal; preview will be skipped
    });
  }, []);

  async function addEntry(playWhenReady: boolean) {
    if (pending) return;
    setPending(true);

    try {
      const previewEnabled = config?.preview_dialog_enabled ?? false;
      const threshold = config?.preview_threshold ?? 200;

      if (previewEnabled) {
        let clipboardText: string | null = null;
        try {
          clipboardText = await navigator.clipboard.readText();
        } catch {
          // No clipboard permission in Tauri webview — skip preview
        }

        if (clipboardText !== null && clipboardText.length > threshold) {
          setPreviewText(clipboardText);
          setPendingPlayWhenReady(playWhenReady);
          setPreviewOpen(true);
          setPending(false);
          return;
        }
      }

      await doAddEntry(playWhenReady);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      notifications.show({ title: 'Ошибка', message, color: 'red' });
      setPending(false);
    }
  }

  async function doAddEntry(playWhenReady: boolean) {
    try {
      await commands.addClipboardEntry(playWhenReady);
      notifications.show({
        title: 'Добавлено в очередь',
        message: playWhenReady
          ? 'Текст будет воспроизведён сразу'
          : 'Текст добавлен для прослушивания позже',
        color: 'green',
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      notifications.show({ title: 'Ошибка', message, color: 'red' });
    } finally {
      setPending(false);
    }
  }

  function handlePreviewSynthesize(_finalText: string, skipShortTexts: boolean) {
    setPreviewOpen(false);
    if (skipShortTexts && config) {
      // Persist user preference: disable preview dialog
      commands.updateConfig({ preview_dialog_enabled: false }).catch(() => {});
      setConfig({ ...config, preview_dialog_enabled: false });
    }
    setPending(true);
    doAddEntry(pendingPlayWhenReady);
  }

  function handlePreviewCancel() {
    setPreviewOpen(false);
    setPending(false);
  }

  useHotkeys([
    ['ctrl+shift+1', () => addEntry(true)],
    ['ctrl+shift+2', () => addEntry(false)],
  ]);

  return (
    <MantineAppShell
      header={{ height: 108 }}
      navbar={{ width: 280, breakpoint: 'sm' }}
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
                onClick={() => addEntry(true)}
              >
                Read Now
              </Button>
              <Button
                variant="default"
                loading={pending}
                disabled={pending}
                onClick={() => addEntry(false)}
              >
                Read Later
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
      />

      <MantineAppShell.Navbar p="md">
        <Title order={6} c="dimmed" mb="xs">Очередь</Title>
        <QueueList />
      </MantineAppShell.Navbar>

      {/* display:flex + minH:0 lets TextViewer fill Main reliably under
          Mantine AppShell, even when the window is maximized on Wayland
          where the default flex-item min-height can collapse to 0. */}
      <MantineAppShell.Main style={{ display: 'flex', flexDirection: 'column', minHeight: 0 }}>
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
