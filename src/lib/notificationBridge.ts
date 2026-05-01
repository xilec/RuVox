import { notifications } from '@mantine/notifications';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { events } from './tauri';

/**
 * Subscribe to backend events and show Mantine notifications.
 * Returns a cleanup function that unsubscribes all handlers.
 */
export async function setupNotificationBridge(): Promise<() => void> {
  const unlisteners: UnlistenFn[] = [];

  // Toast routing for the model_loading → model_loaded/model_error sequence:
  // - Cold-start path uses id 'model-loading'.
  // - Post-respawn path uses id 'ttsd-restart' so the yellow "перезапускается"
  //   toast morphs into "загружаю модель..." → "TTS восстановлен" / error
  //   without ever disappearing silently. `restartActive` flips on
  //   ttsd_restarting and back off when the post-respawn warmup completes
  //   (success, model_error, or tts_fatal).
  let restartActive = false;
  const RESTART_TOAST_ID = 'ttsd-restart';

  unlisteners.push(
    await events.ttsdRestarting(() => {
      restartActive = true;
      notifications.show({
        id: RESTART_TOAST_ID,
        title: 'TTS перезапускается',
        message: 'Подождите несколько секунд — процесс будет запущен заново.',
        color: 'yellow',
        loading: true,
        autoClose: false,
      });
    }),
  );

  unlisteners.push(
    await events.ttsFatal((p) => {
      restartActive = false;
      notifications.hide(RESTART_TOAST_ID);
      notifications.show({
        title: 'TTS не запускается',
        message: p.message || 'Не удалось перезапустить процесс синтеза.',
        color: 'red',
        autoClose: false,
      });
    }),
  );

  unlisteners.push(
    await events.modelLoading(() => {
      if (restartActive) {
        notifications.update({
          id: RESTART_TOAST_ID,
          title: 'Загружаю модель TTS',
          message: 'Перезапуск завершён, повторная загрузка модели...',
          color: 'yellow',
          loading: true,
          autoClose: false,
        });
        return;
      }
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
      if (restartActive) {
        restartActive = false;
        notifications.update({
          id: RESTART_TOAST_ID,
          title: 'TTS восстановлен',
          message: 'Синтез речи снова доступен.',
          color: 'green',
          loading: false,
          autoClose: 3000,
        });
        return;
      }
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
      if (restartActive) {
        restartActive = false;
        notifications.update({
          id: RESTART_TOAST_ID,
          title: 'Ошибка загрузки модели TTS',
          message: p.message,
          color: 'red',
          loading: false,
          autoClose: 8000,
        });
        return;
      }
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

  // Voice-download lifecycle: each voice gets its own toast id keyed on the
  // voice id so concurrent downloads (rare but possible) don't trample each
  // other. Progress events update the body with a kilobyte/megabyte tally;
  // started/finished flip the toast colour and loading state.
  const voiceToastId = (voice: string) => `voice-download-${voice}`;
  const fmtMb = (bytes: number) => `${(bytes / (1024 * 1024)).toFixed(1)} МБ`;

  unlisteners.push(
    await events.voiceDownloadStarted((p) => {
      notifications.show({
        id: voiceToastId(p.voice),
        title: `Загрузка голоса ${p.voice}`,
        message: 'Запрашиваю файлы…',
        loading: true,
        autoClose: false,
      });
    }),
  );

  unlisteners.push(
    await events.voiceDownloadProgress((p) => {
      // `skipped: true` events are no-ops in the toast — they fire when the
      // file is already on disk and we don't want to confuse the user with
      // 0/0 readouts.
      if (p.skipped) return;
      const total = p.total_bytes ?? 0;
      const file = p.file_kind === 'onnx' ? 'модель' : 'конфиг';
      const message = total > 0
        ? `${file} (${p.file_idx + 1}/${p.total_files}): ${fmtMb(p.downloaded_bytes)} / ${fmtMb(total)}`
        : `${file}: ${fmtMb(p.downloaded_bytes)}`;
      notifications.update({
        id: voiceToastId(p.voice),
        title: `Загрузка голоса ${p.voice}`,
        message,
        loading: true,
        autoClose: false,
      });
    }),
  );

  unlisteners.push(
    await events.voiceDownloadFinished((p) => {
      if (p.ok) {
        notifications.update({
          id: voiceToastId(p.voice),
          title: 'Голос загружен',
          message: `Голос «${p.voice}» готов к использованию.`,
          color: 'green',
          loading: false,
          autoClose: 3000,
        });
      } else {
        notifications.update({
          id: voiceToastId(p.voice),
          title: 'Не удалось загрузить голос',
          message: p.message ?? `Голос «${p.voice}» не загружен.`,
          color: 'red',
          loading: false,
          autoClose: 8000,
        });
      }
    }),
  );

  return () => {
    for (const unlisten of unlisteners) {
      unlisten();
    }
  };
}
