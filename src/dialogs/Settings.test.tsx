// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import type { UIConfig, UIConfigPatch } from '../lib/tauri';
import { SettingsModal } from './Settings';

const { getConfig, getAvailableEngines, updateConfig, showNotification } = vi.hoisted(() => ({
  getConfig: vi.fn<() => Promise<UIConfig>>(),
  getAvailableEngines: vi.fn(),
  updateConfig: vi.fn<(patch: UIConfigPatch) => Promise<void>>().mockResolvedValue(undefined),
  showNotification: vi.fn(),
}));

vi.mock('@mantine/notifications', () => ({
  notifications: { show: showNotification },
}));

vi.mock('../lib/tauri', () => ({
  commands: {
    getConfig,
    getAvailableEngines,
    updateConfig,
    getCacheDir: vi.fn().mockResolvedValue(''),
    getLogDir: vi.fn().mockResolvedValue(''),
    downloadSileroNativeBundle: vi.fn(),
    downloadPiperVoice: vi.fn(),
    clearCache: vi.fn(),
  },
  events: {
    bundleDownloadStarted: () => Promise.resolve(() => {}),
    bundleDownloadProgress: () => Promise.resolve(() => {}),
    bundleDownloadFinished: () => Promise.resolve(() => {}),
  },
}));

vi.mock('../lib/updater', () => ({
  updaterSupported: vi.fn().mockResolvedValue(false),
  checkForUpdatesManual: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  revealItemInDir: vi.fn(),
}));

vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn().mockResolvedValue('0.0.0-test'),
}));

function makeConfig(overrides: Partial<UIConfig> = {}): UIConfig {
  return {
    speaker: 'aidar',
    sample_rate: 24000,
    speech_rate: 1.0,
    notify_on_ready: true,
    notify_on_error: true,
    text_format: 'plain',
    max_cache_size_mb: 500,
    code_block_mode: 'brief',
    theme: 'auto',
    language: 'ru',
    player_hotkeys: {},
    window_geometry: null,
    preview_dialog_enabled: true,
    engine: 'silero_native',
    piper_voice: 'ruslan',
    ...overrides,
  };
}

describe('SettingsModal code block narration', () => {
  let container: HTMLElement | null = null;
  let root: Root | null = null;

  beforeEach(() => {
    getConfig.mockResolvedValue(makeConfig());
    getAvailableEngines.mockResolvedValue({
      piper: { available: true, reason: null },
      silero: { available: false, reason: null },
      silero_native: { available: true, reason: null },
    });
  });

  afterEach(() => {
    act(() => root?.unmount());
    container?.remove();
    container = null;
    root = null;
    vi.clearAllMocks();
  });

  const renderDialog = async () => {
    const el = document.createElement('div');
    document.body.appendChild(el);
    container = el;
    const r = createRoot(el);
    root = r;
    await act(async () => {
      r.render(
        <MantineProvider>
          <SettingsModal opened onClose={() => {}} />
        </MantineProvider>,
      );
      await Promise.resolve();
    });
  };

  const clickSave = async () => {
    const save = [...document.querySelectorAll('button')].find((b) =>
      (b.textContent ?? '').includes('Сохранить'),
    );
    expect(save, 'save button rendered').toBeTruthy();
    await act(async () => {
      save!.click();
      await Promise.resolve();
    });
  };

  it('renders the saved mode as the selected segment', async () => {
    getConfig.mockResolvedValue(makeConfig({ code_block_mode: 'read' }));
    await renderDialog();

    const active = document.querySelector('[data-active]');
    expect(active?.textContent).toBe('Читать полностью');
  });

  it('submits the picked mode in the config patch', async () => {
    await renderDialog();

    const segment = document.querySelector<HTMLInputElement>('input[value="read"]');
    expect(segment, 'segment rendered').toBeTruthy();
    await act(async () => {
      segment!.click();
      await Promise.resolve();
    });
    await clickSave();

    const patch = updateConfig.mock.calls[0][0];
    expect(patch.code_block_mode).toBe('read');
  });

  it('keeps the saved mode in the patch when untouched', async () => {
    await renderDialog();
    await clickSave();

    const patch = updateConfig.mock.calls[0][0];
    expect(patch.code_block_mode).toBe('brief');
  });
});
