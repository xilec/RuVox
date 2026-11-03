// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import type { TextEntry } from '../lib/tauri';

const { updateConfig, showNotification } = vi.hoisted(() => ({
  updateConfig: vi.fn().mockResolvedValue(undefined),
  showNotification: vi.fn(),
}));

vi.mock('@mantine/notifications', () => ({
  notifications: { show: showNotification },
}));

vi.mock('../lib/tauri', () => ({
  commands: {
    getTimestamps: vi.fn().mockResolvedValue([]),
    setEntryDisplayMode: vi.fn().mockResolvedValue(undefined),
    updateConfig,
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

describe('TextViewer reading typography', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    updateConfig.mockClear();
    updateConfig.mockResolvedValue(undefined);
    showNotification.mockClear();
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
  });

  function renderWith(
    entry: TextEntry,
    readingWidth: 'narrow' | 'wide' = 'narrow',
    readingScale = 100,
  ): void {
    act(() => {
      root.render(
        <MantineProvider>
          <TextViewer
            entry={entry}
            readingWidth={readingWidth}
            readingScale={readingScale}
          />
        </MantineProvider>,
      );
    });
  }

  function contentEl(): HTMLElement {
    const el = host.querySelector<HTMLElement>('[data-reading-width]');
    expect(el, 'content container rendered').not.toBeNull();
    return el!;
  }

  /** Inline custom property, read from the style attribute so the assertion
   *  does not depend on jsdom's CSSStyleDeclaration support. */
  function cssVar(name: string): string {
    const style = contentEl().getAttribute('style') ?? '';
    const match = new RegExp(`${name}:\\s*([^;]+)`).exec(style);
    return match?.[1]?.trim() ?? '';
  }

  function button(label: string): HTMLButtonElement {
    const el = [...host.querySelectorAll<HTMLButtonElement>('button')].find(
      (b) => b.getAttribute('aria-label') === label,
    );
    expect(el, `button ${label}`).not.toBeUndefined();
    return el!;
  }

  function widthOption(value: 'narrow' | 'wide'): HTMLInputElement {
    const el = host.querySelector<HTMLInputElement>(
      `input[name="viewer_reading_width"][value="${value}"]`,
    );
    expect(el, `width option ${value}`).not.toBeNull();
    return el!;
  }

  async function click(el: HTMLElement): Promise<void> {
    await act(async () => {
      el.click();
      await Promise.resolve();
    });
  }

  it('applies the persisted width and scale to the content container', () => {
    renderWith(makeEntry(), 'wide', 150);

    expect(contentEl().getAttribute('data-reading-width')).toBe('wide');
    expect(cssVar('--ruvox-reading-scale')).toBe('1.5');
    // The visible controls reflect the persisted values.
    expect(host.textContent).toContain('150%');
    expect(widthOption('wide').checked).toBe(true);
  });

  it('defaults to the narrow column at 100% when no props are passed', () => {
    act(() => {
      root.render(
        <MantineProvider>
          <TextViewer entry={makeEntry()} />
        </MantineProvider>,
      );
    });

    expect(contentEl().getAttribute('data-reading-width')).toBe('narrow');
    expect(cssVar('--ruvox-reading-scale')).toBe('1');
    expect(host.textContent).toContain('100%');
  });

  it('steps the scale up the ladder and persists it', async () => {
    renderWith(makeEntry());

    await click(button('Увеличить масштаб чтения'));

    expect(updateConfig).toHaveBeenCalledWith({ reading_scale: 125 });
    expect(cssVar('--ruvox-reading-scale')).toBe('1.25');
    expect(host.textContent).toContain('125%');
  });

  it('steps the scale down and persists it', async () => {
    renderWith(makeEntry(), 'narrow', 150);

    await click(button('Уменьшить масштаб чтения'));

    expect(updateConfig).toHaveBeenCalledWith({ reading_scale: 125 });
    expect(cssVar('--ruvox-reading-scale')).toBe('1.25');
  });

  it('resets the scale to 100% when the percentage is clicked', async () => {
    renderWith(makeEntry(), 'narrow', 175);

    await click(button('Сбросить масштаб чтения до 100%'));

    expect(updateConfig).toHaveBeenCalledWith({ reading_scale: 100 });
    expect(cssVar('--ruvox-reading-scale')).toBe('1');
    expect(host.textContent).toContain('100%');
  });

  it('disables the bounds and leaves the ladder ends reachable', () => {
    renderWith(makeEntry(), 'narrow', 200);
    expect(button('Увеличить масштаб чтения').disabled).toBe(true);
    expect(button('Уменьшить масштаб чтения').disabled).toBe(false);

    renderWith(makeEntry(), 'narrow', 80);
    expect(button('Уменьшить масштаб чтения').disabled).toBe(true);
    expect(button('Увеличить масштаб чтения').disabled).toBe(false);
  });

  it('switches the column width and persists it', async () => {
    renderWith(makeEntry());

    await click(widthOption('wide'));

    expect(updateConfig).toHaveBeenCalledWith({ reading_width: 'wide' });
    expect(contentEl().getAttribute('data-reading-width')).toBe('wide');
  });

  it('reverts an optimistic scale change and shows a toast on failure', async () => {
    updateConfig.mockRejectedValueOnce(new Error('storage failed'));
    renderWith(makeEntry());

    await click(button('Увеличить масштаб чтения'));

    expect(updateConfig).toHaveBeenCalledWith({ reading_scale: 125 });
    // Rolled back to the persisted value.
    expect(cssVar('--ruvox-reading-scale')).toBe('1');
    expect(host.textContent).toContain('100%');
    expect(showNotification).toHaveBeenCalledWith(
      expect.objectContaining({ color: 'red' }),
    );
  });

  it('reverts an optimistic width change and shows a toast on failure', async () => {
    updateConfig.mockRejectedValueOnce(new Error('storage failed'));
    renderWith(makeEntry());

    await click(widthOption('wide'));

    expect(contentEl().getAttribute('data-reading-width')).toBe('narrow');
    expect(widthOption('narrow').checked).toBe(true);
    expect(showNotification).toHaveBeenCalledWith(
      expect.objectContaining({ color: 'red' }),
    );
  });

  it('keeps the controls for a plain source without a display switch', () => {
    renderWith(
      makeEntry({ format: 'plain', original_text: 'Просто заметка' }),
    );

    expect(host.querySelectorAll('input[name="viewer_display_mode"]')).toHaveLength(
      0,
    );
    expect(host.querySelectorAll('input[name="viewer_reading_width"]')).toHaveLength(
      2,
    );
    expect(
      host.querySelector('[role="group"][aria-label="Масштаб чтения"]'),
    ).not.toBeNull();
  });

  it('re-syncs from new config props after Settings saves', () => {
    renderWith(makeEntry(), 'narrow', 100);
    expect(cssVar('--ruvox-reading-scale')).toBe('1');

    renderWith(makeEntry({ id: 'entry-2' }), 'wide', 150);
    expect(contentEl().getAttribute('data-reading-width')).toBe('wide');
    expect(cssVar('--ruvox-reading-scale')).toBe('1.5');
  });
});
