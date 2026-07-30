// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import type { TextEntry } from '../lib/tauri';

const { copyLinkAddress } = vi.hoisted(() => ({ copyLinkAddress: vi.fn() }));

vi.mock('../lib/tauri', () => ({
  commands: {
    getTimestamps: vi.fn().mockResolvedValue([]),
    setEntryFormat: vi.fn().mockResolvedValue(undefined),
  },
  events: {
    playbackStarted: vi.fn().mockResolvedValue(() => {}),
    playbackPosition: vi.fn().mockResolvedValue(() => {}),
    playbackStopped: vi.fn().mockResolvedValue(() => {}),
    playbackFinished: vi.fn().mockResolvedValue(() => {}),
    playbackPaused: vi.fn().mockResolvedValue(() => {}),
  },
}));
vi.mock('../lib/mermaid', () => ({
  renderMermaidIn: vi.fn().mockResolvedValue(undefined),
}));
vi.mock('../lib/viewerCopy', () => ({ copyLinkAddress }));

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

import { TextViewer } from './TextViewer';

function makeEntry(): TextEntry {
  return {
    id: 'entry-1',
    original_text: 'user maybe_elf',
    normalized_text: null,
    status: 'ready',
    format: 'html',
    html_source:
      '<p><a href="/ru/users/maybe_elf/">maybe_elf</a></p>' +
      '<button>Кнопка</button>',
    created_at: '2026-07-27T00:00:00Z',
    audio_generated_at: null,
    audio_path: null,
    timestamps_path: null,
    duration_sec: null,
    was_regenerated: false,
    error_message: null,
  };
}

describe('TextViewer read-only behavior', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    host = document.createElement('div');
    document.body.appendChild(host);
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
  });

  function renderWith(entry: TextEntry | null): void {
    act(() => {
      root.render(
        <MantineProvider>
          <TextViewer entry={entry} />
        </MantineProvider>,
      );
    });
  }

  // Regression: the viewer mounts without an entry (placeholder, no content
  // container), so a click listener attached in an effect with empty deps
  // never lands — links stayed navigable. The listener must attach once an
  // entry appears.
  it('blocks link navigation after an entry appears post-mount', () => {
    root = createRoot(host);
    renderWith(null);
    renderWith(makeEntry());

    const link = host.querySelector('a');
    expect(link).not.toBeNull();
    const event = new MouseEvent('click', { bubbles: true, cancelable: true });
    link?.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
  });

  it('neutralizes interactive elements in the rendered content', () => {
    root = createRoot(host);
    renderWith(makeEntry());

    expect(host.querySelector('button')?.hasAttribute('disabled')).toBe(true);
    expect(host.querySelector('a')?.getAttribute('title')).toBe(
      '/ru/users/maybe_elf/',
    );
  });

  it('copies the verbatim link href on Ctrl+C with a focused link', () => {
    root = createRoot(host);
    renderWith(makeEntry());

    const link = host.querySelector('a');
    link?.focus();
    act(() => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'c',
          ctrlKey: true,
          bubbles: true,
          cancelable: true,
        }),
      );
    });
    expect(copyLinkAddress).toHaveBeenCalledWith('/ru/users/maybe_elf/');
  });
});
