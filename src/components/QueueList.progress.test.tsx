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
  entryUpdatedListeners,
  dialogRenders,
} = vi.hoisted(() => ({
  cancelSynthesis: vi.fn().mockResolvedValue(undefined),
  getEntries: vi.fn(),
  showNotification: vi.fn(),
  // Listeners QueueList registers on mount; tests deliver `entry_updated`
  // payloads through them to drive status transitions.
  entryUpdatedListeners: [] as ((payload: { entry: TextEntry }) => void)[],
  // The params dialog is rendered by every QueueItem, so its invocation count
  // doubles as the item render count: a tick must not change it.
  dialogRenders: { count: 0 },
}));

vi.mock('@mantine/notifications', () => ({
  notifications: { show: showNotification },
}));

vi.mock('../dialogs/GenerationParamsDialog', () => ({
  GenerationParamsDialog: function MockGenerationParamsDialog() {
    dialogRenders.count += 1;
    return null;
  },
}));

vi.mock('../lib/tauri', () => ({
  commands: {
    getEntries,
    cancelSynthesis,
    playEntry: vi.fn().mockResolvedValue(undefined),
    regenerateEntry: vi.fn().mockResolvedValue(undefined),
    deleteEntry: vi.fn().mockResolvedValue(undefined),
    pickExportAudioPath: vi.fn().mockResolvedValue(null),
    exportAudio: vi.fn().mockResolvedValue(undefined),
  },
  events: {
    entryUpdated: vi.fn((cb: (payload: { entry: TextEntry }) => void) => {
      entryUpdatedListeners.push(cb);
      return Promise.resolve(() => {});
    }),
    entryRemoved: vi.fn().mockResolvedValue(() => {}),
    playbackStarted: vi.fn().mockResolvedValue(() => {}),
    playbackStopped: vi.fn().mockResolvedValue(() => {}),
    playbackFinished: vi.fn().mockResolvedValue(() => {}),
  },
}));

import { QueueList } from './QueueList';
import { useLocaleStore } from '../stores/locale';
import { useSelectedEntry } from '../stores/selectedEntry';

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

describe('QueueList synthesis progress', () => {
  let host: HTMLDivElement;
  let root: Root;
  const onRegenerate = vi.fn();

  beforeEach(() => {
    vi.useFakeTimers();
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    useLocaleStore.setState({ locale: 'ru' });
    useSelectedEntry.setState({ selectedId: null, selectedEntry: null });
    cancelSynthesis.mockReset();
    cancelSynthesis.mockResolvedValue(undefined);
    showNotification.mockClear();
    entryUpdatedListeners.length = 0;
    dialogRenders.count = 0;
    getEntries.mockReset();
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
    vi.useRealTimers();
    useLocaleStore.setState({ locale: 'ru' });
  });

  async function renderWithEntries(entries: TextEntry[]): Promise<void> {
    getEntries.mockResolvedValue(entries);
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

  function row(id = 'entry-1'): HTMLElement {
    const el = host.querySelector<HTMLElement>(`[data-entry-id="${id}"]`);
    expect(el).not.toBeNull();
    return el!;
  }

  function progressLine(id = 'entry-1'): HTMLElement | null {
    return row(id).querySelector<HTMLElement>('[data-synthesis-progress]');
  }

  function elapsedText(id = 'entry-1'): string {
    const el = row(id).querySelector<HTMLElement>('[data-synthesis-elapsed]');
    expect(el).not.toBeNull();
    return el!.textContent ?? '';
  }

  function cancelButton(id = 'entry-1'): HTMLButtonElement {
    const el = row(id).querySelector<HTMLButtonElement>('[data-synthesis-cancel]');
    expect(el).not.toBeNull();
    return el!;
  }

  async function deliverEntryUpdated(entry: TextEntry): Promise<void> {
    await act(async () => {
      for (const listener of entryUpdatedListeners) {
        listener({ entry });
      }
      await Promise.resolve();
    });
  }

  async function advance(ms: number): Promise<void> {
    await act(async () => {
      vi.advanceTimersByTime(ms);
      await Promise.resolve();
    });
  }

  async function clickCancel(id = 'entry-1'): Promise<void> {
    await act(async () => {
      cancelButton(id).dispatchEvent(new MouseEvent('click', { bubbles: true }));
      await Promise.resolve();
    });
  }

  it('shows the elapsed time and the visible cancel action on a processing card', async () => {
    await renderWithEntries([makeEntry('processing')]);

    expect(progressLine()!.dataset.synthesisProgress).toBe('processing');
    expect(elapsedText()).toBe('Синтез… 0:00');
    expect(cancelButton().textContent).toBe('Отменить');
    expect(cancelButton().disabled).toBe(false);
  });

  it('starts the timer when the entry becomes processing, not while pending', async () => {
    await renderWithEntries([makeEntry('pending')]);

    expect(progressLine()!.dataset.synthesisProgress).toBe('pending');
    expect(row().textContent).toContain('В очереди');
    expect(row().querySelector('[data-synthesis-elapsed]')).toBeNull();

    // Time spent queued must not leak into the elapsed time.
    await advance(5000);
    expect(row().querySelector('[data-synthesis-elapsed]')).toBeNull();

    await deliverEntryUpdated(makeEntry('processing'));
    expect(elapsedText()).toBe('Синтез… 0:00');

    await advance(2000);
    expect(elapsedText()).toBe('Синтез… 0:02');
  });

  it('advances the elapsed text once per second', async () => {
    await renderWithEntries([makeEntry('processing')]);
    expect(elapsedText()).toBe('Синтез… 0:00');

    await advance(1000);
    expect(elapsedText()).toBe('Синтез… 0:01');

    await advance(1000);
    expect(elapsedText()).toBe('Синтез… 0:02');
  });

  it('does not restart the timer when a fresh processing update arrives', async () => {
    await renderWithEntries([makeEntry('processing')]);
    await advance(2000);
    expect(elapsedText()).toBe('Синтез… 0:02');

    await deliverEntryUpdated(makeEntry('processing'));

    // A repeated `processing` for the same entry is not a new start: the
    // line keeps its clock and its single interval.
    expect(elapsedText()).toBe('Синтез… 0:02');
    expect(vi.getTimerCount()).toBe(1);

    await advance(1000);
    expect(elapsedText()).toBe('Синтез… 0:03');
  });

  it('clamps the elapsed time when the system clock jumps back', async () => {
    await renderWithEntries([makeEntry('processing')]);
    await advance(2000);
    expect(elapsedText()).toBe('Синтез… 0:02');

    // An NTP correction or a manual clock change must not render `-60:00`.
    vi.setSystemTime(new Date(Date.now() - 3_600_000));
    await advance(1000);

    expect(elapsedText()).toBe('Синтез… 0:00');
  });

  it.each(['ready', 'error', 'cancelled'] as const)(
    'stops the timer and removes the line when the entry becomes %s',
    async (status) => {
      await renderWithEntries([makeEntry('processing')]);
      await advance(2000);
      expect(elapsedText()).toBe('Синтез… 0:02');

      await deliverEntryUpdated(
        makeEntry(status, status === 'error' ? { error_message: 'сбой' } : {}),
      );

      expect(progressLine()).toBeNull();
      expect(vi.getTimerCount()).toBe(0);

      // No late tick may bring the line back or update a stale timer.
      await advance(5000);
      expect(row().textContent).not.toContain('Синтез…');
    },
  );

  it('clears the interval on unmount', async () => {
    await renderWithEntries([makeEntry('processing')]);
    expect(vi.getTimerCount()).toBe(1);

    act(() => root.unmount());
    expect(vi.getTimerCount()).toBe(0);

    // Leave a live root for afterEach; it renders nothing.
    root = createRoot(host);
  });

  it('cancels a queued entry from the inline action', async () => {
    await renderWithEntries([makeEntry('pending')]);

    await clickCancel();

    expect(cancelSynthesis).toHaveBeenCalledTimes(1);
    expect(cancelSynthesis).toHaveBeenCalledWith('entry-1');
  });

  it.each(['Enter', ' '] as const)(
    '%s on the inline cancel button cancels without selecting the row',
    async (key) => {
      await renderWithEntries([makeEntry('processing')]);
      const button = cancelButton();

      let defaultPrevented = false;
      act(() => {
        const event = new KeyboardEvent('keydown', {
          key,
          bubbles: true,
          cancelable: true,
        });
        button.dispatchEvent(event);
        defaultPrevented = event.defaultPrevented;
      });

      // The card must not hijack Enter/Space from its nested cancel action:
      // no preventDefault (that would cancel the button's activation) and no
      // row selection.
      expect(defaultPrevented).toBe(false);
      expect(useSelectedEntry.getState().selectedId).toBeNull();

      // The browser's default action for Enter/Space on a native button.
      await act(async () => {
        button.dispatchEvent(new MouseEvent('click', { bubbles: true }));
        await Promise.resolve();
      });
      expect(cancelSynthesis).toHaveBeenCalledTimes(1);
      expect(cancelSynthesis).toHaveBeenCalledWith('entry-1');
    },
  );

  it('clicking the inline cancel button does not select the row', async () => {
    await renderWithEntries([makeEntry('processing')]);

    await clickCancel();

    expect(cancelSynthesis).toHaveBeenCalledTimes(1);
    expect(useSelectedEntry.getState().selectedId).toBeNull();
  });
  it('cancels exactly once and disables the button while the call is in flight', async () => {
    let resolveCancel: (() => void) | undefined;
    cancelSynthesis.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          resolveCancel = resolve;
        }),
    );
    await renderWithEntries([makeEntry('processing')]);

    await clickCancel();

    expect(cancelSynthesis).toHaveBeenCalledTimes(1);
    expect(cancelSynthesis).toHaveBeenCalledWith('entry-1');
    expect(cancelButton().disabled).toBe(true);

    await clickCancel();
    expect(cancelSynthesis).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveCancel?.();
      await Promise.resolve();
    });
    expect(cancelButton().disabled).toBe(false);
  });

  it('re-enables the button and shows the localized toast when cancel rejects', async () => {
    // Backend errors arrive as coded CommandError objects; formatError must
    // resolve the code, not stringify the object.
    cancelSynthesis.mockRejectedValueOnce({
      type: 'synthesis_error',
      code: 'synthesis.failed',
    });
    await renderWithEntries([makeEntry('processing')]);

    await act(async () => {
      cancelButton().dispatchEvent(new MouseEvent('click', { bubbles: true }));
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(cancelButton().disabled).toBe(false);
    expect(showNotification).toHaveBeenCalledWith(
      expect.objectContaining({
        color: 'red',
        message: 'Не удалось отменить синтез: Ошибка синтеза речи',
      }),
    );
  });

  it('labels the cancel action with the entry name', async () => {
    const longText =
      'Тёмная сторона инлайнинга в современном фронтенде и за его пределами';
    await renderWithEntries([makeEntry('processing', { original_text: longText })]);

    expect(cancelButton().getAttribute('aria-label')).toBe(
      `Отменить синтез: ${longText.slice(0, 60)}\u2026`,
    );
  });

  it('relabels the progress line after a locale switch', async () => {
    await renderWithEntries([
      makeEntry('pending'),
      makeEntry('processing', { id: 'entry-2', original_text: 'другой текст' }),
    ]);
    await advance(5000);

    act(() => {
      useLocaleStore.setState({ locale: 'en' });
    });

    expect(row().textContent).toContain('Queued');
    expect(row('entry-2').textContent).toContain('Synthesizing… 0:05');
    expect(cancelButton('entry-2').textContent).toBe('Cancel');
  });

  it('re-renders only the ticking line on a timer tick', async () => {
    await renderWithEntries([
      makeEntry('processing'),
      makeEntry('ready', { id: 'entry-2', original_text: 'готовый текст' }),
    ]);
    // One call per card render: the mocked dialog is a render counter.
    const rendersAfterMount = dialogRenders.count;
    expect(rendersAfterMount).toBe(2);

    await advance(1000);

    expect(elapsedText()).toBe('Синтез… 0:01');
    // A tick updates the line's own state; the cards themselves do not
    // re-render, so the count is unchanged.
    expect(dialogRenders.count).toBe(rendersAfterMount);
  });
});
