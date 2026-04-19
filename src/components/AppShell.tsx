import { AppShell as MantineAppShell, Title, Group, Stack } from '@mantine/core';
import { useState } from 'react';
import type { TextEntry } from '../lib/tauri';
import { ThemeSwitcher } from './ThemeSwitcher';
import { TextViewer } from './TextViewer';
import { Player } from './Player';

export function AppShell() {
  const [selectedEntry] = useState<TextEntry | null>(null);

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
              <ThemeSwitcher />
            </Group>
          </Group>
          <Player />
        </Stack>
      </MantineAppShell.Header>

      <MantineAppShell.Navbar p="md">
        <Title order={6} c="dimmed">Очередь</Title>
        {/* QueueList placeholder (task U2) */}
      </MantineAppShell.Navbar>

      <MantineAppShell.Main>
        <TextViewer entry={selectedEntry} />
      </MantineAppShell.Main>
    </MantineAppShell>
  );
}
