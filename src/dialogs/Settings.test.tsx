// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import type { UIConfig, UIConfigPatch } from '../lib/tauri';
import { SettingsModal } from './Settings';

const {
  getConfig,
  getAvailableEngines,
  getTerattsVoices,
  updateConfig,
  showNotification,
  eventHandlers,
} = vi.hoisted(() => ({
  getConfig: vi.fn<() => Promise<UIConfig>>(),
  getAvailableEngines: vi.fn(),
  getTerattsVoices: vi.fn<() => Promise<string[]>>(),
  updateConfig: vi.fn<(patch: UIConfigPatch) => Promise<void>>().mockResolvedValue(undefined),
  showNotification: vi.fn(),
  /** Handlers the model inventory subscribes to, so a test can emit one. */
  eventHandlers: {
    bundleDownloadFinished: null as null | ((payload: { engine: string; ok: boolean }) => void),
  },
}));

vi.mock('@mantine/notifications', () => ({
  notifications: { show: showNotification },
}));

vi.mock('../lib/tauri', () => ({
  commands: {
    getConfig,
    getAvailableEngines,
    getTerattsVoices,
    updateConfig,
    getCacheDir: vi.fn().mockResolvedValue(''),
    getLogDir: vi.fn().mockResolvedValue(''),
    downloadSileroNativeBundle: vi.fn(),
    downloadPiperVoice: vi.fn(),
    getModelStatus: vi.fn().mockResolvedValue({
      assets: [],
      total_bytes: 0,
      channel_error: null,
      checked_at: null,
    }),
    installModelAsset: vi.fn().mockResolvedValue(undefined),
    deleteModelAsset: vi.fn().mockResolvedValue(0),
    skipModelUpdate: vi.fn().mockResolvedValue(undefined),
    clearCache: vi.fn(),
    getDictionaries: vi.fn().mockResolvedValue([]),
    saveDictionary: vi.fn().mockResolvedValue(undefined),
    importDictionary: vi.fn(),
    exportDictionary: vi.fn().mockResolvedValue(undefined),
    pickDictionaryImportPath: vi.fn().mockResolvedValue(null),
    pickDictionaryExportPath: vi.fn().mockResolvedValue(null),
  },
  events: {
    bundleDownloadStarted: () => Promise.resolve(() => {}),
    bundleDownloadProgress: () => Promise.resolve(() => {}),
    bundleDownloadFinished: (handler: (payload: { engine: string; ok: boolean }) => void) => {
      eventHandlers.bundleDownloadFinished = handler;
      return Promise.resolve(() => {});
    },
    voiceDownloadStarted: () => Promise.resolve(() => {}),
    voiceDownloadProgress: () => Promise.resolve(() => {}),
    voiceDownloadFinished: () => Promise.resolve(() => {}),
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
    english_reading: 'auto',
    theme: 'auto',
    language: 'ru',
    player_hotkeys: {},
    window_geometry: null,
    preview_dialog_enabled: true,
    hide_to_tray_on_close: false,
    engine: 'silero_native',
    piper_voice: 'ruslan',
    teratts_voice: 'ru_m5',
    ...overrides,
  };
}

/** Availability map with TeraTTS selectable, for the voice-selector tests. */
function terattsAvailable() {
  return {
    piper: { available: true, reason: null, reads_latin_natively: false },
    silero: { available: false, reason: null, reads_latin_natively: false },
    silero_native: { available: true, reason: null, reads_latin_natively: false },
    teratts: { available: true, reason: null, reads_latin_natively: true },
  };
}

describe('SettingsModal code block narration', () => {
  let container: HTMLElement | null = null;
  let root: Root | null = null;

  beforeEach(() => {
    getConfig.mockResolvedValue(makeConfig());
    getTerattsVoices.mockResolvedValue([]);
    eventHandlers.bundleDownloadFinished = null;
    getAvailableEngines.mockResolvedValue({
      piper: { available: true, reason: null, reads_latin_natively: false },
      silero: { available: false, reason: null, reads_latin_natively: false },
      silero_native: { available: true, reason: null, reads_latin_natively: false },
      teratts: { available: false, reason: { code: 'teratts.bundle_missing' }, reads_latin_natively: true },
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

  /**
   * Open the TeraTTS voice dropdown (its input mirrors the selected id) and
   * read the options of *that* listbox — closed selects also keep role=option
   * nodes in the document.
   */
  const openVoiceOptions = async (selected: string): Promise<(string | null)[]> => {
    const select = document.querySelector<HTMLInputElement>(`input[value="${selected}"]`);
    expect(select, 'TeraTTS voice selector rendered').toBeTruthy();
    await act(async () => {
      select!.click();
      await Promise.resolve();
    });
    const listId = select!.getAttribute('aria-controls');
    expect(listId, 'voice dropdown opened').toBeTruthy();
    const listbox = document.getElementById(listId!);
    return [...(listbox?.querySelectorAll('[role="option"]') ?? [])].map((o) => o.textContent);
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

  const activeEnglishSegment = () => {
    for (const value of ['auto', 'native', 'narrated']) {
      const input = document.querySelector<HTMLInputElement>(
        `input[name="english_reading"][value="${value}"]`,
      );
      const label = input?.parentElement?.querySelector('label');
      if (label?.hasAttribute('data-active')) return value;
    }
    return null;
  };

  it('renders the saved English reading as the selected segment', async () => {
    getConfig.mockResolvedValue(makeConfig({ english_reading: 'narrated' }));
    await renderDialog();

    expect(activeEnglishSegment()).toBe('narrated');
  });

  it('submits the picked English reading in the config patch', async () => {
    await renderDialog();

    const segment = document.querySelector<HTMLInputElement>(
      'input[name="english_reading"][value="native"]',
    );
    expect(segment, 'English reading segment rendered').toBeTruthy();
    await act(async () => {
      segment!.click();
      await Promise.resolve();
    });
    await clickSave();

    expect(updateConfig.mock.calls[0][0].english_reading).toBe('native');
  });

  it('keeps the saved English reading in the patch when untouched', async () => {
    await renderDialog();
    await clickSave();

    expect(updateConfig.mock.calls[0][0].english_reading).toBe('auto');
  });

  it('explains the fallback when the engine cannot read English natively', async () => {
    // The default fixture engine (silero_native) has no native Latin reading.
    getConfig.mockResolvedValue(makeConfig({ english_reading: 'native' }));
    await renderDialog();

    expect(document.body.textContent ?? '').toContain(
      'Этот движок не читает английский нативно',
    );
    // The preference is engine-independent: it stays selectable and saved.
    await clickSave();
    expect(updateConfig.mock.calls[0][0].english_reading).toBe('native');
  });

  it('shows no fallback hint for an engine that reads English natively', async () => {
    getConfig.mockResolvedValue(
      makeConfig({ engine: 'teratts', english_reading: 'native' }),
    );
    getAvailableEngines.mockResolvedValue(terattsAvailable());
    await renderDialog();

    expect(document.body.textContent ?? '').not.toContain(
      'Этот движок не читает английский нативно',
    );
  });

  it('submits hide_to_tray_on_close when the toggle is enabled', async () => {
    await renderDialog();

    const traySwitch = [
      ...document.querySelectorAll<HTMLInputElement>('input[type="checkbox"]'),
    ].find((input) =>
      input.closest('label')?.textContent?.includes('Сворачивать в трей'),
    );
    expect(traySwitch, 'tray-on-close switch rendered').toBeTruthy();
    // Factory default is off: closing the window quits.
    expect(traySwitch!.checked).toBe(false);

    await act(async () => {
      traySwitch!.click();
      await Promise.resolve();
    });
    await clickSave();

    expect(updateConfig.mock.calls[0][0].hide_to_tray_on_close).toBe(true);
  });

  it('does not show the Silero voice selector for TeraTTS', async () => {
    getConfig.mockResolvedValue(makeConfig({ engine: 'teratts' }));
    getAvailableEngines.mockResolvedValue(terattsAvailable());
    await renderDialog();

    const text = document.body.textContent ?? '';
    expect(text).toContain('Голос TeraTTSv2');
    expect(text).not.toContain('Голос Silero');
  });

  it('offers every voice the bundle reports and preselects the saved one', async () => {
    const voices = [
      'ru_m5',
      'ru_f1',
      'ru_f2',
      'ru_m1',
      'eng_f3',
      'eng_f4_whisper',
      'eng_f5',
      'eng_m2_whisper',
      'eng_m3',
      'eng_m4',
    ];
    getTerattsVoices.mockResolvedValue(voices);
    getConfig.mockResolvedValue(makeConfig({ engine: 'teratts', teratts_voice: 'eng_f5' }));
    getAvailableEngines.mockResolvedValue(terattsAvailable());
    await renderDialog();

    expect(await openVoiceOptions('eng_f5')).toEqual(voices);
  });

  it('keeps a saved voice the catalogue does not list and saves it unchanged', async () => {
    getTerattsVoices.mockResolvedValue(['ru_m5', 'ru_f1']);
    getConfig.mockResolvedValue(makeConfig({ engine: 'teratts', teratts_voice: 'eng_f5' }));
    getAvailableEngines.mockResolvedValue(terattsAvailable());
    await renderDialog();

    expect(await openVoiceOptions('eng_f5')).toEqual(['ru_m5', 'ru_f1', 'eng_f5']);

    await clickSave();
    expect(updateConfig.mock.calls[0][0].teratts_voice).toBe('eng_f5');
  });

  it('refreshes the voice catalogue when a bundle install finishes', async () => {
    // First install from Settings: the bundle was absent when the dialog
    // opened, so only this refresh makes the new bundle's voices selectable
    // without reopening Settings.
    await renderDialog();
    getTerattsVoices.mockClear();

    const openModels = [...document.querySelectorAll('button')].find(
      (b) => b.textContent === 'Открыть',
    );
    expect(openModels, 'model inventory button rendered').toBeTruthy();
    await act(async () => {
      openModels!.click();
      await Promise.resolve();
    });
    expect(eventHandlers.bundleDownloadFinished, 'inventory subscribed to bundle events').toBeTruthy();

    await act(async () => {
      eventHandlers.bundleDownloadFinished!({ engine: 'teratts', ok: true });
      await Promise.resolve();
    });

    expect(getTerattsVoices).toHaveBeenCalledTimes(1);
  });

  it('shows the stored tray setting when the save is rejected', async () => {
    // The backend rolls the setting back when the tray icon cannot be
    // created: the switch must show the stored (off) state, not the rejected
    // one.
    updateConfig.mockRejectedValueOnce({
      type: 'config_error',
      code: 'config.tray_unavailable',
    });
    await renderDialog();

    const traySwitch = [
      ...document.querySelectorAll<HTMLInputElement>('input[type="checkbox"]'),
    ].find((input) =>
      input.closest('label')?.textContent?.includes('Сворачивать в трей'),
    );
    expect(traySwitch, 'tray-on-close switch rendered').toBeTruthy();

    await act(async () => {
      traySwitch!.click();
      await Promise.resolve();
    });
    await clickSave();

    await act(async () => {
      await Promise.resolve();
    });
    expect(traySwitch!.checked).toBe(false);
  });
});
