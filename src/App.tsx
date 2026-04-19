import '@mantine/core/styles.css';
import '@mantine/notifications/styles.css';
import { useEffect } from 'react';
import { MantineProvider } from '@mantine/core';
import { ModalsProvider } from '@mantine/modals';
import { Notifications } from '@mantine/notifications';
import { AppShell } from './components/AppShell';
import { setupNotificationBridge } from './lib/notificationBridge';

export function App() {
  useEffect(() => {
    let cleanup: (() => void) | undefined;
    setupNotificationBridge().then((fn) => {
      cleanup = fn;
    });
    return () => {
      cleanup?.();
    };
  }, []);

  return (
    <MantineProvider defaultColorScheme="auto">
      <ModalsProvider>
        <Notifications />
        <AppShell />
      </ModalsProvider>
    </MantineProvider>
  );
}
