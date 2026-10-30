// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { MantineProvider } from '@mantine/core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../lib/tauri', () => ({
  commands: {
    getDictionaries: vi.fn(() =>
      Promise.resolve([{ scope: 'global', name: 'Global', entries: [] }]),
    ),
    saveDictionary: vi.fn(() => Promise.resolve()),
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
import { commands } from '../lib/tauri';

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
  // The dictionary load is a promise; wait for the form that follows it.
  await vi.waitFor(() => {
    expect(document.body.textContent).toContain('Добавить слово');
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

/** Type into the input whose label is exactly `label`. */
function typeInto(label: string, value: string) {
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

describe('DictionaryModal field help', () => {
  it('offers a scope selector and loads all collections', async () => {
    await renderModal();
    expect(commands.getDictionaries).toHaveBeenCalled();
    expect(document.body.textContent).toContain(
      translate('ru', 'dictionary.scope.label'),
    );
    expect(document.body.textContent).toContain(
      translate('ru', 'dictionary.scope.global'),
    );
  });

  it('offers a help affordance on the spoken-form field', async () => {
    await renderModal();
    clickButton('Добавить слово');

    const help = document.querySelector('[aria-label="Справка об ударении"]');
    expect(help).not.toBeNull();
    expect(help?.textContent).toBe('?');
  });

  it('offers a help affordance on the word-forms field', async () => {
    await renderModal();
    clickButton('Добавить слово');

    const help = document.querySelector('[aria-label="Справка о формах слова"]');
    expect(help).not.toBeNull();
    expect(help?.textContent).toBe('?');
    expect(document.body.textContent).toContain(
      translate('ru', 'dictionary.form.forms.description'),
    );
    expect(document.body.textContent).toContain(
      translate('ru', 'dictionary.form.forms.details.open'),
    );
  });

  it('saves listed forms with the entry', async () => {
    await renderModal();
    clickButton('Добавить слово');
    typeInto('Слово', 'замок');
    typeInto('Как произносить', 'зам+ок');
    typeInto('Формы слова', 'замка, замке');
    clickButton('Сохранить');

    await vi.waitFor(() => {
      expect(commands.saveDictionary).toHaveBeenCalledWith('global', [
        { from: 'замок', to: 'зам+ок', forms: ['замка', 'замке'] },
      ]);
    });
  });

  it('shows the inline error for a form on a Latin entry', async () => {
    await renderModal();
    clickButton('Добавить слово');
    typeInto('Слово', 'docker');
    typeInto('Как произносить', 'докер');
    typeInto('Формы слова', 'dockers');

    await vi.waitFor(() => {
      expect(document.body.textContent).toContain(
        translate('ru', 'dictionary.form.error.forms.requires_cyrillic'),
      );
    });
  });

  it('offers a help affordance on the word field', async () => {
    await renderModal();
    clickButton('Добавить слово');

    const help = document.querySelector('[aria-label="Справка о поиске слова"]');
    expect(help).not.toBeNull();
    expect(help?.textContent).toBe('?');
  });

  it('shows the inline error for a malformed mark', async () => {
    await renderModal();
    clickButton('Добавить слово');
    typeInto('Слово', 'moskva');
    typeInto('Как произносить', 'Моск+ва');

    await vi.waitFor(() => {
      expect(document.body.textContent).toContain(translate('ru', 'dictionary.form.error.to.stress_malformed'));
    });
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
