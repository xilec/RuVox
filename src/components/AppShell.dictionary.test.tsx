// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import { ModalsProvider } from '@mantine/modals';
import type { TextEntry } from '../lib/tauri';

/** Command/event stubs shared with the `../lib/tauri` mock below. The proxy
 *  materializes a resolving stub for any command the mounted tree calls, so
 *  the test only has to spell out the handful whose results or calls it
 *  asserts on. */
const { commands, events } = vi.hoisted(() => ({
  commands: {} as Record<string, ReturnType<typeof vi.fn>>,
  events: {} as Record<string, ReturnType<typeof vi.fn>>,
}));

vi.mock('../lib/tauri', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/tauri')>();
  const lazy = (
    target: Record<string, unknown>,
    fallback: () => unknown,
  ): Record<string, unknown> =>
    new Proxy(target, {
      get(obj, prop) {
        if (typeof prop !== 'string') return undefined;
        if (!(prop in obj)) obj[prop] = vi.fn().mockImplementation(fallback);
        return obj[prop];
      },
    });
  return {
    ...actual,
    commands: lazy(commands, () => Promise.resolve(undefined)),
    events: lazy(events, () => Promise.resolve(() => {})),
  };
});

vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: vi.fn().mockResolvedValue(() => {}),
  }),
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

import { AppShell } from './AppShell';
import { useSelectedEntry } from '../stores/selectedEntry';

const ENTRY: TextEntry = {
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
};

const CONFIG = {
  theme: 'auto',
  language: 'ru',
  reading_width: 'wide',
  reading_scale: 100,
  preview_dialog_enabled: false,
  engine: 'piper',
  speaker: 'xenia',
  sample_rate: 24000,
  speech_rate: 1,
  notify_on_ready: false,
  notify_on_error: false,
  text_format: 'plain',
  max_cache_size_mb: 512,
  code_block_mode: 'brief',
  english_reading: 'auto',
  player_hotkeys: {},
  window_geometry: null,
  hide_to_tray_on_close: false,
  piper_voice: 'ruslan',
  teratts_voice: 'ru_m5',
};

const ENGINES = {
  piper: { available: true, reason: null },
  silero: { available: true, reason: null },
  silero_native: { available: true, reason: null },
  teratts: { available: true, reason: null },
};

function fieldValue(label: string): string {
  const labelEl = [...document.querySelectorAll('label')].find(
    (candidate) => candidate.textContent?.trim() === label,
  );
  const field = labelEl?.htmlFor
    ? (document.getElementById(labelEl.htmlFor) as HTMLInputElement | null)
    : null;
  return field?.value ?? '';
}

function typeInto(label: string, value: string): void {
  const labelEl = [...document.querySelectorAll('label')].find(
    (candidate) => candidate.textContent?.trim() === label,
  );
  const field = labelEl?.htmlFor
    ? (document.getElementById(labelEl.htmlFor) as HTMLInputElement | null)
    : null;
  if (!field) throw new Error(`field ${label} not found`);
  act(() => {
    const setValue = Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      'value',
    )?.set;
    setValue?.call(field, value);
    field.dispatchEvent(new Event('input', { bubbles: true }));
  });
}

function clickButton(label: string): void {
  const button = [...document.querySelectorAll('button')].find(
    (candidate) => candidate.textContent?.trim() === label,
  );
  if (!button) throw new Error(`button ${label} not found`);
  act(() => button.click());
}

function menuItem(label: string): HTMLElement | null {
  return (
    ([...document.body.querySelectorAll('[role="menuitem"]')].find(
      (el) => el.textContent === label,
    ) as HTMLElement | undefined) ?? null
  );
}

function menuItems(): string[] {
  return Array.from(document.body.querySelectorAll('[role="menuitem"]')).map(
    (el) => el.textContent ?? '',
  );
}

async function flushAsync(ms = 50): Promise<void> {
  await act(async () => {
    await new Promise((r) => setTimeout(r, ms));
  });
}

/** Press Escape on the document body — how a Mantine modal is dismissed in a
 *  test (its window-level handler needs an element target). */
function pressEscape(): void {
  act(() => {
    document.body.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }),
    );
  });
}

describe('viewer dictionary quick-add', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    for (const key of Object.keys(commands)) delete commands[key];
    commands.getConfig = vi.fn().mockResolvedValue(CONFIG);
    commands.getAvailableEngines = vi.fn().mockResolvedValue(ENGINES);
    commands.getEntries = vi.fn().mockResolvedValue([]);
    commands.getDictionaries = vi
      .fn()
      .mockResolvedValue([{ scope: 'global', name: 'Global', entries: [] }]);
    commands.saveDictionary = vi.fn().mockResolvedValue(undefined);
    commands.regenerateEntry = vi.fn().mockResolvedValue(undefined);
    commands.getTimestamps = vi.fn().mockResolvedValue([]);
    commands.setEntryDisplayMode = vi.fn().mockResolvedValue(undefined);
    commands.pausePlayback = vi.fn().mockResolvedValue(undefined);
    commands.stopPlayback = vi.fn().mockResolvedValue(undefined);
    commands.playEntry = vi.fn().mockResolvedValue(undefined);

    useSelectedEntry.setState({
      selectedId: ENTRY.id,
      selectedEntry: ENTRY,
    });
    window.getSelection()?.removeAllRanges();

    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    act(() => {
      root.render(
        <MantineProvider>
          <ModalsProvider>
            <AppShell />
          </ModalsProvider>
        </MantineProvider>,
      );
    });
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
    document.body.innerHTML = '';
    useSelectedEntry.setState({ selectedId: null, selectedEntry: null });
    window.getSelection()?.removeAllRanges();
  });

  /** Right-click `word` in the rendered viewer and choose «В словарь». */
  async function openQuickAdd(word: string): Promise<void> {
    const container = host.querySelector('[data-reading-width]');
    expect(container, 'viewer content container').not.toBeNull();
    const selection = window.getSelection();
    selection?.removeAllRanges();
    const walker = document.createTreeWalker(
      container as Node,
      NodeFilter.SHOW_TEXT,
    );
    let node: Node | null = walker.nextNode();
    let target: HTMLElement | null = null;
    while (node !== null) {
      const text = node.textContent ?? '';
      const start = text.indexOf(word);
      if (start >= 0) {
        const range = document.createRange();
        range.setStart(node, start);
        range.setEnd(node, start + word.length);
        selection?.addRange(range);
        target = node.parentElement;
        break;
      }
      node = walker.nextNode();
    }
    expect(target, `word ${word} in the viewer`).not.toBeNull();

    act(() => {
      target!.dispatchEvent(
        new MouseEvent('contextmenu', {
          bubbles: true,
          cancelable: true,
          clientX: 10,
          clientY: 10,
        }),
      );
    });
    await flushAsync();
    expect(menuItems()).toEqual(['В словарь', 'Копировать']);

    act(() => menuItem('В словарь')!.click());
    await vi.waitFor(() => {
      expect(fieldValue('Слово')).toBe(word);
    });
  }

  async function saveWord(to: string): Promise<void> {
    typeInto('Как произносить', to);
    clickButton('Сохранить');
    await vi.waitFor(() => {
      expect(commands.saveDictionary).toHaveBeenCalledTimes(1);
    });
  }

  it('opens the editor prefilled from the selected word', async () => {
    await openQuickAdd('getUserData');

    expect(fieldValue('Слово')).toBe('getUserData');
    // Opening the editor is a pure UI action: the player keeps going.
    expect(commands.pausePlayback).not.toHaveBeenCalled();
    expect(commands.stopPlayback).not.toHaveBeenCalled();
  });

  it('persists the entry and offers a regeneration only after the save', async () => {
    await openQuickAdd('getUserData');
    // No prompt while the editor is still open, and none before a write.
    expect(document.body.textContent).not.toContain('Переозвучить сейчас?');

    await saveWord('гет юзер дата');
    expect(commands.saveDictionary).toHaveBeenCalledWith('global', [
      { from: 'getUserData', to: 'гет юзер дата', forms: [] },
    ]);

    pressEscape();
    await vi.waitFor(() => {
      expect(document.body.textContent).toContain('Переозвучить сейчас?');
    });
    // Closing the editor does not regenerate by itself.
    expect(commands.regenerateEntry).not.toHaveBeenCalled();
  });

  it('regenerates the entry only after the offer is confirmed', async () => {
    await openQuickAdd('getUserData');
    await saveWord('гет юзер дата');
    pressEscape();
    await vi.waitFor(() => {
      expect(document.body.textContent).toContain('Переозвучить сейчас?');
    });

    clickButton('Переозвучить');
    await vi.waitFor(() => {
      expect(commands.regenerateEntry).toHaveBeenCalledWith(ENTRY.id, false);
    });
    expect(commands.regenerateEntry).toHaveBeenCalledTimes(1);
    // The regeneration itself (backend) stops playback; the prompt does not
    // issue a separate player command.
    expect(commands.stopPlayback).not.toHaveBeenCalled();
  });

  it('declining the offer changes nothing', async () => {
    await openQuickAdd('getUserData');
    await saveWord('гет юзер дата');
    pressEscape();
    await vi.waitFor(() => {
      expect(document.body.textContent).toContain('Переозвучить сейчас?');
    });

    clickButton('Не сейчас');
    // The modal keeps its content while the exit transition runs.
    await vi.waitFor(() => {
      expect(document.body.textContent).not.toContain('Переозвучить сейчас?');
    });
    expect(commands.regenerateEntry).not.toHaveBeenCalled();
  });

  it('closing the editor without a save raises no offer', async () => {
    await openQuickAdd('getUserData');

    pressEscape();
    await flushAsync(300);

    expect(document.body.textContent).not.toContain('Переозвучить сейчас?');
    expect(commands.regenerateEntry).not.toHaveBeenCalled();
    expect(commands.saveDictionary).not.toHaveBeenCalled();
  });
});
