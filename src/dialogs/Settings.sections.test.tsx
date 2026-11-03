// @vitest-environment jsdom
//
// Section navigation, search, dirty markers and the footer of the reworked
// Settings dialog (#355). The control/patch behavior itself stays covered by
// Settings.test.tsx; this file owns the layout contract.

import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import type { UIConfig } from '../lib/tauri';
import { setLocale } from '../stores/locale';
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
  updateConfig: vi.fn().mockResolvedValue(undefined),
  showNotification: vi.fn(),
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
    getCacheDir: vi.fn().mockResolvedValue('/home/test/.local/share/ruvox'),
    getLogDir: vi.fn().mockResolvedValue('/home/test/.local/share/ruvox/logs'),
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
    getCacheStats: vi.fn().mockResolvedValue({ total_bytes: 0, audio_file_count: 0 }),
    getDictionaries: vi.fn().mockResolvedValue([{ entries: [{}, {}] }, { entries: [{}] }]),
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
  updaterSupported: vi.fn().mockResolvedValue(true),
  checkForUpdatesManual: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  revealItemInDir: vi.fn(),
}));

vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn().mockResolvedValue('0.6.0-test'),
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
    reading_width: 'narrow',
    reading_scale: 100,
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

function availability() {
  return {
    piper: { available: true, reason: null, reads_latin_natively: false },
    silero: { available: false, reason: null, reads_latin_natively: false },
    silero_native: { available: true, reason: null, reads_latin_natively: false },
    teratts: { available: true, reason: null, reads_latin_natively: true },
  };
}

describe('SettingsModal sections', () => {
  let container: HTMLElement | null = null;
  let root: Root | null = null;

  beforeEach(() => {
    // The language-switch test relabels the global store; keep tests isolated.
    setLocale('ru');
    getConfig.mockResolvedValue(makeConfig());
    getAvailableEngines.mockResolvedValue(availability());
    getTerattsVoices.mockResolvedValue([]);
    eventHandlers.bundleDownloadFinished = null;
  });

  afterEach(() => {
    act(() => root?.unmount());
    container?.remove();
    container = null;
    root = null;
    vi.clearAllMocks();
  });

  const renderDialog = async (
    options: { onClose?: () => void; onOpenDictionary?: () => void } = {},
  ) => {
    const el = document.createElement('div');
    document.body.appendChild(el);
    container = el;
    const r = createRoot(el);
    root = r;
    const onClose = options.onClose ?? (() => {});
    const onOpenDictionary = options.onOpenDictionary;

    const render = async (opened: boolean) => {
      await act(async () => {
        r.render(
          <MantineProvider>
            <SettingsModal
              opened={opened}
              onClose={onClose}
              onOpenDictionary={onOpenDictionary}
            />
          </MantineProvider>,
        );
        await Promise.resolve();
      });
    };

    await render(true);
    return { rerender: render };
  };

  /** Section buttons of the side nav, in document order. */
  const navTitles = () =>
    [...document.querySelectorAll<HTMLElement>('[data-settings-nav] [id^="settings-nav-"]')].map(
      (el) => el.textContent?.trim() ?? '',
    );

  const clickNav = async (section: string) => {
    const button = document.getElementById(`settings-nav-${section}`);
    expect(button, `section ${section} rendered`).toBeTruthy();
    await act(async () => {
      button!.click();
      await Promise.resolve();
    });
  };

  const rowFields = () =>
    [...document.querySelectorAll<HTMLElement>('[data-settings-field]')]
      .map((el) => el.getAttribute('data-settings-field'))
      .filter((field): field is string => field !== null);

  const sectionTitle = () =>
    document.querySelector<HTMLElement>('[data-settings-section-title]')?.textContent?.trim() ??
    null;

  const footerText = () =>
    document.querySelector<HTMLElement>('[data-settings-footer]')?.textContent ?? '';

  const dirtyDot = (section: string) =>
    document
      .getElementById(`settings-nav-${section}`)
      ?.querySelector('[data-settings-dirty-dot]') ?? null;

  const searchInput = () =>
    document.querySelector<HTMLInputElement>('[data-settings-search] input');

  const typeSearch = async (value: string) => {
    const input = searchInput();
    expect(input, 'search input rendered').toBeTruthy();
    const setValue = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype,
      'value',
    )!.set!;
    await act(async () => {
      setValue.call(input, value);
      input!.dispatchEvent(new Event('input', { bubbles: true }));
      await Promise.resolve();
    });
  };

  const pressKey = async (element: Element | null, key: string) => {
    expect(element, `element for ${key}`).toBeTruthy();
    await act(async () => {
      element!.dispatchEvent(
        new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }),
      );
      await Promise.resolve();
    });
  };

  const findButton = (text: string) =>
    [...document.querySelectorAll<HTMLButtonElement>('button')].find(
      (button) => button.textContent === text,
    );

  const clickButton = async (text: string) => {
    const button = findButton(text);
    expect(button, `button "${text}" rendered`).toBeTruthy();
    await act(async () => {
      button!.click();
      await Promise.resolve();
    });
  };

  const findSwitch = (text: string) =>
    [...document.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')].find((input) =>
      input.closest('label')?.textContent?.includes(text),
    );

  const clickSwitch = async (text: string) => {
    const input = findSwitch(text);
    expect(input, `switch "${text}" rendered`).toBeTruthy();
    await act(async () => {
      input!.click();
      await Promise.resolve();
    });
  };

  const clickSave = async () => {
    const save = [...document.querySelectorAll<HTMLButtonElement>('button')].find((button) =>
      (button.textContent ?? '').includes('Сохранить'),
    );
    expect(save, 'save button rendered').toBeTruthy();
    await act(async () => {
      save!.click();
      await Promise.resolve();
    });
  };

  /**
   * Mantine's modal transition mounts the body on the next animation frame;
   * jsdom only fires it through a real timer tick.
   */
  const waitForModalTransition = async () => {
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 350));
    });
  };

  it('renders every settings row in exactly one section', async () => {
    await renderDialog();

    const expected: Record<string, string[]> = {
      engine: ['models', 'engine', 'voice', 'sample_rate'],
      reading: [
        'code_block_mode',
        'english_reading',
        'preview_dialog_enabled',
        'reading_width',
        'reading_scale',
      ],
      notifications: ['notify_on_ready', 'notify_on_error'],
      storage: ['max_cache_size_mb', 'data_folder', 'cleanup'],
      interface: ['hide_to_tray_on_close', 'theme', 'language'],
      dictionary: ['dictionary_entries', 'dictionary_open'],
      about: ['version', 'updates', 'logs_folder'],
    };

    const seen: string[] = [];
    for (const [section, fields] of Object.entries(expected)) {
      await clickNav(section);
      const rendered = rowFields();
      expect(rendered, `rows of ${section}`).toEqual(fields);
      seen.push(...rendered);
    }

    // No control is reachable from two sections, none is missing.
    expect(new Set(seen).size).toBe(seen.length);
    expect(seen).toHaveLength(22);
  });

  it('submits the same config patch for the same edits', async () => {
    await renderDialog();

    await clickNav('interface');
    await clickSwitch('Сворачивать в трей');
    await clickNav('notifications');
    await clickSwitch('Уведомлять о готовности');
    await clickSave();

    expect(updateConfig).toHaveBeenCalledTimes(1);
    expect(updateConfig.mock.calls[0][0]).toEqual({
      piper_voice: 'ruslan',
      teratts_voice: 'ru_m5',
      speaker: 'aidar',
      sample_rate: 24000,
      notify_on_ready: false,
      notify_on_error: true,
      preview_dialog_enabled: true,
      hide_to_tray_on_close: true,
      max_cache_size_mb: 500,
      code_block_mode: 'brief',
      english_reading: 'auto',
      reading_width: 'narrow',
      reading_scale: 100,
      theme: 'auto',
      language: 'ru',
      engine: 'silero_native',
    });
  });

  it('filters rows by Russian and English labels from either catalog', async () => {
    await renderDialog();

    await typeSearch('англ');
    expect(navTitles()).toEqual(['Чтение текста']);
    expect(rowFields()).toEqual(['english_reading']);
    expect(
      document.querySelector('[data-settings-field="english_reading"] mark')?.textContent,
    ).toBe('Англ');

    // An English term typed in the Russian UI finds the row via the EN catalog.
    await typeSearch('theme');
    expect(navTitles()).toEqual(['Интерфейс']);
    expect(rowFields()).toEqual(['theme']);

    // Clearing the query restores the whole nav.
    await typeSearch('');
    expect(navTitles()).toHaveLength(7);
  });

  it('shows the no-results state when nothing matches', async () => {
    await renderDialog();

    await typeSearch('zzzz');
    expect(navTitles()).toHaveLength(0);
    expect(rowFields()).toHaveLength(0);
    expect(document.querySelector('[data-settings-no-results]')).toBeTruthy();
  });

  it('jumps to the first match on Enter without submitting the form', async () => {
    const scrolled: Element[] = [];
    const scrollSpy = vi
      .spyOn(Element.prototype, 'scrollIntoView')
      .mockImplementation(function (this: Element) {
        scrolled.push(this);
      });
    await renderDialog();

    await typeSearch('логи');
    expect(navTitles()).toEqual(['О программе']);

    await pressKey(searchInput(), 'Enter');
    expect(updateConfig).not.toHaveBeenCalled();
    expect(sectionTitle()).toBe('О программе');
    expect(scrolled.map((element) => element.getAttribute('data-settings-field'))).toContain(
      'logs_folder',
    );
    scrollSpy.mockRestore();
  });

  it('clears the query on Esc first and closes the dialog afterwards', async () => {
    const onClose = vi.fn();
    await renderDialog({ onClose });

    await typeSearch('кэш');
    await pressKey(searchInput(), 'Escape');
    expect(onClose).not.toHaveBeenCalled();
    expect(searchInput()?.value).toBe('');
    expect(navTitles()).toHaveLength(7);

    await pressKey(searchInput(), 'Escape');
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('relabels sections, footer and search on a language switch', async () => {
    await renderDialog();
    await clickNav('interface');

    const languageInput = document.querySelector<HTMLInputElement>('input[value="Русский"]');
    expect(languageInput, 'language selector rendered').toBeTruthy();
    await act(async () => {
      languageInput!.click();
      await Promise.resolve();
    });
    const englishOption = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find(
      (option) => option.textContent === 'English',
    );
    expect(englishOption, 'English option rendered').toBeTruthy();
    await act(async () => {
      englishOption!.click();
      await Promise.resolve();
    });

    expect(navTitles()).toEqual([
      'Engine and voice',
      'Text reading',
      'Notifications',
      'Storage',
      'Interface',
      'Dictionary',
      'About',
    ]);
    // The language pick itself is an unsaved change; the count is localized.
    expect(footerText()).toContain('Changed: 1');
    expect(searchInput()?.placeholder).toBe('Search settings');

    // The catalog is indexed in both languages: a Russian query still works.
    await typeSearch('тема');
    expect(navTitles()).toEqual(['Interface']);
    expect(rowFields()).toEqual(['theme']);
  });

  it('marks sections dirty and counts changed fields in the footer', async () => {
    await renderDialog();
    expect(footerText()).toContain('Изменено: 0');

    await clickNav('notifications');
    await clickSwitch('Уведомлять о готовности');
    expect(dirtyDot('notifications')).toBeTruthy();
    expect(dirtyDot('interface')).toBeNull();
    expect(footerText()).toContain('Изменено: 1');

    await clickNav('interface');
    const themeInput = document.querySelector<HTMLInputElement>('input[value="Авто"]');
    expect(themeInput, 'theme selector rendered').toBeTruthy();
    await act(async () => {
      themeInput!.click();
      await Promise.resolve();
    });
    const darkOption = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find(
      (option) => option.textContent === 'Тёмная',
    );
    expect(darkOption, 'dark theme option rendered').toBeTruthy();
    await act(async () => {
      darkOption!.click();
      await Promise.resolve();
    });

    expect(dirtyDot('interface')).toBeTruthy();
    expect(footerText()).toContain('Изменено: 2');
  });

  it('does not mark the engine section dirty when the saved engine was coerced', async () => {
    getConfig.mockResolvedValue(makeConfig({ engine: 'silero' }));
    getAvailableEngines.mockResolvedValue({
      ...availability(),
      silero: {
        available: false,
        reason: { code: 'errors.ttsd.python_missing' },
        reads_latin_natively: false,
      },
    });
    await renderDialog();

    expect(document.body.textContent ?? '').toContain('Сохранённый движок недоступен');
    expect(dirtyDot('engine')).toBeNull();
    expect(footerText()).toContain('Изменено: 0');
  });

  it('reset restores factory defaults and clears the dirty markers', async () => {
    await renderDialog();
    await clickNav('interface');
    const themeInput = document.querySelector<HTMLInputElement>('input[value="Авто"]');
    await act(async () => {
      themeInput!.click();
      await Promise.resolve();
    });
    const darkOption = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find(
      (option) => option.textContent === 'Тёмная',
    );
    await act(async () => {
      darkOption!.click();
      await Promise.resolve();
    });
    expect(footerText()).toContain('Изменено: 1');

    await clickButton('Сбросить');
    expect(footerText()).toContain('Изменено: 0');
    expect(document.querySelector<HTMLInputElement>('input[value="Авто"]')).toBeTruthy();
  });

  it('returns to the storage section after the nested cleanup dialog closes', async () => {
    await renderDialog();
    await clickNav('storage');
    await clickButton('Очистить кэш…');
    await waitForModalTransition();

    expect(document.body.textContent ?? '').toContain('Очистить полностью');
    await clickButton('Отмена');
    expect(sectionTitle()).toBe('Хранилище');
    expect(rowFields()).toContain('cleanup');
  });

  it('restores the dictionary section after the editor round-trip', async () => {
    const onOpenDictionary = vi.fn();
    const dialog = await renderDialog({ onOpenDictionary });

    await clickNav('dictionary');
    expect(sectionTitle()).toBe('Словарь');
    await clickButton('Открыть…');
    expect(onOpenDictionary).toHaveBeenCalledTimes(1);

    // AppShell hides Settings for the editing session and brings it back after.
    await dialog.rerender(false);
    await dialog.rerender(true);
    expect(sectionTitle()).toBe('Словарь');
    expect(rowFields()).toEqual(['dictionary_entries', 'dictionary_open']);
  });

  it('keeps the footer outside the scrolling content pane', async () => {
    await renderDialog();

    const footer = document.querySelector<HTMLElement>('[data-settings-footer]');
    expect(footer).toBeTruthy();
    expect(footer!.closest('[data-settings-scroll]')).toBeNull();
    expect(document.querySelector('[data-settings-scroll]')).toBeTruthy();
  });

  it('collapses the nav into a select below 700px', async () => {
    const originalMatchMedia = window.matchMedia;
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      configurable: true,
      value: (query: string) => ({
        matches: query.includes('max-width: 700px'),
        media: query,
        onchange: null,
        addListener: () => {},
        removeListener: () => {},
        addEventListener: () => {},
        removeEventListener: () => {},
        dispatchEvent: () => false,
      }),
    });

    try {
      await renderDialog();
      expect(document.querySelector('[data-settings-nav-select]')).toBeTruthy();
      expect(document.querySelector('[data-settings-nav]')).toBeNull();

      const select = document.querySelector<HTMLInputElement>(
        '[data-settings-nav-select] input',
      );
      expect(select, 'narrow nav select rendered').toBeTruthy();
      await act(async () => {
        select!.click();
        await Promise.resolve();
      });
      const option = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find(
        (candidate) => candidate.textContent === 'Уведомления',
      );
      expect(option, 'section option rendered').toBeTruthy();
      await act(async () => {
        option!.click();
        await Promise.resolve();
      });
      expect(sectionTitle()).toBe('Уведомления');
    } finally {
      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        configurable: true,
        value: originalMatchMedia,
      });
    }
  });

  it('shows the side nav above 700px', async () => {
    await renderDialog();
    expect(document.querySelector('[data-settings-nav]')).toBeTruthy();
    expect(document.querySelector('[data-settings-nav-select]')).toBeNull();
  });
});
