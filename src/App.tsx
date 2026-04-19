import '@mantine/core/styles.css';
import '@mantine/notifications/styles.css';
import { MantineProvider } from '@mantine/core';
import { ModalsProvider } from '@mantine/modals';
import { Notifications } from '@mantine/notifications';
import { AppShell } from './components/AppShell';

export function App() {
  return (
    <MantineProvider defaultColorScheme="auto">
      <ModalsProvider>
        <Notifications />
        <AppShell />
      </ModalsProvider>
    </MantineProvider>
  );
}
