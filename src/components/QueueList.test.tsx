// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import type { TextEntry } from '../lib/tauri';

const { cancelSynthesis, getEntries, showNotification } = vi.hoisted(() => ({
  cancelSynthesis: vi.fn().mockResolvedValue(undefined),
  getEntries: vi.fn(),
  showNotification: vi.fn(),
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
  },
  events: {
    entryUpdated: vi.fn().mockResolvedValue(() => {}),
    entryRemoved: vi.fn().mockResolvedValue(() => {}),
    playbackStarted: vi.fn().mockResolvedValue(() => {}),
    playbackStopped: vi.fn().mockResolvedValue(() => {}),
    playbackFinished: vi.fn().mockResolvedValue(() => {}),
  },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

// jsdom has no matchMedia; Mantine's useComputedColorScheme needs it.
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
});

// jsdom has no ResizeObserver; Mantine's ScrollArea needs it.
class ResizeObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}
(globalThis as { ResizeObserver?: unknown }).ResizeObserver ??=
  ResizeObserverStub;

import { QueueList } from './QueueList';

function makeEntry(status: TextEntry['status']): TextEntry {
  return {
    id: 'entry-1',
    original_text: 'тестовый текст',
    normalized_text: null,
    status,
    format: 'plain',
    html_source: null,
    created_at: '2026-07-27T00:00:00Z',
    audio_generated_at: null,
    audio_path: null,
    timestamps_path: null,
    duration_sec: null,
    was_regenerated: false,
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
    cancelSynthesis.mockRejectedValueOnce(new Error('boom'));
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
        message: 'Не удалось отменить синтез: boom',
      }),
    );
  });
});
