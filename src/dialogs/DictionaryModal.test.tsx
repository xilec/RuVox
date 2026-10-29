// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { MantineProvider } from '@mantine/core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../lib/tauri', () => ({
  commands: {
    getUserDictionary: vi.fn(() => Promise.resolve([])),
    saveUserDictionary: vi.fn(() => Promise.resolve()),
  },
}));

vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: vi.fn(() => Promise.resolve(() => {})),
  }),
}));

vi.mock('@mantine/notifications', () => ({
  notifications: { show: vi.fn() },
}));

vi.mock('@mantine/modals', () => ({
  modals: { openConfirmModal: vi.fn() },
}));

import { DictionaryModal } from './DictionaryModal';
import { translate } from '../lib/i18n';

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  container = document.createElement('div');
  document.body.append(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

async function renderModal() {
  act(() => {
    root.render(
      <MantineProvider>
        <DictionaryModal opened onClose={() => {}} />
      </MantineProvider>,
    );
  });
  // The dictionary load is a promise; let it settle before interacting.
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
}

/** Click a button by its visible label. */
function clickButton(label: string) {
  const button = [...document.querySelectorAll('button')].find(
    (b) => b.textContent?.trim() === label,
  );
  if (!button) throw new Error(`button ${label} not found`);
  act(() => button.click());
}

describe('DictionaryModal field help', () => {
  it('offers a help affordance on the spoken-form field', async () => {
    await renderModal();
    clickButton('Добавить слово');

    const help = document.querySelector('[aria-label="Справка об ударении"]');
    expect(help).not.toBeNull();
    expect(help?.textContent).toBe('?');
  });

  it('offers a help affordance on the word field', async () => {
    await renderModal();
    clickButton('Добавить слово');

    const help = document.querySelector('[aria-label="Справка о поиске слова"]');
    expect(help).not.toBeNull();
    expect(help?.textContent).toBe('?');
  });

  it('describes the mark and its engine dependency in the tooltip', () => {
    // The tooltip label is a plain string; asserting its wording keeps the
    // hint and the i18n key wired together without simulating a hover.
    const hint = translate('ru', 'dictionary.form.stress.hint');
    expect(hint).toContain('+');
    expect(hint).toContain('ударение');
    expect(hint).toContain('движками');
    expect(translate('en', 'dictionary.form.stress.hint')).toContain('+');
  });

  it('says that a Russian word is matched exactly', () => {
    expect(translate('ru', 'dictionary.form.from.hint')).toContain('точной форме');
    expect(translate('en', 'dictionary.form.from.hint')).toContain('exact form');
  });
});
