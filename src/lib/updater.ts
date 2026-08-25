import { modals } from '@mantine/modals';
import { notifications } from '@mantine/notifications';
import { error as logError, info as logInfo } from '@tauri-apps/plugin-log';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { formatError } from './errors';
import { t } from './i18n';
import { commands } from './tauri';

/**
 * Auto-update front end (tauri-plugin-updater).
 *
 * Only meaningful on Windows, where the NSIS installer is the distribution
 * channel — on Linux the app ships via nix and `latest.json` carries no
 * linux platform entry. `navigator.userAgent` is the zero-dependency way to
 * gate this (no plugin-os just for one check); the webview UA contains
 * "Windows" on Windows.
 */
export const UPDATER_ENABLED = navigator.userAgent.includes('Windows');

const UPDATE_TOAST_ID = 'app-update';

/** Download + install + relaunch, with progress reflected in a toast. */
async function installAndRelaunch(update: Update) {
  // NOTE: Mantine's notifications.update is a no-op for an id that was
  // never shown — the FIRST call must be `show`, updates follow.
  notifications.show({
    id: UPDATE_TOAST_ID,
    title: t('notify.update.downloading.title'),
    message: t('notify.update.preparing'),
    loading: true,
    autoClose: false,
  });
  let downloaded = 0;
  // #211: the updater-launched installer force-kills the app, so the
  // exit-time mpv cleanup never runs and the orphaned mpv.exe would lock
  // $INSTDIR — destroy it up front, before the download even starts.
  await commands.shutdownPlayerForUpdate();
  await update.downloadAndInstall((event) => {
    if (event.event === 'Started' && event.data.contentLength) {
      notifications.update({
        id: UPDATE_TOAST_ID,
        title: t('notify.update.downloading.title'),
        message: t('notify.update.of_total', [
          (event.data.contentLength / (1024 * 1024)).toFixed(0),
        ]),
        loading: true,
        autoClose: false,
      });
    } else if (event.event === 'Progress') {
      downloaded += event.data.chunkLength;
      notifications.update({
        id: UPDATE_TOAST_ID,
        title: t('notify.update.downloading.title'),
        message: t('notify.update.mb', [(downloaded / (1024 * 1024)).toFixed(0)]),
        loading: true,
        autoClose: false,
      });
    } else if (event.event === 'Finished') {
      notifications.update({
        id: UPDATE_TOAST_ID,
        title: t('notify.update.installing.title'),
        message: t('notify.update.installing.message'),
        loading: true,
        autoClose: false,
      });
    }
  });
  await relaunch();
}

/** Shared prompt once an update is known to be available. */
function promptInstall(update: Update) {
  modals.openConfirmModal({
    title: t('notify.update.available.title', [update.version]),
    children: t('notify.update.prompt'),
    labels: { confirm: t('notify.update.confirm'), cancel: t('notify.update.later') },
    confirmProps: { color: 'blue' },
    onConfirm: () => {
      installAndRelaunch(update).catch((err) => {
        notifications.update({
          id: UPDATE_TOAST_ID,
          title: t('notify.update.install_failed.title'),
          message: formatError(err),
          color: 'red',
          loading: false,
          autoClose: 8000,
        });
      });
    },
  });
}

/** Startup check: silent on any failure (offline, no release yet, Linux). */
export async function checkForUpdatesOnStartup() {
  if (!UPDATER_ENABLED) return;
  try {
    const update = await check();
    if (update) {
      await logInfo(`update check (startup): update available: ${update.version}`);
      promptInstall(update);
    } else {
      await logInfo('update check (startup): up to date');
    }
  } catch (err) {
    // Offline / endpoint missing / draft-only releases — stay silent in the UI,
    // but keep the reason in the log file for diagnostics.
    await logError(`update check (startup) failed: ${formatError(err)}`);
  }
}

/** Manual check from Settings: reports every outcome to the user. */
export async function checkForUpdatesManual() {
  try {
    const update = await check();
    if (update) {
      await logInfo(`update check (manual): update available: ${update.version}`);
      promptInstall(update);
    } else {
      await logInfo('update check (manual): up to date');
      notifications.show({
        title: t('notify.update.up_to_date.title'),
        message: t('notify.update.up_to_date.message'),
        color: 'green',
      });
    }
  } catch (err) {
    await logError(`update check (manual) failed: ${formatError(err)}`);
    notifications.show({
      title: t('notify.update.check_failed.title'),
      message: formatError(err),
      color: 'red',
    });
  }
}
