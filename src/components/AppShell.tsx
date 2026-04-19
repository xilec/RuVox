import { AppShell as MantineAppShell, Title, Group, Stack, Button } from '@mantine/core';
import { useHotkeys } from '@mantine/hooks';
import { notifications } from '@mantine/notifications';
import { useState } from 'react';
import { commands } from '../lib/tauri';
import { ThemeSwitcher } from './ThemeSwitcher';
import { TextViewer } from './TextViewer';
import { Player } from './Player';
import { QueueList } from './QueueList';
import { useSelectedEntry } from '../stores/selectedEntry';

export function AppShell() {
  const { selectedEntry } = useSelectedEntry();
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
            </Group>
          </Group>
          <Player />
        </Stack>
      </MantineAppShell.Header>

      <MantineAppShell.Navbar p="md">
        <Title order={6} c="dimmed" mb="xs">Очередь</Title>
        <QueueList />
      </MantineAppShell.Navbar>

      <MantineAppShell.Main>
        <TextViewer entry={selectedEntry} />
      </MantineAppShell.Main>
    </MantineAppShell>
  );
}
