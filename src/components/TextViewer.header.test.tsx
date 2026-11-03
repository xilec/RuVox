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
    setEntryTitle: vi.fn().mockResolvedValue(undefined),
    setEntryNote: vi.fn().mockResolvedValue(undefined),
    updateConfig: vi.fn().mockResolvedValue(undefined),
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
vi.mock('../lib/viewerCopy', () => ({ copyLinkAddress: vi.fn() }));

import { TextViewer } from './TextViewer';
import { useLocaleStore } from '../stores/locale';

function makeEntry(overrides: Partial<TextEntry> = {}): TextEntry {
  return {
    id: 'entry-1',
    original_text: '## Установка\n\nЗапусти `pnpm install`.',
    normalized_text: null,
    status: 'ready',
    format: 'markdown',
    display_mode: null,
    html_source: null,
    source: null,
    title: 'Тёмная сторона инлайнинга',
    note: 'вернуться к бенчмаркам Wasm',
    created_at: '2026-11-02T07:03:00',
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

describe('TextViewer entry header', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    useLocaleStore.setState({ locale: 'ru' });
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
  });

  function render(entry: TextEntry): void {
    act(() => {
      root.render(
        <MantineProvider>
          <TextViewer entry={entry} />
        </MantineProvider>,
      );
    });
  }

  function titleButton(label = 'Тёмная сторона инлайнинга'): HTMLButtonElement | null {
    return host.querySelector<HTMLButtonElement>(
      `button[aria-label="${label}"]`,
    );
  }

  it('renders the header above the reading area, outside the scroll content', () => {
    render(makeEntry());

    const title = titleButton();
    expect(title).not.toBeNull();
    expect(title!.textContent).toBe('Тёмная сторона инлайнинга');
    expect(host.textContent).toContain('вернуться к бенчмаркам Wasm');
    expect(host.textContent).toContain('Готово');

    const content = host.querySelector<HTMLElement>('[data-reading-width]');
    expect(content).not.toBeNull();
    expect(content!.contains(title)).toBe(false);
  });

  it('shows the derived title and the empty-note affordance for a legacy entry', () => {
    render(makeEntry({ title: null, note: null }));

    expect(titleButton('Установка')!.textContent).toBe('Установка');
    const addNote = [...host.querySelectorAll('button')].find(
      (b) => b.textContent?.trim() === 'Добавить заметку',
    );
    expect(addNote).not.toBeUndefined();
  });

  it('keeps the format switch and reading controls together in the header', () => {
    render(makeEntry());

    expect(
      host.querySelectorAll('input[name="viewer_display_mode"]'),
    ).toHaveLength(2);
    expect(
      host.querySelectorAll('input[name="viewer_reading_width"]'),
    ).toHaveLength(2);
    expect(
      host.querySelector('[role="group"][aria-label="Масштаб чтения"]'),
    ).not.toBeNull();
  });

  it('keeps the reading controls for a plain source without a display switch', () => {
    render(makeEntry({ format: 'plain', original_text: 'Просто заметка' }));

    expect(
      host.querySelectorAll('input[name="viewer_display_mode"]'),
    ).toHaveLength(0);
    expect(
      host.querySelectorAll('input[name="viewer_reading_width"]'),
    ).toHaveLength(2);
  });
});
