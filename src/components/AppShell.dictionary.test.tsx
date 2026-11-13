// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import { ModalsProvider } from '@mantine/modals';
import { Notifications, notifications } from '@mantine/notifications';
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

// The Add button reads the clipboard through the Tauri plugin; the preview
// quick-add test drives that path (`preview_dialog_enabled` below), while the
// viewer quick-add tests never touch it.
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({
  readText: vi.fn().mockResolvedValue('Вызови getUserData через API'),
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

/** Second entry for the stale-capture tests: its own text also contains
 *  `getUserData`, so a stale selection would still look valid. */
const ENTRY_B: TextEntry = {
  ...ENTRY,
  id: 'entry-2',
  title: 'Вторая заметка',
  original_text: 'Позови getUserData снова',
};

const CONFIG = {
  theme: 'auto',
  language: 'ru',
  reading_width: 'wide',
  reading_scale: 100,
  preview_dialog_enabled: true,
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

function findButton(label: string): HTMLElement | null {
  return (
    [...document.querySelectorAll('button')].find(
      (candidate) => candidate.textContent?.trim() === label,
    ) ?? null
  );
}

function clickButton(label: string): void {
  const button = findButton(label);
  if (!button) throw new Error(`button ${label} not found`);
  act(() => button.click());
}

/** The dictionary editor dialog, once mounted. */
function dictionaryDialog(): HTMLElement | null {
  return (
    [...document.querySelectorAll<HTMLElement>('[role="dialog"]')].find(
      (candidate) => candidate.textContent?.includes('Словарь произношений'),
    ) ?? null
  );
}

/** Click a button inside the dictionary dialog: while the editor opens from
 *  Settings, the Settings modal (with its own «Сохранить») is still in the
 *  DOM during its exit transition. */
function clickDialogButton(label: string): void {
  const button = [
    ...(dictionaryDialog()?.querySelectorAll('button') ?? []),
  ].find((candidate) => candidate.textContent?.trim() === label);
  if (!button) throw new Error(`dialog button ${label} not found`);
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

/** Press Escape on the document body — how a Mantine modal is dismissed in a
 *  test (its window-level handler needs an element target). */
function pressEscape(): void {
  act(() => {
    document.body.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }),
    );
  });
}

/** Switch the selected entry as the queue would. */
function selectEntry(entry: TextEntry): void {
  act(() => {
    useSelectedEntry.setState({ selectedId: entry.id, selectedEntry: entry });
  });
}

/** Wait until the dictionary editor is fully unmounted (Mantine keeps the
 *  content through its exit transition). */
async function waitForEditorClosed(): Promise<void> {
  await vi.waitFor(() => {
    expect(
      [...document.querySelectorAll('label')].some(
        (label) => label.textContent?.trim() === 'Как произносить',
      ),
    ).toBe(false);
  });
}


describe('viewer dictionary quick-add', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(async () => {
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
    // Destructive commands the failed-regeneration test must not reach.
    commands.deleteAudio = vi.fn().mockResolvedValue(undefined);
    commands.deleteEntry = vi.fn().mockResolvedValue(undefined);
    // Settings renders alongside the viewer in the entry-point tests; its
    // sections read these on open.
    commands.getTerattsVoices = vi.fn().mockResolvedValue([]);
    commands.getCacheDir = vi.fn().mockResolvedValue('');
    commands.getLogDir = vi.fn().mockResolvedValue('');

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
          <Notifications />
        </MantineProvider>,
      );
    });
    // Flush the mount-time config load so the Add flow sees the preview gate.
    await act(async () => {
      await Promise.resolve();
    });
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
    document.body.innerHTML = '';
    notifications.clean();
    useSelectedEntry.setState({ selectedId: null, selectedEntry: null });
    window.getSelection()?.removeAllRanges();
  });

  /** Right-click `word` in the rendered viewer and wait for the menu. */
  async function openMenu(word: string): Promise<void> {
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
    // The dropdown mounts into the portal asynchronously; wait for its items
    // instead of sleeping for a fixed time.
    await vi.waitFor(() => {
      expect(menuItem('В словарь')).not.toBeNull();
    });
    expect(menuItems()).toEqual(['В словарь', 'Копировать']);
  }

  /** Right-click `word` in the rendered viewer and choose «В словарь». */
  async function openQuickAdd(word: string): Promise<void> {
    await openMenu(word);
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

  /** Open the dictionary editor through Settings, like the other entry point
   *  the offer must ignore. */
  async function openDictionaryFromSettings(): Promise<void> {
    const settingsButton = document.querySelector<HTMLElement>(
      '[aria-label="Открыть настройки"]',
    );
    expect(settingsButton, 'Settings button').not.toBeNull();
    act(() => settingsButton!.click());
    await vi.waitFor(() => {
      expect(
        document.querySelector('#settings-nav-dictionary'),
      ).not.toBeNull();
    });
    act(() =>
      document.querySelector<HTMLElement>('#settings-nav-dictionary')!.click(),
    );
    await vi.waitFor(() => {
      expect(findButton('Открыть…')).not.toBeNull();
    });
    clickButton('Открыть…');
    await vi.waitFor(() => {
      expect(document.body.textContent).toContain('Словарь произношений');
      // Settings closes for the editing session; wait for its unmount so its
      // own «Сохранить» cannot be picked up by the editor assertions.
      expect(document.querySelector('#settings-nav-dictionary')).toBeNull();
    });
  }

  /** Save one word through the blank add form (no viewer prefill). */
  async function saveFromBlankForm(): Promise<void> {
    clickDialogButton('Добавить слово');
    await vi.waitFor(() => {
      expect(
        [...document.querySelectorAll('label')].some(
          (label) => label.textContent?.trim() === 'Слово',
        ),
      ).toBe(true);
    });
    typeInto('Слово', 'замок');
    typeInto('Как произносить', 'зам+ок');
    clickDialogButton('Сохранить');
    await vi.waitFor(() => {
      expect(commands.saveDictionary).toHaveBeenCalledTimes(1);
    });
  }

  it('opens the editor prefilled from the selected word', async () => {
    await openQuickAdd('getUserData');

    expect(fieldValue('Слово')).toBe('getUserData');
    // The spoken form starts empty and the scope stays on the global
    // collection, exactly like the preview quick-add.
    expect(fieldValue('Как произносить')).toBe('');
    expect(fieldValue('Словарь')).toBe('Общий');
    // Opening the editor is a pure UI action: the player keeps going.
    expect(commands.pausePlayback).not.toHaveBeenCalled();
    expect(commands.stopPlayback).not.toHaveBeenCalled();
  });

  it('keeps the menu and editor in step with the selected entry', async () => {
    await openMenu('getUserData');
    selectEntry(ENTRY_B);

    // The menu captured entry A's selection: it must close instead of
    // offering a stale word that the AppShell would resolve against entry B.
    await vi.waitFor(() => {
      expect(menuItems()).toEqual([]);
    });
    expect(document.body.textContent).not.toContain('Словарь произношений');
    expect(fieldValue('Слово')).toBe('');
  });

  it('persists the entry and regenerates nothing when the editor closes', async () => {
    await openQuickAdd('getUserData');

    await saveWord('гет юзер дата');
    expect(commands.saveDictionary).toHaveBeenCalledWith('global', [
      { from: 'getUserData', to: 'гет юзер дата', forms: [] },
    ]);

    pressEscape();
    await waitForEditorClosed();

    // Closing the editor after a write is quiet: the recording stays stale
    // until the user regenerates it from the entry's context menu.
    expect(document.body.textContent).not.toContain('Переозвучить');
    expect(commands.regenerateEntry).not.toHaveBeenCalled();
    expect(commands.pausePlayback).not.toHaveBeenCalled();
    expect(commands.stopPlayback).not.toHaveBeenCalled();
    expect(commands.deleteAudio).not.toHaveBeenCalled();
    expect(commands.playEntry).not.toHaveBeenCalled();
  });

  it('closing the editor without a save regenerates nothing', async () => {
    await openQuickAdd('getUserData');

    pressEscape();
    await waitForEditorClosed();

    expect(commands.regenerateEntry).not.toHaveBeenCalled();
    expect(commands.saveDictionary).not.toHaveBeenCalled();
    expect(commands.pausePlayback).not.toHaveBeenCalled();
    expect(commands.stopPlayback).not.toHaveBeenCalled();
  });

  it('a save from Settings does not regenerate the recording', async () => {
    await openDictionaryFromSettings();
    await saveFromBlankForm();

    pressEscape();
    await vi.waitFor(() => {
      expect(
        document.querySelector('#settings-nav-dictionary'),
      ).not.toBeNull();
    });

    expect(commands.regenerateEntry).not.toHaveBeenCalled();
  });
  it('a save from the preview quick-add does not regenerate the recording', async () => {
    clickButton('Добавить');
    const pane = await vi.waitFor(() => {
      const element = [...document.querySelectorAll('pre')].find((candidate) =>
        candidate.textContent?.includes('getUserData'),
      );
      expect(element, 'preview source pane').not.toBeUndefined();
      return element as HTMLPreElement;
    });
    const walker = document.createTreeWalker(pane, NodeFilter.SHOW_TEXT);
    let node: Node | null = walker.nextNode();
    while (node !== null && !(node.textContent ?? '').includes('getUserData')) {
      node = walker.nextNode();
    }
    expect(node, 'word in the preview pane').not.toBeNull();
    const text = node!.textContent ?? '';
    const start = text.indexOf('getUserData');
    const range = document.createRange();
    range.setStart(node!, start);
    range.setEnd(node!, start + 'getUserData'.length);
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
    act(() => {
      pane.dispatchEvent(new MouseEvent('mouseup', { bubbles: true }));
    });
    await vi.waitFor(() => {
      const button = document.querySelector<HTMLButtonElement>(
        '[data-quick-add-dictionary]',
      );
      expect(button?.disabled).toBe(false);
    });
    act(() =>
      document
        .querySelector<HTMLElement>('[data-quick-add-dictionary]')!
        .click(),
    );
    await vi.waitFor(() => {
      expect(fieldValue('Слово')).toBe('getUserData');
    });

    await saveWord('гет юзер дата');
    pressEscape();
    await waitForEditorClosed();

    expect(commands.regenerateEntry).not.toHaveBeenCalled();
  });
});
