// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import type { TextEntry } from '../lib/tauri';

const {
  cancelSynthesis,
  getEntries,
  showNotification,
  pickExportAudioPath,
  exportAudio,
  regenerateEntry,
  entryUpdatedListeners,
} = vi.hoisted(() => ({
  cancelSynthesis: vi.fn().mockResolvedValue(undefined),
  getEntries: vi.fn(),
  showNotification: vi.fn(),
  pickExportAudioPath: vi.fn().mockResolvedValue(null),
  exportAudio: vi.fn().mockResolvedValue(undefined),
  regenerateEntry: vi.fn().mockResolvedValue(undefined),
  // Listeners QueueList registers on mount; tests deliver `entry_updated`
  // payloads through them to drive transitions (cleared busy retry).
  entryUpdatedListeners: [] as ((payload: { entry: TextEntry }) => void)[],
}));

vi.mock('@mantine/notifications', () => ({
  notifications: { show: showNotification },
}));

vi.mock('../lib/tauri', () => ({
  commands: {
    getEntries,
    cancelSynthesis,
    playEntry: vi.fn().mockResolvedValue(undefined),
    regenerateEntry,
    deleteEntry: vi.fn().mockResolvedValue(undefined),
    pickExportAudioPath,
    exportAudio,
  },
  events: {
    entryUpdated: vi.fn(
      (cb: (payload: { entry: TextEntry }) => void) => {
        entryUpdatedListeners.push(cb);
        return Promise.resolve(() => {});
      },
    ),
    entryRemoved: vi.fn().mockResolvedValue(() => {}),
    playbackStarted: vi.fn().mockResolvedValue(() => {}),
    playbackStopped: vi.fn().mockResolvedValue(() => {}),
    playbackFinished: vi.fn().mockResolvedValue(() => {}),
  },
}));

import { QueueList } from './QueueList';

function makeEntry(
  status: TextEntry['status'],
  overrides: Partial<TextEntry> = {},
): TextEntry {
  return {
    id: 'entry-1',
    original_text: 'тестовый текст',
    normalized_text: null,
    status,
    format: 'plain',
    html_source: null,
    source: null,
    created_at: '2026-07-27T00:00:00Z',
    audio_generated_at: null,
    audio_path: null,
    timestamps_path: null,
    duration_sec: null,
    was_regenerated: false,
    generation_count: 0,
    generation: null,
    error_message: null,
    ...overrides,
  };
}

describe('QueueList context menu', () => {
  let host: HTMLDivElement;
  let root: Root;
  const onRegenerate = vi.fn();

  beforeEach(() => {
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    cancelSynthesis.mockClear();
    showNotification.mockClear();
    pickExportAudioPath.mockClear();
    pickExportAudioPath.mockResolvedValue(null);
    exportAudio.mockClear();
    exportAudio.mockResolvedValue(undefined);
    onRegenerate.mockClear();
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
  });

  async function renderWith(entry: TextEntry): Promise<void> {
    getEntries.mockResolvedValue([entry]);
    await act(async () => {
      root.render(
        <MantineProvider>
          <QueueList onRegenerate={onRegenerate} />
        </MantineProvider>,
      );
      // Let the getEntries effect resolve and re-render the list.
      await Promise.resolve();
    });
  }

  async function openMenu(): Promise<void> {
    const item = host.querySelector('[data-entry-id="entry-1"]');
    expect(item).not.toBeNull();
    act(() => {
      item!.dispatchEvent(
        new MouseEvent('contextmenu', {
          bubbles: true,
          cancelable: true,
          clientX: 10,
          clientY: 10,
        }),
      );
    });
    // Mantine's Transition mounts the dropdown after two rAF frames and the
    // transition duration — poll instead of sleeping a fixed delay.
    await vi.waitFor(() => {
      expect(
        document.querySelectorAll('[role="menuitem"]').length,
      ).toBeGreaterThan(0);
    });
  }

  function cancelItem(): HTMLElement {
    const el = Array.from(
      document.querySelectorAll<HTMLElement>('[role="menuitem"]'),
    ).find((e) => e.textContent === 'Отменить синтез');
    expect(el).toBeDefined();
    return el!;
  }

  function regenerateItem(): HTMLElement {
    const el = Array.from(
      document.querySelectorAll<HTMLElement>('[role="menuitem"]'),
    ).find((e) => e.textContent === 'Перегенерировать аудио');
    expect(el).toBeDefined();
    return el!;
  }

  it('clicking "Перегенерировать аудио" hands the entry to onRegenerate without invoking the backend', async () => {
    const entry = makeEntry('ready');
    await renderWith(entry);
    await openMenu();

    act(() => {
      regenerateItem().dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });

    expect(onRegenerate).toHaveBeenCalledTimes(1);
    expect(onRegenerate).toHaveBeenCalledWith(entry);
  });

  it.each(['pending', 'ready', 'playing', 'error'] as const)(
    '"Перегенерировать аудио" is enabled for a %s entry',
    async (status) => {
      await renderWith(makeEntry(status));
      await openMenu();

      const item = regenerateItem();
      expect(
        item.hasAttribute('disabled') || item.dataset.disabled !== undefined,
      ).toBe(false);
    },
  );

  it('"Перегенерировать аудио" is disabled for a processing entry', async () => {
    await renderWith(makeEntry('processing'));
    await openMenu();

    const item = regenerateItem();
    expect(
      item.hasAttribute('disabled') || item.dataset.disabled !== undefined,
    ).toBe(true);
  });

  it('clicking "Отменить синтез" on a processing entry calls cancelSynthesis', async () => {
    await renderWith(makeEntry('processing'));
    await openMenu();

    act(() => {
      cancelItem().dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });

    expect(cancelSynthesis).toHaveBeenCalledTimes(1);
    expect(cancelSynthesis).toHaveBeenCalledWith('entry-1');
  });

  it('clicking "Отменить синтез" on a pending entry calls cancelSynthesis', async () => {
    // A pending entry may still be inside normalization (long texts), and the
    // backend accepts cancelling it — the menu item must stay enabled.
    await renderWith(makeEntry('pending'));
    await openMenu();

    act(() => {
      cancelItem().dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });

    expect(cancelSynthesis).toHaveBeenCalledTimes(1);
    expect(cancelSynthesis).toHaveBeenCalledWith('entry-1');
  });

  it.each(['ready', 'playing', 'error', 'cancelled'] as const)(
    '"Отменить синтез" is disabled for a %s entry',
    async (status) => {
      await renderWith(makeEntry(status));
      await openMenu();

      const item = cancelItem();
      expect(
        item.hasAttribute('disabled') || item.dataset.disabled !== undefined,
      ).toBe(true);
    },
  );

  it('shows an error notification when cancelSynthesis rejects', async () => {
    // Backend errors arrive as coded CommandError objects; formatError must
    // resolve the code, not stringify the object.
    cancelSynthesis.mockRejectedValueOnce({
      type: 'synthesis_error',
      code: 'synthesis.failed',
    });
    await renderWith(makeEntry('processing'));
    await openMenu();

    await act(async () => {
      cancelItem().dispatchEvent(new MouseEvent('click', { bubbles: true }));
      await Promise.resolve();
    });

    expect(showNotification).toHaveBeenCalledTimes(1);
    expect(showNotification).toHaveBeenCalledWith(
      expect.objectContaining({
        color: 'red',
        message: 'Не удалось отменить синтез: Ошибка синтеза речи',
      }),
    );
  });

  function exportItem(): HTMLElement {
    const el = Array.from(
      document.querySelectorAll<HTMLElement>('[role="menuitem"]'),
    ).find((e) => e.textContent === 'Сохранить аудио как…');
    expect(el).toBeDefined();
    return el!;
  }

  it('clicking "Сохранить аудио как…" exports the chosen path and confirms', async () => {
    pickExportAudioPath.mockResolvedValue('/home/user/ruvox-entry-1.opus');
    await renderWith(makeEntry('ready'));
    await openMenu();

    await act(async () => {
      exportItem().dispatchEvent(new MouseEvent('click', { bubbles: true }));
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(pickExportAudioPath).toHaveBeenCalledWith('entry-1');
    expect(exportAudio).toHaveBeenCalledWith('entry-1', '/home/user/ruvox-entry-1.opus');
    expect(showNotification).toHaveBeenCalledWith(
      expect.objectContaining({
        color: 'green',
        message: 'Аудио сохранено: /home/user/ruvox-entry-1.opus',
      }),
    );
  });

  it('a cancelled save dialog invokes no export and shows nothing', async () => {
    pickExportAudioPath.mockResolvedValue(null);
    await renderWith(makeEntry('ready'));
    await openMenu();

    await act(async () => {
      exportItem().dispatchEvent(new MouseEvent('click', { bubbles: true }));
      await Promise.resolve();
    });

    expect(exportAudio).not.toHaveBeenCalled();
    expect(showNotification).not.toHaveBeenCalled();
  });

  it('shows an error notification when exportAudio rejects', async () => {
    // The cached file was evicted: the coded export.no_audio error must be
    // localized, not stringified.
    pickExportAudioPath.mockResolvedValue('/home/user/ruvox-entry-1.opus');
    exportAudio.mockRejectedValueOnce({
      type: 'not_found',
      code: 'export.no_audio',
      params: ['entry-1'],
    });
    await renderWith(makeEntry('ready'));
    await openMenu();

    await act(async () => {
      exportItem().dispatchEvent(new MouseEvent('click', { bubbles: true }));
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(showNotification).toHaveBeenCalledTimes(1);
    expect(showNotification).toHaveBeenCalledWith(
      expect.objectContaining({
        color: 'red',
        message: 'У записи entry-1 нет сохранённого аудиофайла',
      }),
    );
  });

  it('shows an error notification when the save-dialog pick itself rejects', async () => {
    pickExportAudioPath.mockRejectedValueOnce({
      type: 'internal',
      code: 'export.dialog_panicked',
    });
    await renderWith(makeEntry('ready'));
    await openMenu();

    await act(async () => {
      exportItem().dispatchEvent(new MouseEvent('click', { bubbles: true }));
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(exportAudio).not.toHaveBeenCalled();
    expect(showNotification).toHaveBeenCalledTimes(1);
    expect(showNotification).toHaveBeenCalledWith(
      expect.objectContaining({
        color: 'red',
        message: 'Внутренняя ошибка при открытии диалога сохранения',
      }),
    );
  });

  it.each(['ready', 'playing'] as const)(
    '"Сохранить аудио как…" is enabled for a %s entry',
    async (status) => {
      await renderWith(makeEntry(status));
      await openMenu();

      const item = exportItem();
      expect(
        item.hasAttribute('disabled') || item.dataset.disabled !== undefined,
      ).toBe(false);
    },
  );

  it.each(['pending', 'processing', 'error'] as const)(
    '"Сохранить аудио как…" is disabled for a %s entry',
    async (status) => {
      await renderWith(makeEntry(status));
      await openMenu();

      const item = exportItem();
      expect(
        item.hasAttribute('disabled') || item.dataset.disabled !== undefined,
      ).toBe(true);
    },
  );

  function generationParamsItem(): HTMLElement {
    const el = Array.from(
      document.querySelectorAll<HTMLElement>('[role="menuitem"]'),
    ).find((e) => e.textContent === 'Параметры записи…');
    expect(el).toBeDefined();
    return el!;
  }

  it('clicking "Параметры озвучки…" opens the dialog with the snapshot', async () => {
    // The base factory carries no snapshot (menu item disabled); give this
    // entry one so the item is enabled and the dialog has data to show.
    const entry: TextEntry = {
      ...makeEntry('ready'),
      generation_count: 1,
      generation: {
        engine: 'silero_native',
        voice: 'xenia',
        sample_rate: 24000,
        model: null,
        app_version: '0.5.0',
        code_block_mode: 'read',
        normalized_text_sha256: null,
        audio_codec: 'Ogg Opus',
        audio_bytes: 1024,
      },
    };
    await renderWith(entry);
    await openMenu();

    act(() => {
      generationParamsItem().dispatchEvent(
        new MouseEvent('click', { bubbles: true }),
      );
    });

    // The dialog renders into a Mantine portal; wait for its title.
    await vi.waitFor(() => {
      expect(document.body.textContent).toContain('Silero (нативный)');
    });
  });

  it('"Параметры озвучки…" is disabled for an entry that never produced audio', async () => {
    await renderWith(makeEntry('pending'));
    await openMenu();

    const item = generationParamsItem();
    expect(
      item.hasAttribute('disabled') || item.dataset.disabled !== undefined,
    ).toBe(true);
  });

  it('"Параметры озвучки…" opens with the legacy line for an old audio entry', async () => {
    const legacy = {
      ...makeEntry('ready'),
      generation: null,
      generation_count: 0,
      audio_generated_at: '2026-01-01T10:00:00',
    };
    await renderWith(legacy);
    await openMenu();

    act(() => {
      generationParamsItem().dispatchEvent(
        new MouseEvent('click', { bubbles: true }),
      );
    });

    await vi.waitFor(() => {
      expect(document.body.textContent).toContain(
        'Параметры не записаны: аудио создано в более старой версии приложения.',
      );
    });
  });

  describe('double-click opens the parameters dialog', () => {
    function dblclickItem(): void {
      const item = host.querySelector('[data-entry-id="entry-1"]');
      expect(item).not.toBeNull();
      act(() => {
        item!.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
      });
    }

    it('opens the dialog for a synthesized entry', async () => {
      const entry: TextEntry = {
        ...makeEntry('ready'),
        audio_generated_at: '2026-08-31T00:00:00Z',
        generation: {
          engine: 'silero_native',
          voice: 'xenia',
          sample_rate: 24000,
          model: null,
          app_version: '0.5.0',
          code_block_mode: 'brief',
          normalized_text_sha256: null,
          audio_codec: 'Ogg Opus',
          audio_bytes: 1024,
        },
      };
      await renderWith(entry);
      dblclickItem();

      await vi.waitFor(() => {
        expect(document.body.textContent).toContain('Параметры записи');
      });
    });

    it('is a no-op for a never-synthesized entry', async () => {
      await renderWith(makeEntry('pending'));
      dblclickItem();

      expect(document.body.textContent).not.toContain('Параметры записи');
    });
  });
});


describe('QueueList synthesis failure', () => {
  let host: HTMLDivElement;
  let root: Root;
  const onRegenerate = vi.fn();

  beforeEach(() => {
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    onRegenerate.mockClear();
    regenerateEntry.mockClear();
    regenerateEntry.mockResolvedValue(undefined);
    showNotification.mockClear();
    entryUpdatedListeners.length = 0;
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
    // The overflow tests shadow jsdom's zero-height layout metrics; drop the
    // shadow so other files inherit the real prototype again.
    Reflect.deleteProperty(HTMLElement.prototype, 'scrollHeight');
    Reflect.deleteProperty(HTMLElement.prototype, 'clientHeight');
  });

  async function renderWith(entry: TextEntry): Promise<void> {
    getEntries.mockResolvedValue([entry]);
    await act(async () => {
      root.render(
        <MantineProvider>
          <QueueList onRegenerate={onRegenerate} />
        </MantineProvider>,
      );
      await Promise.resolve();
    });
  }

  function failureBlock(): HTMLElement | null {
    return host.querySelector<HTMLElement>('[data-synthesis-failure]');
  }

  function failureText(): HTMLElement {
    const el = host.querySelector<HTMLElement>('[data-failure-text]');
    expect(el).not.toBeNull();
    return el!;
  }

  function retryButton(): HTMLButtonElement {
    const el = host.querySelector<HTMLButtonElement>(
      'button[aria-label="Повторить"]',
    );
    expect(el).not.toBeNull();
    return el!;
  }

  function buttonWithText(text: string): HTMLButtonElement {
    const el = Array.from(
      host.querySelectorAll<HTMLButtonElement>('button'),
    ).find((button) => button.textContent === text);
    expect(el).toBeDefined();
    return el!;
  }

  /** jsdom reports zero layout heights, so the two-line clamp cannot be
   * measured for real. Shadow the metrics on the prototype: a 100 px content
   * box inside a 40 px client box means "overflows", 40 inside 40 means it
   * fits. */
  function stubLayout(overflows: boolean): void {
    Object.defineProperty(HTMLElement.prototype, 'scrollHeight', {
      configurable: true,
      get: () => (overflows ? 100 : 40),
    });
    Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
      configurable: true,
      get: () => 40,
    });
  }

  it('renders the stored failure reason from the entry', async () => {
    const entry = makeEntry('error', {
      error_message: 'модель не загружена (ttsd)',
    });
    // No entry_updated is delivered: the message comes from the entry
    // snapshot loaded on mount, so it survives a restart.
    await renderWith(entry);

    const block = failureBlock();
    expect(block).not.toBeNull();
    expect(block!.dataset.synthesisFailure).toBe('error');
    expect(failureText().textContent).toBe(
      'Ошибка синтеза: модель не загружена (ttsd)',
    );
    expect(failureText().title).toBe(
      'Ошибка синтеза: модель не загружена (ttsd)',
    );
    expect(
      host.querySelector('[data-entry-id="entry-1"]')!.className,
    ).toContain('itemError');
  });

  it('expands and collapses a message that overflows the clamp', async () => {
    stubLayout(true);
    await renderWith(
      makeEntry('error', {
        error_message: 'Очень длинное сообщение движка. '.repeat(10),
      }),
    );

    expect(buttonWithText('Показать полностью')).toBeDefined();
    act(() => {
      buttonWithText('Показать полностью').dispatchEvent(
        new MouseEvent('click', { bubbles: true }),
      );
    });

    expect(buttonWithText('Свернуть')).toBeDefined();
    expect(failureText().textContent).toContain('Очень длинное сообщение');

    act(() => {
      buttonWithText('Свернуть').dispatchEvent(
        new MouseEvent('click', { bubbles: true }),
      );
    });
    expect(buttonWithText('Показать полностью')).toBeDefined();
  });

  it('shows no expand control for a message that fits the clamp', async () => {
    stubLayout(false);
    await renderWith(makeEntry('error', { error_message: 'сбой' }));

    expect(host.textContent).not.toContain('Показать полностью');
  });

  it('retries through regenerateEntry exactly once and blocks while busy', async () => {
    await renderWith(makeEntry('error', { error_message: 'сбой движка' }));

    act(() => {
      retryButton().dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });

    expect(regenerateEntry).toHaveBeenCalledTimes(1);
    expect(regenerateEntry).toHaveBeenCalledWith('entry-1', false);
    // No regeneration preview: the queue calls the command directly.
    expect(onRegenerate).not.toHaveBeenCalled();
    expect(retryButton().disabled).toBe(true);

    act(() => {
      retryButton().dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });
    expect(regenerateEntry).toHaveBeenCalledTimes(1);
  });

  it('clears the busy state when entry_updated clears the failure', async () => {
    const entry = makeEntry('error', { error_message: 'сбой движка' });
    await renderWith(entry);
    act(() => {
      retryButton().dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });
    expect(retryButton().disabled).toBe(true);

    await act(async () => {
      for (const listener of entryUpdatedListeners) {
        listener({
          entry: { ...entry, status: 'pending', error_message: null },
        });
      }
      await Promise.resolve();
    });

    expect(failureBlock()).toBeNull();
    expect(host.textContent).not.toContain('Повторить');
  });

  it('shows the amber failed-regeneration note on a playable entry', async () => {
    await renderWith(
      makeEntry('ready', {
        error_message: 'таймаут ttsd',
        audio_path: 'entry-1.opus',
      }),
    );

    const block = failureBlock();
    expect(block).not.toBeNull();
    expect(block!.dataset.synthesisFailure).toBe('warning');
    expect(failureText().textContent).toBe(
      'Последняя перегенерация не удалась: таймаут ttsd',
    );
    expect(host.textContent).toContain('Играет прежняя версия.');

    const play = host.querySelector<HTMLButtonElement>(
      'button[aria-label="Воспроизвести"]',
    );
    expect(play).not.toBeNull();
    expect(play!.disabled).toBe(false);
    expect(
      host.querySelector('[data-entry-id="entry-1"]')!.className,
    ).not.toContain('itemError');
    expect(retryButton().disabled).toBe(false);
  });

  it('clears the busy state and shows the red toast when the retry rejects', async () => {
    regenerateEntry.mockRejectedValueOnce({
      type: 'synthesis_error',
      code: 'synthesis.failed',
    });
    await renderWith(makeEntry('error', { error_message: 'сбой движка' }));

    await act(async () => {
      retryButton().dispatchEvent(new MouseEvent('click', { bubbles: true }));
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(retryButton().disabled).toBe(false);
    expect(showNotification).toHaveBeenCalledWith(
      expect.objectContaining({
        color: 'red',
        message: 'Не удалось запустить перегенерацию: Ошибка синтеза речи',
      }),
    );
  });

  it('leaves a healthy entry without a failure block', async () => {
    await renderWith(makeEntry('ready'));

    expect(failureBlock()).toBeNull();
    expect(host.textContent).not.toContain('Повторить');
    expect(host.textContent).not.toContain('Ошибка синтеза');
    expect(
      host.querySelector('[data-entry-id="entry-1"]')!.className,
    ).not.toContain('itemError');
  });
});
