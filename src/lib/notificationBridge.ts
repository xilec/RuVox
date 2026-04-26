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

  // Toast lifecycle keyed by entry_id: synth-<id>.
  // ttsd does not stream chunk-level progress, so the toast just reflects
  // status transitions: processing → ready/error.
  const synthesisShown = new Set<string>();
  const truncate = (text: string, max = 60): string =>
    text.length > max ? `${text.slice(0, max).trimEnd()}…` : text;

  unlisteners.push(
    await events.entryUpdated((p) => {
      const { id, status, original_text } = p.entry;
      const toastId = `synth-${id}`;

      if (status === 'processing') {
        if (synthesisShown.has(id)) return;
        synthesisShown.add(id);
        notifications.show({
          id: toastId,
          title: 'Синтез речи',
          message: truncate(original_text),
          loading: true,
          autoClose: false,
        });
      } else if (status === 'ready' && synthesisShown.has(id)) {
        synthesisShown.delete(id);
        notifications.update({
          id: toastId,
          title: 'Готово',
          message: truncate(original_text),
          color: 'green',
          loading: false,
          autoClose: 2000,
        });
      } else if (status === 'error' && synthesisShown.has(id)) {
        synthesisShown.delete(id);
        notifications.hide(toastId);
      }
    }),
  );

  return () => {
    for (const unlisten of unlisteners) {
      unlisten();
    }
  };
}
