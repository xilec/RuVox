import { afterEach, describe, expect, it } from 'vitest';
import { en } from '../i18n/en';
import { translate } from './i18n';
import { currentLocale, setLocale } from '../stores/locale';

describe('translate', () => {
  it('resolves a RU catalog entry', () => {
    expect(translate('ru', 'queue.play')).toBe('Воспроизвести');
  });

  it('resolves an EN catalog entry', () => {
    expect(translate('en', 'queue.play')).toBe('Play');
  });

  it('interpolates positional {0} params', () => {
    expect(
      translate('ru', 'errors.entry.not_found', ['entry-7']),
    ).toBe('Запись entry-7 не найдена');
  });

  it('interpolates multiple params in order', () => {
    expect(
      translate('en', 'notify.voice.progress.tallied', [
        'model',
        1,
        3,
        '0.5 MB',
        '10.0 MB',
      ]),
    ).toBe('model (1/3): 0.5 MB / 10.0 MB');
  });

  it('leaves an unmatched placeholder intact when its param is missing', () => {
    expect(translate('ru', 'errors.entry.not_found')).toBe(
      'Запись {0} не найдена',
    );
  });

  it('falls back to the key itself for an unknown key', () => {
    expect(translate('en', 'totally.bogus.key')).toBe('totally.bogus.key');
    expect(translate('ru', 'totally.bogus.key')).toBe('totally.bogus.key');
  });
});

describe('RU fallback from the EN catalog path', () => {
  // The EN catalog type (`Record<MessageKey, string>`) makes a missing key
  // unrepresentable at compile time, so the runtime fallback is exercised by
  // removing one entry and restoring it afterwards.
  const editable = en as Record<string, string | undefined>;
  const saved = editable['common.cancel'];

  afterEach(() => {
    editable['common.cancel'] = saved;
  });

  it('falls back to the RU entry when EN lacks the key', () => {
    delete editable['common.cancel'];
    expect(translate('en', 'common.cancel')).toBe('Отмена');
  });
});

describe('t reads the locale store at call time', () => {
  afterEach(() => setLocale('ru'));

  it('follows locale switches without re-imports', () => {
    expect(currentLocale()).toBe('ru');
    setLocale('en');
    expect(translate(currentLocale(), 'player.pause')).toBe('Pause');
    setLocale('ru');
    expect(translate(currentLocale(), 'player.pause')).toBe('Пауза');
  });
});
