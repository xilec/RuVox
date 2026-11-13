// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { MantineProvider } from '@mantine/core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { notifications } from '@mantine/notifications';

vi.mock('../lib/tauri', () => ({
  commands: {
    getDictionaries: vi.fn(() =>
      Promise.resolve([{ scope: 'global', name: 'Global', entries: [] }]),
    ),
    saveDictionary: vi.fn(() => Promise.resolve()),
    pickDictionaryImportPath: vi.fn(() => Promise.resolve(null)),
    importDictionary: vi.fn(() => Promise.resolve({ added: 0, updated: 0, skipped: 0 })),
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
    previewPlay: vi.fn(() => Promise.resolve()),
    previewStop: vi.fn(() => Promise.resolve()),
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
import type { PreviewPronunciationResult } from '../lib/tauri';

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  container = document.createElement('div');
  document.body.append(container);
  root = createRoot(container);
  // The preview is tested through the playback commands: the webview has no
  // media pipeline of its own any more (issue #349).
  vi.mocked(commands.previewPlay).mockClear();
  vi.mocked(commands.previewStop).mockClear();
  vi.mocked(commands.previewPronunciation).mockClear();
  vi.mocked(commands.pausePlayback).mockClear();
  vi.mocked(commands.stopPlayback).mockClear();
  vi.mocked(commands.saveDictionary).mockClear();
  vi.mocked(commands.importDictionary).mockClear();
  vi.mocked(commands.pickDictionaryImportPath).mockClear();
  vi.mocked(commands.getConfig).mockClear();
  vi.mocked(commands.getAvailableEngines).mockClear();
  vi.mocked(commands.getDictionaries).mockClear();
  vi.mocked(notifications.show).mockClear();
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
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

/** The input element of the field labelled `label` (null when absent). */
function fieldFor(label: string): HTMLInputElement | null {
  const labelEl = [...document.querySelectorAll('label')].find(
    (candidate) => candidate.textContent?.trim() === label,
  );
  return labelEl?.htmlFor
    ? (document.getElementById(labelEl.htmlFor) as HTMLInputElement | null)
    : null;
}

/** Type into the input whose label is exactly `label`. */
function typeInto(label: string, value: string) {
  const field = fieldFor(label);
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
  return fieldFor(label)?.value ?? '';
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

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/** Preview response for the cases that only need a playable WAV. */
const FIXTURE: PreviewPronunciationResult = {
  spoken_text: 'тест',
  wav_base64: 'UklGRg==',
  duration_sec: 0.4,
};

/** Labels the preview cases drive the dialog with — no raw Russian. */
const ADD_LABEL = translate('ru', 'dictionary.add');
const SAVE_LABEL = translate('ru', 'common.save');
const CANCEL_LABEL = translate('ru', 'common.cancel');
const FROM_LABEL = translate('ru', 'dictionary.form.from');
const TO_LABEL = translate('ru', 'dictionary.form.to');

/** A rejected Tauri command shaped like `CommandError`. */
function codedError(code: string) {
  return { type: 'synthesis_error', code, params: [], message: `backend: ${code}` };
}

describe('DictionaryModal pronunciation preview', () => {
  /** The play/stop control of one field. */
  const control = (field: 'from' | 'to') =>
    document.querySelector<HTMLButtonElement>(`[data-preview="${field}"]`);

  /** Open the add form once the serving-engine probe has settled. */
  async function openAddForm() {
    await vi.waitFor(() => expect(commands.getAvailableEngines).toHaveBeenCalled());
    clickButton(ADD_LABEL);
  }

  /** Wait for the control to offer "stop" — proof the preview is playing. */
  async function waitForPlayback(field: 'from' | 'to') {
    await vi.waitFor(() =>
      expect(control(field)!.getAttribute('aria-label')).toBe(
        translate('ru', 'dictionary.preview.stop'),
      ),
    );
  }

  /** Type a value that makes the `from` control playable. */
  async function typePlayableFrom(value = 'IPv6') {
    typeInto(FROM_LABEL, value);
    await vi.waitFor(() => expect(control('from')!.disabled).toBe(false));
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

  it('enables a control on a valid value and disables it again when the value breaks', async () => {
    await renderModal();
    await openAddForm();
    // Start from a value the control accepts: a permanently disabled control
    // would otherwise pass the "invalid" half of this test.
    await typePlayableFrom();
    typeInto(FROM_LABEL, 'getUserData()');
    await vi.waitFor(() => expect(control('from')!.disabled).toBe(true));
  });

  it('speaks the current reading of the word', async () => {
    await renderModal();
    await openAddForm();
    await typePlayableFrom();
    act(() => control('from')!.click());
    await vi.waitFor(() =>
      expect(commands.previewPronunciation).toHaveBeenCalledWith('IPv6', 'normalize'),
    );
  });

  it('speaks the typed spoken form', async () => {
    await renderModal();
    await openAddForm();
    typeInto(TO_LABEL, 'айпи ви ш+есть');
    await vi.waitFor(() => expect(control('to')!.disabled).toBe(false));
    act(() => control('to')!.click());
    await vi.waitFor(() =>
      expect(commands.previewPronunciation).toHaveBeenCalledWith('айпи ви ш+есть', 'spoken'),
    );
  });

  it('blocks both controls while synthesizing', async () => {
    const pending = deferred<PreviewPronunciationResult>();
    vi.mocked(commands.previewPronunciation).mockReturnValueOnce(pending.promise);
    await renderModal();
    await openAddForm();
    await typePlayableFrom();
    typeInto(TO_LABEL, 'шесть');

    act(() => control('from')!.click());
    await vi.waitFor(() => expect(control('from')!.getAttribute('data-loading')).toBe('true'));
    expect(control('to')!.disabled).toBe(true);

    await act(async () => {
      pending.resolve(FIXTURE);
      await Promise.resolve();
    });
    await waitForPlayback('from');
  });

  it('stops the audio on a second click', async () => {
    await renderModal();
    await openAddForm();
    await typePlayableFrom();
    act(() => control('from')!.click());
    await waitForPlayback('from');

    act(() => control('from')!.click());
    await vi.waitFor(() =>
      expect(control('from')!.getAttribute('aria-label')).toBe(
        translate('ru', 'dictionary.preview.from.aria'),
      ),
    );
    expect(commands.previewStop).toHaveBeenCalled();
  });

  it('serves a repeated preview from the cache', async () => {
    await renderModal();
    await openAddForm();
    await typePlayableFrom();

    act(() => control('from')!.click());
    await waitForPlayback('from');
    // Stop, then play again: the second play must come from the cache.
    act(() => control('from')!.click());
    await vi.waitFor(() =>
      expect(control('from')!.getAttribute('aria-label')).toBe(
        translate('ru', 'dictionary.preview.from.aria'),
      ),
    );
    act(() => control('from')!.click());
    await waitForPlayback('from');

    expect(commands.previewPronunciation).toHaveBeenCalledTimes(1);
    // Two plays, no second synthesis: the cached WAV is replayed through the
    // backend player.
    expect(commands.previewPlay).toHaveBeenCalledTimes(2);
  });

  it('retires the cache after a save', async () => {
    await renderModal();
    await openAddForm();
    await typePlayableFrom();
    typeInto(TO_LABEL, 'ай пи ви ш+есть');

    act(() => control('from')!.click());
    await waitForPlayback('from');
    clickButton(SAVE_LABEL);
    await vi.waitFor(() => expect(commands.saveDictionary).toHaveBeenCalledTimes(1));

    clickButton(ADD_LABEL);
    await typePlayableFrom();
    act(() => control('from')!.click());
    // The saved dictionary changed, so the cached reading is retired.
    await vi.waitFor(() => expect(commands.previewPronunciation).toHaveBeenCalledTimes(2));
  });

  it('retires the cache after an import', async () => {
    vi.mocked(commands.pickDictionaryImportPath).mockResolvedValueOnce('/tmp/words.toml');
    vi.mocked(commands.importDictionary).mockResolvedValueOnce({
      added: 1,
      updated: 0,
      skipped: 0,
    });
    await renderModal();
    await openAddForm();
    await typePlayableFrom();

    act(() => control('from')!.click());
    await waitForPlayback('from');
    act(() => control('from')!.click());
    await vi.waitFor(() =>
      expect(control('from')!.getAttribute('aria-label')).toBe(
        translate('ru', 'dictionary.preview.from.aria'),
      ),
    );

    clickButton(translate('ru', 'dictionary.import'));
    await vi.waitFor(() =>
      expect(document.body.textContent).toContain(
        translate('ru', 'dictionary.import.choose.title'),
      ),
    );
    clickButton(translate('ru', 'dictionary.import.merge'));
    await vi.waitFor(() => expect(commands.importDictionary).toHaveBeenCalledTimes(1));

    act(() => control('from')!.click());
    await vi.waitFor(() => expect(commands.previewPronunciation).toHaveBeenCalledTimes(2));
  });

  it('pauses narration before the preview synthesizes', async () => {
    await renderModal();
    await openAddForm();
    await typePlayableFrom();

    act(() => control('from')!.click());
    await vi.waitFor(() => expect(commands.previewPronunciation).toHaveBeenCalledTimes(1));
    // The pause lands first: the click is the user asking to listen now, and
    // a cache miss can take seconds.
    const pauseOrder = vi.mocked(commands.pausePlayback).mock.invocationCallOrder[0];
    const synthesisOrder =
      vi.mocked(commands.previewPronunciation).mock.invocationCallOrder[0];
    expect(pauseOrder).toBeLessThan(synthesisOrder);

    // The preview still succeeds, and pauses again right before the audio for
    // narration started while it was synthesizing.
    await waitForPlayback('from');
    expect(commands.pausePlayback).toHaveBeenCalledTimes(2);
    expect(commands.stopPlayback).not.toHaveBeenCalled();
  });

  it('reports a failed pause without failing the preview', async () => {
    vi.mocked(commands.pausePlayback).mockRejectedValueOnce(
      codedError('playback.pause_failed'),
    );
    await renderModal();
    await openAddForm();
    await typePlayableFrom();

    act(() => control('from')!.click());
    await vi.waitFor(() =>
      expect(vi.mocked(notifications.show)).toHaveBeenCalledWith(
        expect.objectContaining({
          title: translate('ru', 'dictionary.preview.pause_failed.title'),
        }),
      ),
    );
    await waitForPlayback('from');
  });

  it('shows one inline message and keeps the values on failure', async () => {
    vi.mocked(commands.previewPronunciation).mockRejectedValueOnce(
      codedError('bundle_not_installed'),
    );
    await renderModal();
    await openAddForm();
    await typePlayableFrom();
    act(() => control('from')!.click());

    await vi.waitFor(() => {
      expect(document.querySelector('[data-preview-error]')?.textContent).toContain(
        translate('ru', 'errors.bundle_not_installed'),
      );
    });
    expect(fieldValue(FROM_LABEL)).toBe('IPv6');
    // A failed synthesis leaves the narration paused: no resume.
    expect(commands.pausePlayback).toHaveBeenCalledTimes(1);
  });

  // Every engine rejection the backend can answer with must land in one
  // localized sentence, interpolated into the "Не удалось прослушать" wrapper
  // (the same `formatError` the rest of the app uses).
  const ENGINE_REJECTIONS: [string, string][] = [
    ['preview.engine_busy', 'errors.preview.engine_busy'],
    ['preview.invalid_stress', 'errors.preview.invalid_stress'],
    ['voice_not_installed', 'errors.voice_not_installed'],
  ];

  for (const [code, key] of ENGINE_REJECTIONS) {
    it(`localizes a ${code} rejection`, async () => {
      vi.mocked(commands.previewPronunciation).mockRejectedValueOnce(codedError(code));
      await renderModal();
      await openAddForm();
      await typePlayableFrom();
      act(() => control('from')!.click());

      await vi.waitFor(() => {
        expect(document.querySelector('[data-preview-error]')?.textContent).toBe(
          translate('ru', 'dictionary.preview.error', [translate('ru', key)]),
        );
      });
    });
  }

  it('reports a playback failure', async () => {
    vi.mocked(commands.previewPlay).mockRejectedValueOnce(new Error('no audio device'));
    await renderModal();
    await openAddForm();
    await typePlayableFrom();
    act(() => control('from')!.click());

    await vi.waitFor(() => {
      expect(document.querySelector('[data-preview-error]')?.textContent).toBe(
        translate('ru', 'dictionary.preview.error.play_failed'),
      );
    });
  });

  it('drops a response that arrives after the editor closed', async () => {
    const pending = deferred<PreviewPronunciationResult>();
    vi.mocked(commands.previewPronunciation).mockReturnValueOnce(pending.promise);
    await renderModal();
    await openAddForm();
    await typePlayableFrom();

    act(() => control('from')!.click());
    await vi.waitFor(() => expect(control('from')!.getAttribute('data-loading')).toBe('true'));

    clickButton(CANCEL_LABEL);
    await vi.waitFor(() => expect(control('from')).toBeNull());

    await act(async () => {
      pending.resolve(FIXTURE);
      await Promise.resolve();
    });
    // The stale response must not start playback.
    expect(commands.previewPlay).not.toHaveBeenCalled();
  });

  it('stops the preview when the editor closes', async () => {
    await renderModal();
    await openAddForm();
    await typePlayableFrom();
    act(() => control('from')!.click());
    await waitForPlayback('from');

    clickButton(CANCEL_LABEL);
    await vi.waitFor(() => expect(control('from')).toBeNull());
    expect(commands.previewStop).toHaveBeenCalled();
  });

  it('returns to the play state when the clip ends', async () => {
    await renderModal();
    await openAddForm();
    await typePlayableFrom();
    act(() => control('from')!.click());
    await waitForPlayback('from');

    // The dialog stops the backend player after `duration_sec` plus a margin;
    // the backend destroys mpv and deletes the WAV on that stop.
    await vi.waitFor(
      () => expect(commands.previewStop).toHaveBeenCalled(),
      { timeout: 3000 },
    );
    await vi.waitFor(() =>
      expect(control('from')!.getAttribute('aria-label')).toBe(
        translate('ru', 'dictionary.preview.from.aria'),
      ),
    );
  });

  it('renders no media element', async () => {
    await renderModal();
    await openAddForm();
    await typePlayableFrom();
    act(() => control('from')!.click());
    await waitForPlayback('from');

    // The webview must not decode audio: a media element is the second audio
    // stack whose missing plugins killed the web process (issue #349).
    expect(document.querySelector('audio, video')).toBeNull();
  });

  it('places the hint right under the spoken-form field', async () => {
    await renderModal();
    await vi.waitFor(() => expect(commands.getAvailableEngines).toHaveBeenCalled());
    await selectScope('TeraTTSv2');
    clickButton(ADD_LABEL);
    await vi.waitFor(() => {
      expect(document.querySelector('[data-preview-hint]')?.textContent).toContain('Piper');
    });

    const hint = document.querySelector('[data-preview-hint]')!;
    const toField = fieldFor(TO_LABEL)!;
    const formsField = fieldFor(translate('ru', 'dictionary.form.forms'))!;
    expect(toField.compareDocumentPosition(hint) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(hint.compareDocumentPosition(formsField) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('names the serving engine when the scope belongs to another engine', async () => {
    await renderModal();
    await vi.waitFor(() => expect(commands.getAvailableEngines).toHaveBeenCalled());
    await selectScope('TeraTTSv2');
    clickButton(ADD_LABEL);
    await vi.waitFor(() => {
      expect(document.querySelector('[data-preview-hint]')?.textContent).toContain('Piper');
    });
  });

  it('shows no engine hint for the active engine scope', async () => {
    await renderModal();
    await vi.waitFor(() => expect(commands.getAvailableEngines).toHaveBeenCalled());
    await selectScope('Piper');
    clickButton(ADD_LABEL);
    expect(document.querySelector('[data-preview-hint]')).toBeNull();
  });
});
