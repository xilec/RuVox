import { AppShell, MantineProvider, Title } from '@mantine/core';
import { ModalsProvider } from '@mantine/modals';
import { Notifications } from '@mantine/notifications';

export function App() {
  return (
    <MantineProvider defaultColorScheme="auto">
      <ModalsProvider>
        <Notifications />
        <AppShell header={{ height: 56 }} padding="md">
          <AppShell.Header>
            <Title order={3} px="md" py="sm">RuVox 2</Title>
          </AppShell.Header>
          <AppShell.Main>
            Работающий скелет Tauri+React+Mantine. Интерфейс и логика — в следующих задачах.
          </AppShell.Main>
        </AppShell>
      </ModalsProvider>
    </MantineProvider>
  );
}
