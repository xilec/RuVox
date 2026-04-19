import { AppShell as MantineAppShell, Title, Group } from '@mantine/core';
import { ThemeSwitcher } from './ThemeSwitcher';

export function AppShell() {
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
        <Title order={6} c="dimmed">Очередь</Title>
        {/* QueueList placeholder (task U2) */}
      </MantineAppShell.Navbar>

      <MantineAppShell.Main>
        {/* Player placeholder (U3) + TextViewer placeholder (U4) */}
        <Title order={6} c="dimmed">Просмотр текста появится в U4</Title>
      </MantineAppShell.Main>
    </MantineAppShell>
  );
}
