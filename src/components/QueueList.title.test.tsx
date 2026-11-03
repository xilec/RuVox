// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import type { TextEntry } from '../lib/tauri';

const { getEntries } = vi.hoisted(() => ({ getEntries: vi.fn() }));

vi.mock('../lib/tauri', () => ({
  commands: {
    getEntries,
    cancelSynthesis: vi.fn().mockResolvedValue(undefined),
    playEntry: vi.fn().mockResolvedValue(undefined),
    regenerateEntry: vi.fn().mockResolvedValue(undefined),
    deleteEntry: vi.fn().mockResolvedValue(undefined),
    pickExportAudioPath: vi.fn().mockResolvedValue(null),
    exportAudio: vi.fn().mockResolvedValue(undefined),
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
import { useSearchQuery } from '../stores/searchQuery';

function makeEntry(overrides: Partial<TextEntry> = {}): TextEntry {
  return {
    id: 'entry-1',
    original_text: 'тестовый текст',
    normalized_text: null,
    status: 'ready',
    format: 'plain',
    display_mode: null,
    html_source: null,
    source: null,
    title: 'Заголовок',
    note: null,
    created_at: '2026-07-27T00:00:00',
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

const ENTRIES: TextEntry[] = [
  makeEntry({ id: 'a', title: 'Введение в Rust', original_text: 'Первая заметка' }),
  makeEntry({
    id: 'b',
    title: null,
    note: 'про бенчмарки Wasm',
    original_text: '## Установка\nТекст',
  }),
  makeEntry({
    id: 'c',
    title: null,
    note: null,
    original_text: 'Старый текст про serde_json',
  }),
];

describe('Queue item titles and search (#353)', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    useSearchQuery.setState({ query: '' });
    getEntries.mockResolvedValue(ENTRIES.map((e) => ({ ...e })));
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
    useSearchQuery.setState({ query: '' });
  });

  async function renderList(): Promise<void> {
    await act(async () => {
      root.render(
        <MantineProvider>
          <QueueList onRegenerate={() => {}} />
        </MantineProvider>,
      );
      await Promise.resolve();
    });
  }

  function visibleIds(): string[] {
    return [...host.querySelectorAll<HTMLElement>('[data-entry-id]')].map(
      (el) => el.dataset.entryId ?? '',
    );
  }

  async function search(query: string): Promise<void> {
    await act(async () => {
      useSearchQuery.setState({ query });
      await Promise.resolve();
    });
  }

  it('shows the stored title instead of the 60-character text slice', async () => {
    await renderList();

    expect(visibleIds()).toEqual(['a', 'b', 'c']);
    expect(host.textContent).toContain('Введение в Rust');
    // The old preview was a slice of original_text; the card no longer shows
    // it when a title exists.
    expect(host.textContent).not.toContain('Первая заметка');
  });

  it('puts the full title in the tooltip', async () => {
    await renderList();

    const titles = [...host.querySelectorAll<HTMLElement>('[title]')].map((el) =>
      el.getAttribute('title'),
    );
    expect(titles).toContain('Введение в Rust');
  });

  it('derives the title for legacy entries without one', async () => {
    await renderList();

    expect(host.textContent).toContain('Установка');
    expect(host.textContent).toContain('Старый текст про serde_json');
  });

  it('filters by title, case-insensitively', async () => {
    await renderList();
    await search('rust');

    expect(visibleIds()).toEqual(['a']);
  });

  it('filters by note', async () => {
    await renderList();
    await search('бенчмарки');

    expect(visibleIds()).toEqual(['b']);
  });

  it('still filters by original_text', async () => {
    await renderList();
    await search('SERDE');

    expect(visibleIds()).toEqual(['c']);
  });

  it('shows the empty-search message when nothing matches', async () => {
    await renderList();
    await search('нет такого');

    expect(visibleIds()).toEqual([]);
    expect(host.textContent).toContain('Ничего не найдено');
  });

  it('filters by the derived title of a legacy entry', async () => {
    // The derived title is not a substring of `original_text`: the Markdown
    // marker and the repeated spaces only disappear in the derived form, so a
    // filter over the raw fields alone would miss the entry the card shows.
    getEntries.mockResolvedValue([
      makeEntry({
        id: 'derived',
        title: null,
        note: null,
        original_text: '## Заголовок  с   пробелами\nТекст',
      }),
    ]);
    await renderList();
    await search('заголовок с пробелами');

    expect(visibleIds()).toEqual(['derived']);
  });
});
