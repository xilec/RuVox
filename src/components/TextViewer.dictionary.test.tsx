// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import type { TextEntry } from '../lib/tauri';

vi.mock('../lib/tauri', () => ({
  commands: {
    getTimestamps: vi.fn().mockResolvedValue([]),
    setEntryDisplayMode: vi.fn().mockResolvedValue(undefined),
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
vi.mock('../lib/viewerCopy', () => ({
  copyLinkAddress: vi.fn(),
  copySelection: vi.fn(),
  copyImageAddress: vi.fn(),
  copyImageBitmap: vi.fn(),
}));

import { TextViewer } from './TextViewer';

function makeEntry(overrides: Partial<TextEntry> = {}): TextEntry {
  return {
    id: 'entry-1',
    original_text: 'Вызови getUserData через API',
    normalized_text: null,
    status: 'ready',
    format: 'plain',
    display_mode: null,
    html_source: null,
    source: null,
    title: null,
    note: null,
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

/** One entry per display path: verbatim plain text, rendered Markdown and
 *  rendered HTML. The quick-add gate must be identical in all three. */
const MODES = [
  {
    mode: 'plain',
    word: 'getUserData',
    entry: makeEntry({
      format: 'plain',
      original_text: 'Вызови getUserData через API',
    }),
  },
  {
    mode: 'markdown',
    word: 'Установка',
    entry: makeEntry({
      format: 'markdown',
      original_text: '## Установка\n\nЗапусти pnpm install.',
    }),
  },
  {
    mode: 'html',
    word: 'getUserData',
    entry: makeEntry({
      format: 'html',
      original_text: 'Вызови getUserData через API',
      html_source: '<p>Вызови getUserData через API</p>',
    }),
  },
];

function menuItems(): string[] {
  return Array.from(document.body.querySelectorAll('[role="menuitem"]')).map(
    (el) => el.textContent ?? '',
  );
}

describe.each(MODES)('viewer quick-add in $mode mode', ({ word, entry }) => {
  let host: HTMLDivElement;
  let root: Root;
  let onAddToDictionary: ReturnType<typeof vi.fn>;

  /** Select `word` in the rendered document and return the element to
   *  right-click (the text node's parent inside the viewer container). */
  function selectWord(): HTMLElement {
    const container = host.querySelector('[data-reading-width]');
    expect(container, 'viewer content container').not.toBeNull();
    const selection = window.getSelection();
    selection?.removeAllRanges();
    const walker = document.createTreeWalker(container as Node, NodeFilter.SHOW_TEXT);
    let node: Node | null = walker.nextNode();
    while (node !== null) {
      const text = node.textContent ?? '';
      const start = text.indexOf(word);
      if (start >= 0) {
        const range = document.createRange();
        range.setStart(node, start);
        range.setEnd(node, start + word.length);
        selection?.addRange(range);
        return node.parentElement as HTMLElement;
      }
      node = walker.nextNode();
    }
    throw new Error(`selection target not found: ${word}`);
  }

  beforeEach(() => {
    onAddToDictionary = vi.fn();
    window.getSelection()?.removeAllRanges();
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    act(() => {
      root.render(
        <MantineProvider>
          <TextViewer entry={entry} onAddToDictionary={onAddToDictionary} />
        </MantineProvider>,
      );
    });
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
    window.getSelection()?.removeAllRanges();
  });

  it('offers «В словарь» for a single-word selection', async () => {
    const target = selectWord();
    act(() => {
      target.dispatchEvent(
        new MouseEvent('contextmenu', {
          bubbles: true,
          cancelable: true,
          clientX: 10,
          clientY: 10,
        }),
      );
    });
    // The Mantine dropdown mounts into the portal asynchronously.
    await act(async () => {
      await new Promise((r) => setTimeout(r, 50));
    });

    expect(menuItems()).toEqual(['В словарь', 'Копировать']);

    const item = Array.from(
      document.body.querySelectorAll('[role="menuitem"]'),
    ).find((el) => el.textContent === 'В словарь');
    act(() => (item as HTMLElement).click());
    expect(onAddToDictionary).toHaveBeenCalledWith(word);
  });
});
