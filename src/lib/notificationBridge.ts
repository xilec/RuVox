import { notifications } from '@mantine/notifications';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { events } from './tauri';

/**
 * Subscribe to backend events and show Mantine notifications.
 * Returns a cleanup function that unsubscribes all handlers.
 */
export async function setupNotificationBridge(): Promise<() => void> {
  const unlisteners: UnlistenFn[] = [];

  unlisteners.push(
    await events.modelLoading(() => {
      notifications.show({
        id: 'model-loading',
        title: 'Загрузка модели TTS',
        message: 'Первый запуск может занять несколько минут...',
        loading: true,
        autoClose: false,
      });
    }),
  );

  unlisteners.push(
    await events.modelLoaded(() => {
      notifications.update({
        id: 'model-loading',
        title: 'Модель TTS загружена',
        message: 'Готово к синтезу речи',
        color: 'green',
        loading: false,
        autoClose: 3000,
      });
    }),
  );

  unlisteners.push(
    await events.modelError((p) => {
      notifications.update({
        id: 'model-loading',
        title: 'Ошибка загрузки модели TTS',
        message: p.message,
        color: 'red',
        loading: false,
        autoClose: 8000,
      });
    }),
  );

  unlisteners.push(
    await events.ttsError((p) => {
      notifications.show({
        id: `tts-error-${p.entry_id}`,
        title: 'Ошибка синтеза',
        message: p.message,
        color: 'red',
        autoClose: 5000,
      });
    }),
  );

  unlisteners.push(
    await events.synthesisProgress((p) => {
      notifications.update({
        id: `synth-${p.entry_id}`,
        title: 'Синтез речи',
        message: `${Math.round(p.progress * 100)}%`,
        loading: true,
        autoClose: false,
      });
    }),
  );

  return () => {
    for (const unlisten of unlisteners) {
      unlisten();
    }
  };
}
