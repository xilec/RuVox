import { AppShell as MantineAppShell, Title, Group, ActionIcon, Tooltip } from '@mantine/core';
import { useState } from 'react';
import type { TextEntry } from '../lib/tauri';
import { ThemeSwitcher } from './ThemeSwitcher';
import { TextViewer } from './TextViewer';
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
  const [selectedEntry] = useState<TextEntry | null>(null);
  const [settingsOpened, setSettingsOpened] = useState(false);

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
      </MantineAppShell.Header>

      <SettingsModal
        opened={settingsOpened}
        onClose={() => setSettingsOpened(false)}
      />

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
