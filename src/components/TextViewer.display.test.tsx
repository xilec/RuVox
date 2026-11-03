// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import type { TextEntry } from '../lib/tauri';

const { setEntryDisplayMode } = vi.hoisted(() => ({
  setEntryDisplayMode: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('../lib/tauri', () => ({
  commands: {
    getTimestamps: vi.fn().mockResolvedValue([]),
    setEntryDisplayMode,
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

const MARKDOWN_TEXT = '## Установка\n\nЗапусти `pnpm install`.';

function makeEntry(overrides: Partial<TextEntry> = {}): TextEntry {
  return {
    id: 'entry-1',
    original_text: MARKDOWN_TEXT,
    normalized_text: null,
    status: 'ready',
    format: 'markdown',
    display_mode: null,
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

describe('TextViewer display modes', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    setEntryDisplayMode.mockClear();
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

  function controlLabels(): string[] {
    return Array.from(host.querySelectorAll('label')).map(
      (label) => label.textContent ?? '',
    );
  }

  it('offers Markdown and Исходный for a markdown source and renders it', () => {
    renderWith(makeEntry());

    expect(controlLabels()).toEqual(['Markdown', 'Исходный']);
    expect(host.textContent).not.toContain('HTML');
    expect(host.querySelector('h2')?.textContent).toBe('Установка');
    expect(host.querySelector('code')?.textContent).toBe('pnpm install');
  });

  it('shows the verbatim markup and persists the Исходный choice', async () => {
    renderWith(makeEntry());

    const plainOption = host.querySelector<HTMLInputElement>('input[value="plain"]');
    expect(plainOption).not.toBeNull();
    await act(async () => {
      plainOption!.click();
      await Promise.resolve();
    });

    expect(host.querySelector('h2')).toBeNull();
    expect(host.textContent).toContain('## Установка');
    expect(setEntryDisplayMode).toHaveBeenCalledWith('entry-1', 'plain');

    // The backend echoes the persisted choice through entry_updated; the
    // viewer must keep the verbatim view instead of resetting to rendered.
    renderWith(makeEntry({ display_mode: 'plain' }));
    expect(host.querySelector('h2')).toBeNull();
    expect(host.textContent).toContain('## Установка');
  });

  it('returns to the rendered view and persists it', async () => {
    renderWith(makeEntry({ display_mode: 'plain' }));
    expect(host.querySelector('h2')).toBeNull();

    const renderedOption = host.querySelector<HTMLInputElement>(
      'input[value="rendered"]',
    );
    expect(renderedOption).not.toBeNull();
    await act(async () => {
      renderedOption!.click();
      await Promise.resolve();
    });

    expect(host.querySelector('h2')?.textContent).toBe('Установка');
    expect(setEntryDisplayMode).toHaveBeenCalledWith('entry-1', 'rendered');
  });

  it('offers HTML and Исходный for an html source and renders stored markup', () => {
    renderWith(
      makeEntry({
        format: 'html',
        original_text: 'ссылка',
        html_source: '<p><a href="/x">ссылка</a></p>',
      }),
    );

    expect(controlLabels()).toEqual(['HTML', 'Исходный']);
    expect(host.querySelector('h2')).toBeNull();
    expect(host.querySelector('a')?.getAttribute('href')).toBe('/x');
  });

  it('shows no control for a plain source and renders the text verbatim', () => {
    renderWith(
      makeEntry({
        format: 'plain',
        original_text: '**жирный** и `код`',
      }),
    );

    expect(controlLabels()).toEqual([]);
    expect(host.querySelectorAll('input[type="radio"]')).toHaveLength(0);
    expect(host.textContent).toContain('**жирный** и `код`');
    expect(host.querySelector('strong')).toBeNull();
    expect(host.querySelector('code')).toBeNull();
  });

  it('derives Markdown for a legacy entry without a source format', () => {
    renderWith(makeEntry({ format: null }));

    expect(controlLabels()).toEqual(['Markdown', 'Исходный']);
    expect(host.querySelector('h2')?.textContent).toBe('Установка');
  });

  it('shows no control for a legacy plain-text entry', () => {
    renderWith(makeEntry({ format: null, original_text: 'Просто заметка' }));

    expect(controlLabels()).toEqual([]);
    expect(host.textContent).toContain('Просто заметка');
  });

  it('clamps a stale html format without stored markup to the detected markdown', () => {
    renderWith(makeEntry({ format: 'html', html_source: null }));

    expect(controlLabels()).toEqual(['Markdown', 'Исходный']);
    expect(host.textContent).not.toContain('HTML');
    expect(host.querySelector('h2')?.textContent).toBe('Установка');
  });

  it('restores the persisted choice when switching entries', () => {
    const plainEntry = makeEntry({ id: 'entry-plain', display_mode: 'plain' });
    const renderedEntry = makeEntry({
      id: 'entry-rendered',
      original_text: '## Другой раздел',
    });

    renderWith(plainEntry);
    expect(host.querySelector('h2')).toBeNull();

    renderWith(renderedEntry);
    expect(host.querySelector('h2')?.textContent).toBe('Другой раздел');

    renderWith(plainEntry);
    expect(host.querySelector('h2')).toBeNull();
    expect(host.textContent).toContain('## Установка');
  });

  it('reverts the optimistic switch when persisting fails', async () => {
    setEntryDisplayMode.mockRejectedValueOnce(new Error('storage failed'));
    renderWith(makeEntry());

    const plainOption = host.querySelector<HTMLInputElement>('input[value="plain"]');
    await act(async () => {
      plainOption!.click();
      await Promise.resolve();
    });

    expect(host.querySelector('h2')?.textContent).toBe('Установка');
    expect(host.textContent).not.toContain('## Установка');
  });

  it('keeps an optimistic switch when an unrelated entry update arrives', async () => {
    let resolvePersist: () => void = () => {};
    setEntryDisplayMode.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          resolvePersist = resolve;
        }),
    );
    renderWith(makeEntry());

    const plainOption = host.querySelector<HTMLInputElement>('input[value="plain"]');
    await act(async () => {
      plainOption!.click();
      await Promise.resolve();
    });
    expect(host.querySelector('h2')).toBeNull();

    // entry_updated with the same display_mode (null) but a changed status:
    // only the derived view fields may reset the optimistic switch.
    renderWith(makeEntry({ status: 'playing' }));
    expect(host.querySelector('h2')).toBeNull();
    expect(host.textContent).toContain('## Установка');

    // Persistence resolves later and the echoed entry confirms the choice.
    await act(async () => {
      resolvePersist();
      await Promise.resolve();
    });
    renderWith(makeEntry({ display_mode: 'plain', status: 'ready' }));
    expect(host.querySelector('h2')).toBeNull();
    expect(setEntryDisplayMode).toHaveBeenCalledWith('entry-1', 'plain');
  });
});
