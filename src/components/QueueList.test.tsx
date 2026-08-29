// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import type { TextEntry } from '../lib/tauri';

const { cancelSynthesis, getEntries, showNotification, pickExportAudioPath, exportAudio } =
  vi.hoisted(() => ({
    cancelSynthesis: vi.fn().mockResolvedValue(undefined),
    getEntries: vi.fn(),
    showNotification: vi.fn(),
    pickExportAudioPath: vi.fn().mockResolvedValue(null),
    exportAudio: vi.fn().mockResolvedValue(undefined),
  }));

vi.mock('@mantine/notifications', () => ({
  notifications: { show: showNotification },
}));

vi.mock('../lib/tauri', () => ({
  commands: {
    getEntries,
    cancelSynthesis,
    playEntry: vi.fn().mockResolvedValue(undefined),
    regenerateEntry: vi.fn().mockResolvedValue(undefined),
    deleteEntry: vi.fn().mockResolvedValue(undefined),
    pickExportAudioPath,
    exportAudio,
  },
  events: {
    entryUpdated: vi.fn().mockResolvedValue(() => {}),
    entryRemoved: vi.fn().mockResolvedValue(() => {}),
    playbackStarted: vi.fn().mockResolvedValue(() => {}),
    playbackStopped: vi.fn().mockResolvedValue(() => {}),
    playbackFinished: vi.fn().mockResolvedValue(() => {}),
  },
}));

import { QueueList } from './QueueList';

function makeEntry(status: TextEntry['status']): TextEntry {
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
  };
}

describe('QueueList context menu', () => {
  let host: HTMLDivElement;
  let root: Root;

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
          <QueueList />
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

  it('clicking "Отменить синтез" on a processing entry calls cancelSynthesis', async () => {
    await renderWith(makeEntry('processing'));
    await openMenu();

    act(() => {
      cancelItem().dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });

    expect(cancelSynthesis).toHaveBeenCalledTimes(1);
    expect(cancelSynthesis).toHaveBeenCalledWith('entry-1');
  });

  it.each(['ready', 'playing', 'pending', 'error'] as const)(
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
        read_operators: true,
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
});
