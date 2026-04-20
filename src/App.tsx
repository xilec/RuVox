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

    // Disable the default webview context menu; editable elements keep their
    // native menu (cut/copy/paste) so the Edit-mode Textarea still works.
    const blockContextMenu = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && (target.isContentEditable || target.tagName === 'TEXTAREA' || target.tagName === 'INPUT')) {
        return;
      }
      e.preventDefault();
    };
    window.addEventListener('contextmenu', blockContextMenu);

    return () => {
      cleanup?.();
      window.removeEventListener('contextmenu', blockContextMenu);
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
