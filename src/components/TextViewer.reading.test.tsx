// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import type { ReadingWidth, TextEntry } from '../lib/tauri';

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
    readingWidth: string = 'narrow',
    readingScale = 100,
  ): void {
    act(() => {
      root.render(
        <MantineProvider>
          <TextViewer
            entry={entry}
            readingWidth={readingWidth as ReadingWidth}
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

  /** Drain the serialized persist chain and React's post-write re-render. */
  async function flush(): Promise<void> {
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
  }

  async function click(el: HTMLElement): Promise<void> {
    act(() => {
      el.click();
    });
    await flush();
  }

  it('applies the persisted width and scale to the content container', () => {
    renderWith(makeEntry(), 'wide', 150);

    expect(contentEl().getAttribute('data-reading-width')).toBe('wide');
    expect(cssVar('--ruvox-reading-scale')).toBe('1.5');
    // The visible controls reflect the persisted values.
    expect(host.textContent).toContain('150 %');
    expect(widthOption('wide').checked).toBe(true);
  });

  it('defaults to the narrow column at 100 % when no props are passed', () => {
    act(() => {
      root.render(
        <MantineProvider>
          <TextViewer entry={makeEntry()} />
        </MantineProvider>,
      );
    });

    expect(contentEl().getAttribute('data-reading-width')).toBe('narrow');
    expect(cssVar('--ruvox-reading-scale')).toBe('1');
    expect(host.textContent).toContain('100 %');
  });

  it('labels the width options «Узко» and «Широко»', () => {
    renderWith(makeEntry());

    expect(host.textContent).toContain('Узко');
    expect(host.textContent).toContain('Широко');
    const labelFor = (input: HTMLInputElement) =>
      host.querySelector<HTMLLabelElement>(`label[for="${input.id}"]`);
    expect(labelFor(widthOption('narrow'))?.textContent).toContain('Узко');
    expect(labelFor(widthOption('wide'))?.textContent).toContain('Широко');
  });

  it('steps the scale up the ladder and persists it', async () => {
    renderWith(makeEntry());

    await click(button('Увеличить масштаб чтения'));

    expect(updateConfig).toHaveBeenCalledWith({ reading_scale: 125 });
    expect(cssVar('--ruvox-reading-scale')).toBe('1.25');
    expect(host.textContent).toContain('125 %');
  });

  it('steps the scale down and persists it', async () => {
    renderWith(makeEntry(), 'narrow', 150);

    await click(button('Уменьшить масштаб чтения'));

    expect(updateConfig).toHaveBeenCalledWith({ reading_scale: 125 });
    expect(cssVar('--ruvox-reading-scale')).toBe('1.25');
  });

  it('steps an off-ladder 93 % down to 80 %', async () => {
    renderWith(makeEntry(), 'narrow', 93);

    expect(host.textContent).toContain('93 %');
    expect(cssVar('--ruvox-reading-scale')).toBe('0.93');

    await click(button('Уменьшить масштаб чтения'));

    expect(updateConfig).toHaveBeenCalledWith({ reading_scale: 80 });
    expect(cssVar('--ruvox-reading-scale')).toBe('0.8');
  });

  it('steps an off-ladder 93 % up to 100 %', async () => {
    renderWith(makeEntry(), 'narrow', 93);

    await click(button('Увеличить масштаб чтения'));

    expect(updateConfig).toHaveBeenCalledWith({ reading_scale: 100 });
    expect(cssVar('--ruvox-reading-scale')).toBe('1');
    expect(host.textContent).toContain('100 %');
  });

  it('resets the scale to 100 % with the reset button', async () => {
    renderWith(makeEntry(), 'narrow', 175);

    await click(button('Сбросить масштаб чтения до 100%'));

    expect(updateConfig).toHaveBeenCalledWith({ reading_scale: 100 });
    expect(cssVar('--ruvox-reading-scale')).toBe('1');
    expect(host.textContent).toContain('100 %');
  });

  it('exposes the current scale to assistive tech through a live region', () => {
    renderWith(makeEntry(), 'narrow', 150);

    const value = host.querySelector('[aria-live="polite"]');
    expect(value, 'live scale value').not.toBeNull();
    expect(value!.textContent).toBe('150 %');
    // The reset action carries its own name; the value is not hidden behind an
    // aria-label on the value element (#364 review).
    expect(button('Сбросить масштаб чтения до 100%').textContent?.trim()).toBe('↺');
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

  it('canonicalizes stale width values on seed', () => {
    renderWith(makeEntry(), 'Wide');
    expect(contentEl().getAttribute('data-reading-width')).toBe('narrow');
    expect(widthOption('narrow').checked).toBe(true);

    renderWith(makeEntry(), 'medium');
    expect(contentEl().getAttribute('data-reading-width')).toBe('narrow');
  });

  it('canonicalizes stale width values in the prop sync effect', () => {
    renderWith(makeEntry(), 'wide');
    expect(contentEl().getAttribute('data-reading-width')).toBe('wide');

    renderWith(makeEntry(), 'Wide');
    expect(contentEl().getAttribute('data-reading-width')).toBe('narrow');
    expect(widthOption('narrow').checked).toBe(true);
  });

  it('reverts an optimistic scale change and shows a toast on failure', async () => {
    updateConfig.mockRejectedValueOnce(new Error('storage failed'));
    renderWith(makeEntry());

    await click(button('Увеличить масштаб чтения'));

    expect(updateConfig).toHaveBeenCalledWith({ reading_scale: 125 });
    // Rolled back to the persisted value.
    expect(cssVar('--ruvox-reading-scale')).toBe('1');
    expect(host.textContent).toContain('100 %');
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

  it('keeps the newer scale when an older write rejects after it', async () => {
    let rejectFirst!: (reason: Error) => void;
    updateConfig
      .mockImplementationOnce(
        () =>
          new Promise<void>((_, reject) => {
            rejectFirst = reject;
          }),
      )
      .mockResolvedValueOnce(undefined);
    renderWith(makeEntry());

    await click(button('Увеличить масштаб чтения'));
    await click(button('Увеличить масштаб чтения'));

    expect(host.textContent).toContain('150 %');
    expect(cssVar('--ruvox-reading-scale')).toBe('1.5');

    act(() => {
      rejectFirst(new Error('storage failed'));
    });
    await flush();

    // The superseded write's rejection must not roll the UI back to 100 %.
    expect(cssVar('--ruvox-reading-scale')).toBe('1.5');
    expect(host.textContent).toContain('150 %');
    expect(updateConfig).toHaveBeenLastCalledWith({ reading_scale: 150 });
    expect(showNotification).not.toHaveBeenCalled();
  });

  it('makes rendered table scroll boxes keyboard-focusable regions', () => {
    renderWith(
      makeEntry({
        original_text: '| A | B |\n| --- | --- |\n| 1 | 2 |',
      }),
    );

    const box = host.querySelector<HTMLElement>('[data-scroll-x]');
    expect(box, 'table scroll box').not.toBeNull();
    expect(box!.getAttribute('tabindex')).toBe('0');
    expect(box!.getAttribute('role')).toBe('region');
    expect(box!.getAttribute('aria-label')).toBe(
      'Таблица с горизонтальной прокруткой',
    );
    expect(box!.querySelector('table')).not.toBeNull();
    expect(box!.querySelector('th')).not.toBeNull();
    expect(box!.querySelector('td')).not.toBeNull();
  });

  it('reports a persisted reading patch to the shell', async () => {
    const onReadingConfigChange = vi.fn();
    act(() => {
      root.render(
        <MantineProvider>
          <TextViewer
            entry={makeEntry()}
            onReadingConfigChange={onReadingConfigChange}
          />
        </MantineProvider>,
      );
    });

    await click(button('Увеличить масштаб чтения'));

    expect(onReadingConfigChange).toHaveBeenCalledWith({ reading_scale: 125 });
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

  it('re-syncs when only the reading props change for the same entry', () => {
    const entry = makeEntry();
    renderWith(entry, 'narrow', 100);
    expect(cssVar('--ruvox-reading-scale')).toBe('1');
    expect(contentEl().getAttribute('data-reading-width')).toBe('narrow');

    renderWith(entry, 'wide', 150);
    expect(contentEl().getAttribute('data-reading-width')).toBe('wide');
    expect(cssVar('--ruvox-reading-scale')).toBe('1.5');
    expect(host.textContent).toContain('150 %');
  });
});
