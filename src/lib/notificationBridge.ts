import { notifications } from '@mantine/notifications';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { t } from './i18n';
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
        title: t('notify.ttsd.restarting.title'),
        message: t('notify.ttsd.restarting.message'),
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
        title: t('notify.ttsd.fatal.title'),
        message: p.message || t('notify.ttsd.fatal.fallback'),
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
          title: t('notify.model.loading_restart.title'),
          message: t('notify.model.loading_restart.message'),
          color: 'yellow',
          loading: true,
          autoClose: false,
        });
        return;
      }
      notifications.show({
        id: 'model-loading',
        title: t('notify.model.loading.title'),
        message: t('notify.model.loading.message'),
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
          title: t('notify.model.loaded_restart.title'),
          message: t('notify.model.loaded_restart.message'),
          color: 'green',
          loading: false,
          autoClose: 3000,
        });
        return;
      }
      notifications.update({
        id: 'model-loading',
        title: t('notify.model.loaded.title'),
        message: t('notify.model.loaded.message'),
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
          title: t('notify.model.error.title'),
          message: p.message,
          color: 'red',
          loading: false,
          autoClose: 8000,
        });
        return;
      }
      notifications.update({
        id: 'model-loading',
        title: t('notify.model.error.title'),
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
        title: t('notify.synthesis.error.title'),
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
          title: t('notify.synthesis.title'),
          message: truncate(original_text),
          loading: true,
          autoClose: false,
        });
      } else if (status === 'pending' && synthesisShown.has(id)) {
        // Cancelled: the entry is back in the queue, the spinner toast would
        // otherwise stay forever (autoClose is off).
        synthesisShown.delete(id);
        notifications.hide(toastId);
      } else if (status === 'ready' && synthesisShown.has(id)) {
        synthesisShown.delete(id);
        notifications.update({
          id: toastId,
          title: t('notify.synthesis.done.title'),
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
  const fmtMb = (bytes: number) =>
    `${(bytes / (1024 * 1024)).toFixed(1)} ${t('common.mb')}`;

  unlisteners.push(
    await events.voiceDownloadStarted((p) => {
      notifications.show({
        id: voiceToastId(p.voice),
        title: t('notify.voice.downloading.title', [p.voice]),
        message: t('notify.voice.requesting'),
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
      const file = t(p.file_kind === 'onnx' ? 'notify.voice.file.model' : 'notify.voice.file.config');
      const message = total > 0
        ? t('notify.voice.progress.tallied', [
            file,
            p.file_idx + 1,
            p.total_files,
            fmtMb(p.downloaded_bytes),
            fmtMb(total),
          ])
        : t('notify.voice.progress.plain', [file, fmtMb(p.downloaded_bytes)]);
      notifications.update({
        id: voiceToastId(p.voice),
        title: t('notify.voice.downloading.title', [p.voice]),
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
          title: t('notify.voice.done.title'),
          message: t('notify.voice.done.message', [p.voice]),
          color: 'green',
          loading: false,
          autoClose: 3000,
        });
      } else {
        notifications.update({
          id: voiceToastId(p.voice),
          title: t('notify.voice.failed.title'),
          message: p.message ?? t('notify.voice.failed.fallback', [p.voice]),
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
