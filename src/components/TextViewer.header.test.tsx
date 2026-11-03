// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import { commands, type TextEntry } from '../lib/tauri';

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
    vi.mocked(commands.setEntryTitle).mockClear();
    vi.mocked(commands.setEntryNote).mockClear();
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

    const header = host.querySelector<HTMLElement>('[data-entry-header]');
    expect(header).not.toBeNull();
    expect(
      header!.contains(host.querySelector('input[name="viewer_display_mode"]')),
    ).toBe(true);
    expect(
      header!.contains(host.querySelector('input[name="viewer_reading_width"]')),
    ).toBe(true);
    expect(
      header!.contains(
        host.querySelector('[role="group"][aria-label="Масштаб чтения"]'),
      ),
    ).toBe(true);
    expect(
      header!.querySelectorAll('input[name="viewer_display_mode"]'),
    ).toHaveLength(2);
    expect(
      header!.querySelectorAll('input[name="viewer_reading_width"]'),
    ).toHaveLength(2);
    expect(
      header!.querySelector('[role="group"][aria-label="Масштаб чтения"]'),
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
  it('commits an open draft when the selected entry changes', () => {
    render(makeEntry());

    const pencil = host.querySelector<HTMLButtonElement>(
      'button[aria-label="Редактировать заметку"]',
    );
    expect(pencil).not.toBeNull();
    act(() => pencil!.click());

    const textarea = host.querySelector<HTMLTextAreaElement>(
      'textarea[aria-label="Заметка к записи"]',
    );
    expect(textarea).not.toBeNull();
    act(() => {
      Object.getOwnPropertyDescriptor(
        HTMLTextAreaElement.prototype,
        'value',
      )?.set?.call(textarea, 'черновик A');
      textarea!.dispatchEvent(new Event('input', { bubbles: true }));
    });

    // `TextViewer` keys `EntryHeader` by entry id: switching entries unmounts
    // the editor without a blur, so the draft must be committed on the way
    // out instead of being lost with the component.
    render(
      makeEntry({
        id: 'entry-2',
        title: 'Другая запись',
        note: null,
        original_text: 'Текст второй записи',
      }),
    );

    expect(commands.setEntryNote).toHaveBeenCalledWith('entry-1', 'черновик A');
    expect(
      host.querySelector('textarea[aria-label="Заметка к записи"]'),
    ).toBeNull();
    expect(titleButton('Другая запись')).not.toBeNull();
  });
});
