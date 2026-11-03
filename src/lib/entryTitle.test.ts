import { describe, expect, it } from 'vitest';
import { deriveTitle, entryDisplayTitle, TITLE_MAX_CHARS } from './entryTitle';

/**
 * Shared case table — mirrored verbatim in
 * `src-tauri/src/storage/title.rs`. The backend seeds `TextEntry.title` with
 * `derive_title`, the UI re-derives it for legacy entries; a divergence would
 * make identical texts render different titles. Change both tables at once.
 */
const CASES: ReadonlyArray<[input: string, expected: string | null]> = [
  ['Привет мир', 'Привет мир'],
  ['## Установка\nТекст', 'Установка'],
  ['# Заголовок с  пробелами', 'Заголовок с пробелами'],
  ['- Первый пункт\n- Второй', 'Первый пункт'],
  ['* Звёздный пункт', 'Звёздный пункт'],
  ['+ Плюсовой пункт', 'Плюсовой пункт'],
  ['1. Первый шаг', 'Первый шаг'],
  ['12) Двенадцатый шаг', 'Двенадцатый шаг'],
  ['   \n\n  # Заголовок после пустых строк  \n', 'Заголовок после пустых строк'],
  ['#\n# Настоящий заголовок', 'Настоящий заголовок'],
  ['#hashtag', '#hashtag'],
  ['-5 градусов', '-5 градусов'],
  ['', null],
  ['   \n\t\n ', null],
  ['#', null],
  ['- ', null],
  ['\u2003\n', null],
  // CRLF input: the trailing `\r` is whitespace, not part of the title.
  ['## Заголовок\r\nТекст', 'Заголовок'],
  // A lone `\r` is not a line break, but it still collapses to a space.
  ['Заголовок\rХвост', 'Заголовок Хвост'],
  // NEL (U+0085) is Unicode White_Space — trimmed at the edge and collapsed
  // inside, unlike JS `String.prototype.trim`.
  ['\u0085# Заголовок', 'Заголовок'],
  ['# Заголовок\u0085Текст', 'Заголовок Текст'],
  // BOM (U+FEFF) is not White_Space, unlike JS `String.prototype.trim`.
  ['\uFEFF# Заголовок', '\uFEFF# Заголовок'],
];

describe('deriveTitle', () => {
  it('matches the shared backend case table', () => {
    for (const [input, expected] of CASES) {
      expect(deriveTitle(input), JSON.stringify(input)).toBe(expected);
    }
  });

  it('uses the first non-empty line', () => {
    expect(deriveTitle('\n\n  Вторая строка\nТретья')).toBe('Вторая строка');
  });

  it('caps at the shared limit in code points', () => {
    const long = 'ж'.repeat(TITLE_MAX_CHARS + 50);
    expect(Array.from(deriveTitle(long) ?? '')).toHaveLength(TITLE_MAX_CHARS);
  });

  it('does not leave a trailing space after the cap', () => {
    const text = `${'ж'.repeat(TITLE_MAX_CHARS - 1)} хвост`;
    const title = deriveTitle(text);
    expect(title).not.toBeNull();
    expect(Array.from(title!)).toHaveLength(TITLE_MAX_CHARS - 1);
    expect(title!.endsWith(' ')).toBe(false);
  });

  it('caps an astral character whole instead of splitting a surrogate', () => {
    const text = `${'ж'.repeat(TITLE_MAX_CHARS - 1)}😀хвост`;
    const title = deriveTitle(text);
    expect(title).toBe(`${'ж'.repeat(TITLE_MAX_CHARS - 1)}😀`);
    expect(Array.from(title!)).toHaveLength(TITLE_MAX_CHARS);
  });

  it('strips nested markers', () => {
    expect(deriveTitle('- ## Вложенный')).toBe('Вложенный');
  });
});

describe('entryDisplayTitle', () => {
  it('prefers the stored title', () => {
    expect(
      entryDisplayTitle({ title: 'Своё имя', original_text: '## Другое' }),
    ).toBe('Своё имя');
  });

  it('derives a fallback for legacy entries', () => {
    expect(
      entryDisplayTitle({ title: null, original_text: '## Установка\nТекст' }),
    ).toBe('Установка');
  });

  it('treats a blank stored title as absent', () => {
    expect(
      entryDisplayTitle({ title: '   ', original_text: 'обычный текст' }),
    ).toBe('обычный текст');
  });

  it('falls back to the raw text when nothing is derivable', () => {
    expect(entryDisplayTitle({ title: null, original_text: '###' })).toBe('###');
  });
});
