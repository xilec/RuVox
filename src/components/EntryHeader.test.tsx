// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import type { GenerationParams, TextEntry } from '../lib/tauri';

const { setEntryTitle, setEntryNote, showNotification } = vi.hoisted(() => ({
  setEntryTitle: vi.fn().mockResolvedValue(undefined),
  setEntryNote: vi.fn().mockResolvedValue(undefined),
  showNotification: vi.fn(),
}));

vi.mock('@mantine/notifications', () => ({
  notifications: { show: showNotification },
}));

vi.mock('../lib/tauri', () => ({
  commands: { setEntryTitle, setEntryNote },
}));

import { EntryHeader } from './EntryHeader';
import { useLocaleStore } from '../stores/locale';

const GENERATION: GenerationParams = {
  engine: 'silero_native',
  voice: 'xenia',
  sample_rate: 24000,
  model: null,
  app_version: '0.5.0',
  code_block_mode: 'brief',
  normalized_text_sha256: null,
  audio_codec: 'Ogg Opus',
  audio_bytes: 1024,
};

function makeEntry(overrides: Partial<TextEntry> = {}): TextEntry {
  return {
    id: 'entry-1',
    original_text: '## Установка\n\nТекст записи',
    normalized_text: null,
    status: 'ready',
    format: 'markdown',
    display_mode: null,
    html_source: null,
    source: null,
    title: 'Тёмная сторона инлайнинга',
    note: 'вернуться к бенчмаркам Wasm',
    created_at: '2026-11-02T07:03:00',
    audio_generated_at: '2026-11-02T07:04:00',
    audio_path: 'entry-1.opus',
    timestamps_path: null,
    duration_sec: 42,
    was_regenerated: false,
    generation_count: 1,
    generation: GENERATION,
    error_message: null,
    ...overrides,
  };
}

describe('EntryHeader', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    useLocaleStore.setState({ locale: 'ru' });
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    setEntryTitle.mockClear();
    setEntryTitle.mockResolvedValue(undefined);
    setEntryNote.mockClear();
    setEntryNote.mockResolvedValue(undefined);
    showNotification.mockClear();
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
  });

  function render(entry: TextEntry): void {
    act(() => {
      root.render(
        <MantineProvider>
          <EntryHeader entry={entry} />
        </MantineProvider>,
      );
    });
  }

  function buttonByLabel(label: string): HTMLButtonElement {
    const el = [...host.querySelectorAll<HTMLButtonElement>('button')].find(
      (b) => b.getAttribute('aria-label') === label,
    );
    if (!el) throw new Error(`button ${label} not found`);
    return el;
  }

  function buttonByText(text: string): HTMLButtonElement {
    const el = [...host.querySelectorAll<HTMLButtonElement>('button')].find(
      (b) => b.textContent?.trim() === text,
    );
    if (!el) throw new Error(`button ${text} not found`);
    return el;
  }

  function typeInto(
    field: HTMLInputElement | HTMLTextAreaElement,
    value: string,
  ): void {
    const proto =
      field instanceof HTMLTextAreaElement
        ? HTMLTextAreaElement.prototype
        : HTMLInputElement.prototype;
    act(() => {
      Object.getOwnPropertyDescriptor(proto, 'value')?.set?.call(field, value);
      field.dispatchEvent(new Event('input', { bubbles: true }));
    });
  }

  function keyDown(
    field: Element,
    key: string,
    modifiers: { ctrlKey?: boolean; metaKey?: boolean } = {},
  ): void {
    act(() => {
      field.dispatchEvent(
        new KeyboardEvent('keydown', {
          key,
          bubbles: true,
          cancelable: true,
          ...modifiers,
        }),
      );
    });
  }

  function titleInput(): HTMLInputElement {
    const el = host.querySelector<HTMLInputElement>(
      'input[aria-label="Заголовок записи"]',
    );
    expect(el).not.toBeNull();
    return el!;
  }

  function noteTextarea(): HTMLTextAreaElement {
    const el = host.querySelector<HTMLTextAreaElement>(
      'textarea[aria-label="Заметка к записи"]',
    );
    expect(el).not.toBeNull();
    return el!;
  }

  it('renders the stored title, note and metadata row', () => {
    render(makeEntry());

    expect(host.textContent).toContain('Тёмная сторона инлайнинга');
    expect(host.textContent).toContain('вернуться к бенчмаркам Wasm');
    expect(host.textContent).toContain('Готово');
    expect(host.textContent).toContain('Silero (нативный)');
    expect(host.textContent).toContain('Xenia');
    // The date is the local rendering of the stored UTC timestamp.
    expect(host.textContent).toContain('2026');
  });

  it('derives the title for legacy entries without one', () => {
    render(makeEntry({ title: null }));

    expect(host.textContent).toContain('Установка');
    // The accessible name is the displayed title, not a generic label.
    expect(buttonByLabel('Установка')).not.toBeNull();
  });

  it('shows explicit placeholders when no recording snapshot exists', () => {
    render(makeEntry({ generation: null, audio_generated_at: null }));

    expect(host.textContent).toContain('не озвучено');
    expect(host.textContent).toContain('—');
  });

  it('edits the title with Enter and shows the new value immediately', () => {
    render(makeEntry());

    act(() => buttonByLabel('Тёмная сторона инлайнинга').click());
    const input = titleInput();
    expect(input.value).toBe('Тёмная сторона инлайнинга');
    typeInto(input, '  Новое имя  ');
    keyDown(input, 'Enter');

    expect(setEntryTitle).toHaveBeenCalledWith('entry-1', 'Новое имя');
    expect(host.textContent).toContain('Новое имя');
    expect(
      host.querySelector('input[aria-label="Заголовок записи"]'),
    ).toBeNull();
  });

  it('saves the title on blur', () => {
    render(makeEntry());

    act(() => buttonByLabel('Тёмная сторона инлайнинга').click());
    const input = titleInput();
    input.focus();
    typeInto(input, 'Сохранено по blur');
    act(() => input.blur());

    expect(setEntryTitle).toHaveBeenCalledWith('entry-1', 'Сохранено по blur');
  });

  it('cancels title editing with Escape without calling IPC', () => {
    render(makeEntry());

    act(() => buttonByLabel('Тёмная сторона инлайнинга').click());
    const input = titleInput();
    typeInto(input, 'не сохранять');
    keyDown(input, 'Escape');

    expect(setEntryTitle).not.toHaveBeenCalled();
    expect(host.textContent).toContain('Тёмная сторона инлайнинга');
  });

  it('clears the title to null and falls back to the derived title', () => {
    render(makeEntry());

    act(() => buttonByLabel('Тёмная сторона инлайнинга').click());
    const input = titleInput();
    typeInto(input, '   ');
    keyDown(input, 'Enter');

    expect(setEntryTitle).toHaveBeenCalledWith('entry-1', null);
    expect(host.textContent).toContain('Установка');
  });

  it('rolls the title back and shows a red toast when IPC fails', async () => {
    setEntryTitle.mockRejectedValueOnce(new Error('boom'));
    render(makeEntry());

    act(() => buttonByLabel('Тёмная сторона инлайнинга').click());
    const input = titleInput();
    typeInto(input, 'Откатится');
    keyDown(input, 'Enter');

    expect(host.textContent).toContain('Откатится');
    await vi.waitFor(() => {
      expect(showNotification).toHaveBeenCalledWith(
        expect.objectContaining({ color: 'red' }),
      );
    });
    expect(host.textContent).not.toContain('Откатится');
    expect(host.textContent).toContain('Тёмная сторона инлайнинга');
  });

  it('offers the empty note state and saves the note with Ctrl+Enter', () => {
    render(makeEntry({ note: null }));

    act(() => buttonByText('Добавить заметку').click());
    const textarea = noteTextarea();
    typeInto(textarea, '  новая заметка\nвторая строка  ');
    keyDown(textarea, 'Enter', { ctrlKey: true });

    expect(setEntryNote).toHaveBeenCalledWith(
      'entry-1',
      'новая заметка\nвторая строка',
    );
    expect(host.textContent).toContain('новая заметка');
  });

  it('cancels note editing with Escape without calling IPC', () => {
    render(makeEntry());

    act(() => buttonByLabel('Редактировать заметку').click());
    const textarea = noteTextarea();
    typeInto(textarea, 'не сохранять');
    keyDown(textarea, 'Escape');

    expect(setEntryNote).not.toHaveBeenCalled();
    expect(host.textContent).toContain('вернуться к бенчмаркам Wasm');
  });

  it('keeps an open note draft when the entry updates (status change)', () => {
    render(makeEntry());
    act(() => buttonByLabel('Редактировать заметку').click());
    const textarea = noteTextarea();
    typeInto(textarea, 'черновик заметки');

    render(makeEntry({ status: 'processing', note: 'вернуться к бенчмаркам Wasm' }));

    expect(noteTextarea().value).toBe('черновик заметки');
  });

  it('keeps an open title draft when the entry updates (status change)', () => {
    render(makeEntry());
    act(() => buttonByLabel('Тёмная сторона инлайнинга').click());
    const input = titleInput();
    typeInto(input, 'черновик заголовка');

    render(makeEntry({ status: 'processing' }));

    expect(titleInput().value).toBe('черновик заголовка');
  });

  it('applies an incoming note when no draft is open', () => {
    render(makeEntry());
    render(makeEntry({ note: 'заметка извне' }));

    expect(host.textContent).toContain('заметка извне');
  });

  it('expands a clamped note and collapses it again', () => {
    // jsdom has no layout: make the clamp report hidden overflow.
    Object.defineProperty(HTMLElement.prototype, 'scrollHeight', {
      configurable: true,
      get: () => 100,
    });
    Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
      configurable: true,
      get: () => 20,
    });
    try {
      render(makeEntry({ note: 'первая\nвторая\nтретья\nчетвёртая' }));

      const expand = buttonByLabel('Показать полностью');
      act(() => expand.click());
      expect(buttonByLabel('Свернуть')).not.toBeNull();

      act(() => buttonByLabel('Свернуть').click());
      expect(buttonByLabel('Показать полностью')).not.toBeNull();
    } finally {
      delete (HTMLElement.prototype as { scrollHeight?: number }).scrollHeight;
      delete (HTMLElement.prototype as { clientHeight?: number }).clientHeight;
    }
  });
});
