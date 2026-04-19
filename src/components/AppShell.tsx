import { AppShell as MantineAppShell, Title, Group, Button } from '@mantine/core';
import { useHotkeys } from '@mantine/hooks';
import { notifications } from '@mantine/notifications';
import { useState } from 'react';
import type { TextEntry } from '../lib/tauri';
import { commands } from '../lib/tauri';
import { ThemeSwitcher } from './ThemeSwitcher';
import { TextViewer } from './TextViewer';

export function AppShell() {
  const [selectedEntry] = useState<TextEntry | null>(null);
  const [pending, setPending] = useState(false);

  async function addEntry(playWhenReady: boolean) {
    if (pending) return;
    setPending(true);
    try {
      await commands.addClipboardEntry(playWhenReady);
      notifications.show({
        title: 'Добавлено в очередь',
        message: playWhenReady ? 'Текст будет воспроизведён сразу' : 'Текст добавлен для прослушивания позже',
        color: 'green',
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      notifications.show({
        title: 'Ошибка',
        message,
        color: 'red',
      });
    } finally {
      setPending(false);
    }
  }

  useHotkeys([
    ['ctrl+shift+1', () => addEntry(true)],
    ['ctrl+shift+2', () => addEntry(false)],
  ]);

  return (
    <MantineAppShell
      header={{ height: 56 }}
      navbar={{ width: 280, breakpoint: 'sm' }}
      padding="md"
    >
      <MantineAppShell.Header>
        <Group h="100%" px="md" justify="space-between">
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
          </Group>
        </Group>
      </MantineAppShell.Header>

      <MantineAppShell.Navbar p="md">
        <Title order={6} c="dimmed">Очередь</Title>
        {/* QueueList placeholder (task U2) */}
      </MantineAppShell.Navbar>

      <MantineAppShell.Main>
        {/* Player placeholder (U3) */}
        <TextViewer entry={selectedEntry} />
      </MantineAppShell.Main>
    </MantineAppShell>
  );
}
