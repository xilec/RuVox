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
    // The preview controls resolve the serving engine on open (issue #349);
    // Piper is the active engine in these tests.
    getConfig: vi.fn(() =>
      Promise.resolve({
        engine: 'piper',
        piper_voice: 'ruslan',
        speaker: 'aidar',
        teratts_voice: 'ru_m5',
      }),
    ),
    getAvailableEngines: vi.fn(() =>
      Promise.resolve({
        piper: { available: true, reason: null, reads_latin_natively: false },
        silero: { available: false, reason: null, reads_latin_natively: false },
        silero_native: { available: false, reason: null, reads_latin_natively: false },
        teratts: { available: false, reason: null, reads_latin_natively: false },
      }),
    ),
    previewPronunciation: vi.fn(() =>
      Promise.resolve({ spoken_text: 'тест', wav_base64: 'UklGRg==', duration_sec: 0.4 }),
    ),
    pausePlayback: vi.fn(() => Promise.resolve()),
    stopPlayback: vi.fn(() => Promise.resolve()),
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
  // jsdom has no media pipeline and no blob URLs: the preview control is
  // tested through these two seams (issue #349).
  HTMLMediaElement.prototype.play = vi.fn(() => Promise.resolve());
  HTMLMediaElement.prototype.pause = vi.fn();
  URL.createObjectURL = vi.fn(() => 'blob:preview-audio');
  URL.revokeObjectURL = vi.fn();
  vi.mocked(commands.previewPronunciation).mockClear();
  vi.mocked(commands.pausePlayback).mockClear();
  vi.mocked(commands.stopPlayback).mockClear();
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
  vi.mocked(HTMLMediaElement.prototype.play).mockClear();
});

interface RenderOptions {
  initialFrom?: string;
  onInitialFromConsumed?: () => void;
  onSaved?: () => void;
}

async function renderModal(options: RenderOptions = {}) {
  act(() => {
    root.render(
      <MantineProvider>
        <DictionaryModal opened onClose={() => {}} {...options} />
      </MantineProvider>,
    );
  });
  // The dictionary load is a promise; wait for the form that follows it.
  await vi.waitFor(() => {
    expect(document.body.textContent).toContain('Добавить слово');
  });
  // The serving-engine probe (getConfig + getAvailableEngines) settles right
  // after the first paint; flush it inside act so its state update is covered.
  await act(async () => {
    await Promise.resolve();
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

/** Read the current value of the input labelled `label`. */
function fieldValue(label: string): string {
  const labelEl = [...document.querySelectorAll('label')].find(
    (candidate) => candidate.textContent?.trim() === label,
  );
  const field = labelEl?.htmlFor
    ? (document.getElementById(labelEl.htmlFor) as HTMLInputElement | null)
    : null;
  return field?.value ?? '';
}

describe('DictionaryModal quick-add', () => {
  it('prefills the add form from initialFrom and consumes it once', async () => {
    const onInitialFromConsumed = vi.fn();
    await renderModal({ initialFrom: 'getUserData', onInitialFromConsumed });

    await vi.waitFor(() => {
      expect(fieldValue('Слово')).toBe('getUserData');
    });
    expect(onInitialFromConsumed).toHaveBeenCalledTimes(1);
  });

  it('reports a successful save through onSaved', async () => {
    const onSaved = vi.fn();
    vi.mocked(commands.saveDictionary).mockClear();
    await renderModal({ onSaved });
    clickButton('Добавить слово');
    typeInto('Слово', 'замок');
    typeInto('Как произносить', 'зам+ок');
    clickButton('Сохранить');

    await vi.waitFor(() => expect(onSaved).toHaveBeenCalledTimes(1));
    expect(commands.saveDictionary).toHaveBeenCalledWith('global', [
      { from: 'замок', to: 'зам+ок', forms: [] },
    ]);
  });

  it('does not report a save when nothing was written', async () => {
    const onSaved = vi.fn();
    await renderModal({ onSaved });
    expect(onSaved).not.toHaveBeenCalled();
  });
});

interface PreviewResult {
  spoken_text: string;
  wav_base64: string;
  duration_sec: number;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe('DictionaryModal pronunciation preview', () => {
  const FIXTURE: PreviewResult = {
    spoken_text: 'тест',
    wav_base64: 'UklGRg==',
    duration_sec: 0.4,
  };

  /** The play/stop control of one field. */
  const control = (field: 'from' | 'to') =>
    document.querySelector<HTMLButtonElement>(`[data-preview="${field}"]`);

  /** Open the add form once the serving-engine probe has settled. */
  async function openAddForm() {
    await vi.waitFor(() => expect(commands.getAvailableEngines).toHaveBeenCalled());
    clickButton('Добавить слово');
  }

  /** Select a dictionary scope through the Mantine combobox. */
  async function selectScope(optionLabel: string) {
    const select = document.querySelector<HTMLInputElement>('input[value="Общий"]');
    expect(select, 'scope selector rendered').not.toBeNull();
    await act(async () => {
      select!.click();
      await Promise.resolve();
    });
    const option = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find(
      (candidate) => candidate.textContent === optionLabel,
    );
    expect(option, `scope option ${optionLabel} rendered`).toBeTruthy();
    await act(async () => {
      option!.click();
      await Promise.resolve();
    });
  }

  it('renders a play control per field and disables both while empty', async () => {
    await renderModal();
    await openAddForm();
    expect(control('from')).not.toBeNull();
    expect(control('to')).not.toBeNull();
    expect(control('from')!.disabled).toBe(true);
    expect(control('to')!.disabled).toBe(true);
  });

  it('disables a control while its field is invalid', async () => {
    await renderModal();
    await openAddForm();
    typeInto('Слово', 'getUserData()');
    typeInto('Как произносить', 'Моск+ва');
    expect(control('from')!.disabled).toBe(true);
    expect(control('to')!.disabled).toBe(true);
  });

  it('speaks the current reading of the word', async () => {
    await renderModal();
    await openAddForm();
    typeInto('Слово', 'IPv6');
    await vi.waitFor(() => expect(control('from')!.disabled).toBe(false));
    act(() => control('from')!.click());
    await vi.waitFor(() =>
      expect(commands.previewPronunciation).toHaveBeenCalledWith('IPv6', 'normalize'),
    );
  });

  it('speaks the typed spoken form', async () => {
    await renderModal();
    await openAddForm();
    typeInto('Как произносить', 'айпи ви ш+есть');
    await vi.waitFor(() => expect(control('to')!.disabled).toBe(false));
    act(() => control('to')!.click());
    await vi.waitFor(() =>
      expect(commands.previewPronunciation).toHaveBeenCalledWith('айпи ви ш+есть', 'spoken'),
    );
  });

  it('blocks both controls while synthesizing', async () => {
    const pending = deferred<PreviewResult>();
    vi.mocked(commands.previewPronunciation).mockReturnValueOnce(pending.promise);
    await renderModal();
    await openAddForm();
    typeInto('Слово', 'IPv6');
    typeInto('Как произносить', 'шесть');
    await vi.waitFor(() => expect(control('from')!.disabled).toBe(false));

    act(() => control('from')!.click());
    await vi.waitFor(() => expect(control('from')!.getAttribute('data-loading')).toBe('true'));
    expect(control('to')!.disabled).toBe(true);

    await act(async () => {
      pending.resolve(FIXTURE);
      await Promise.resolve();
    });
    await vi.waitFor(() =>
      expect(control('from')!.getAttribute('aria-label')).toBe(
        translate('ru', 'dictionary.preview.stop'),
      ),
    );
  });

  it('stops the audio on a second click', async () => {
    await renderModal();
    await openAddForm();
    typeInto('Слово', 'IPv6');
    await vi.waitFor(() => expect(control('from')!.disabled).toBe(false));
    act(() => control('from')!.click());
    await vi.waitFor(() =>
      expect(control('from')!.getAttribute('aria-label')).toBe(
        translate('ru', 'dictionary.preview.stop'),
      ),
    );

    act(() => control('from')!.click());
    await vi.waitFor(() =>
      expect(control('from')!.getAttribute('aria-label')).toBe(
        translate('ru', 'dictionary.preview.from.aria'),
      ),
    );
    expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:preview-audio');
  });

  it('serves a repeated preview from the cache', async () => {
    await renderModal();
    await openAddForm();
    typeInto('Слово', 'IPv6');
    await vi.waitFor(() => expect(control('from')!.disabled).toBe(false));
    act(() => control('from')!.click());
    await vi.waitFor(() => expect(commands.previewPronunciation).toHaveBeenCalledTimes(1));
    act(() => control('from')!.click());
    await vi.waitFor(() => expect(control('from')!.getAttribute('data-preview')).toBe('from'));

    act(() => control('from')!.click());
    await vi.waitFor(() => expect(commands.previewPronunciation).toHaveBeenCalledTimes(1));
  });

  it('shows one inline message and keeps the values on failure', async () => {
    vi.mocked(commands.previewPronunciation).mockRejectedValueOnce({
      type: 'synthesis_error',
      code: 'bundle_not_installed',
      message: 'бандл не скачан',
    });
    await renderModal();
    await openAddForm();
    typeInto('Слово', 'IPv6');
    await vi.waitFor(() => expect(control('from')!.disabled).toBe(false));
    act(() => control('from')!.click());

    await vi.waitFor(() => {
      expect(document.querySelector('[data-preview-error]')?.textContent).toContain(
        translate('ru', 'dictionary.preview.error.model_missing'),
      );
    });
    expect(fieldValue('Слово')).toBe('IPv6');
  });

  it('pauses narration instead of stopping it', async () => {
    await renderModal();
    await openAddForm();
    typeInto('Слово', 'IPv6');
    await vi.waitFor(() => expect(control('from')!.disabled).toBe(false));
    act(() => control('from')!.click());
    await vi.waitFor(() => expect(commands.pausePlayback).toHaveBeenCalledTimes(1));
    expect(commands.stopPlayback).not.toHaveBeenCalled();
  });

  it('stops the preview when the editor closes', async () => {
    await renderModal();
    await openAddForm();
    typeInto('Слово', 'IPv6');
    await vi.waitFor(() => expect(control('from')!.disabled).toBe(false));
    act(() => control('from')!.click());
    await vi.waitFor(() =>
      expect(control('from')!.getAttribute('aria-label')).toBe(
        translate('ru', 'dictionary.preview.stop'),
      ),
    );

    clickButton('Отмена');
    await vi.waitFor(() => expect(control('from')).toBeNull());
    expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:preview-audio');
  });

  it('names the serving engine when the scope belongs to another engine', async () => {
    await renderModal();
    await vi.waitFor(() => expect(commands.getAvailableEngines).toHaveBeenCalled());
    await selectScope('TeraTTSv2');
    clickButton('Добавить слово');
    await vi.waitFor(() => {
      expect(document.querySelector('[data-preview-hint]')?.textContent).toContain('Piper');
    });
  });

  it('shows no engine hint for the active engine scope', async () => {
    await renderModal();
    await vi.waitFor(() => expect(commands.getAvailableEngines).toHaveBeenCalled());
    await selectScope('Piper');
    clickButton('Добавить слово');
    expect(document.querySelector('[data-preview-hint]')).toBeNull();
  });
});
