// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mantine notifications/modals and the Tauri updater/process plugins are
// mocked wholesale: the tests pin our orchestration (which toast when,
// show-before-update, silent startup), not the plugins themselves.
vi.mock('@mantine/notifications', () => ({
  notifications: {
    show: vi.fn(),
    update: vi.fn(),
    hide: vi.fn(),
  },
}));
vi.mock('@mantine/modals', () => ({
  modals: { openConfirmModal: vi.fn() },
}));
vi.mock('@tauri-apps/plugin-updater', () => ({
  check: vi.fn(),
}));
vi.mock('@tauri-apps/plugin-process', () => ({
  relaunch: vi.fn(),
}));
vi.mock('@tauri-apps/plugin-log', () => ({
  info: vi.fn(),
  error: vi.fn(),
}));
vi.mock('./tauri', () => ({
  commands: {
    shutdownPlayerForUpdate: vi.fn().mockResolvedValue(undefined),
    updaterSupported: vi.fn().mockResolvedValue(true),
  },
}));

import { notifications } from '@mantine/notifications';
import { modals } from '@mantine/modals';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { error as logError, info as logInfo } from '@tauri-apps/plugin-log';
import { commands } from './tauri';
import {
  checkForUpdatesManual,
  checkForUpdatesOnStartup,
  updaterSupported,
} from './updater';

const checkMock = vi.mocked(check);
const showMock = vi.mocked(notifications.show);
const updateMock = vi.mocked(notifications.update);
const modalMock = vi.mocked(modals.openConfirmModal);
const relaunchMock = vi.mocked(relaunch);
const logInfoMock = vi.mocked(logInfo);
const logErrorMock = vi.mocked(logError);
const supportedMock = vi.mocked(commands.updaterSupported);

function fakeUpdate() {
  return {
    version: '9.9.9',
    downloadAndInstall: vi.fn().mockResolvedValue(undefined),
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe('checkForUpdatesOnStartup', () => {
  it('is a no-op when the install cannot self-update (.deb/nix)', async () => {
    supportedMock.mockResolvedValue(false);
    expect(await updaterSupported()).toBe(false);
    await checkForUpdatesOnStartup();
    expect(checkMock).not.toHaveBeenCalled();
  });

  it('checks and logs when the install is served (Windows / Linux AppImage)', async () => {
    supportedMock.mockResolvedValue(true);
    checkMock.mockResolvedValue(null);
    await checkForUpdatesOnStartup();
    expect(checkMock).toHaveBeenCalled();
    expect(logInfoMock).toHaveBeenCalledWith('update check (startup): up to date');
  });
});

describe('checkForUpdatesManual', () => {
  it('reports "no updates" when check returns null', async () => {
    checkMock.mockResolvedValue(null);
    await checkForUpdatesManual();
    expect(showMock).toHaveBeenCalledWith(
      expect.objectContaining({ title: 'Обновлений нет', color: 'green' }),
    );
    expect(modalMock).not.toHaveBeenCalled();
    expect(logInfoMock).toHaveBeenCalledWith('update check (manual): up to date');
  });

  it('opens the confirm modal when an update is available', async () => {
    checkMock.mockResolvedValue(fakeUpdate() as never);
    await checkForUpdatesManual();
    expect(modalMock).toHaveBeenCalledWith(
      expect.objectContaining({ title: 'Доступна новая версия 9.9.9' }),
    );
    expect(logInfoMock).toHaveBeenCalledWith('update check (manual): update available: 9.9.9');
  });

  it('shows a red toast when the check fails (manual = not silent)', async () => {
    checkMock.mockRejectedValue(new Error('network down'));
    await checkForUpdatesManual();
    expect(showMock).toHaveBeenCalledWith(
      expect.objectContaining({ title: 'Не удалось проверить обновления', color: 'red' }),
    );
    expect(logErrorMock).toHaveBeenCalledWith(
      expect.stringContaining('update check (manual) failed'),
    );
  });
});

describe('install flow (confirm → downloadAndInstall → relaunch)', () => {
  it('shows the progress toast with `show` before any `update`', async () => {
    const update = fakeUpdate();
    checkMock.mockResolvedValue(update as never);
    await checkForUpdatesManual();

    // Simulate the user pressing «Обновить и перезапустить».
    const onConfirm = modalMock.mock.calls[0][0].onConfirm as () => void;
    onConfirm();
    await vi.waitFor(() => expect(relaunchMock).toHaveBeenCalled());

    // Regression pin: Mantine drops `update` for unknown ids, so the first
    // progress call must be `show` with the stable toast id.
    expect(showMock).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'app-update', loading: true }),
    );
    // #211: mpv is destroyed before the installer starts so the orphaned
    // process cannot lock the install dir.
    expect(commands.shutdownPlayerForUpdate).toHaveBeenCalled();
    expect(update.downloadAndInstall).toHaveBeenCalled();
  });

  it('reports install failure via the same toast id', async () => {
    const update = fakeUpdate();
    update.downloadAndInstall.mockRejectedValue(new Error('broken pipe'));
    checkMock.mockResolvedValue(update as never);
    await checkForUpdatesManual();

    const onConfirm = modalMock.mock.calls[0][0].onConfirm as () => void;
    onConfirm();
    await vi.waitFor(() =>
      expect(updateMock).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'app-update',
          title: 'Не удалось установить обновление',
          color: 'red',
        }),
      ),
    );
    expect(relaunchMock).not.toHaveBeenCalled();
  });
});
