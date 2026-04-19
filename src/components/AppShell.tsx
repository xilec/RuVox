import { AppShell as MantineAppShell, Title, Group } from '@mantine/core';
import { ThemeSwitcher } from './ThemeSwitcher';
import { TextViewer } from './TextViewer';
import { QueueList } from './QueueList';
import { useSelectedEntry } from '../stores/selectedEntry';

export function AppShell() {
  const { selectedEntry } = useSelectedEntry();

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
            <ThemeSwitcher />
          </Group>
        </Group>
      </MantineAppShell.Header>

      <MantineAppShell.Navbar p="md">
        <Title order={6} c="dimmed" mb="xs">Очередь</Title>
        <QueueList />
      </MantineAppShell.Navbar>

      <MantineAppShell.Main>
        {/* Player placeholder (U3) */}
        <TextViewer entry={selectedEntry} />
      </MantineAppShell.Main>
    </MantineAppShell>
  );
}
