import { describe, expect, it } from 'vitest';

import {
  detectFormat,
  HTML_MIN_TAG_FRAGMENTS,
  MARKDOWN_MIN_INLINE_LINKS,
  MARKDOWN_MIN_LIST_LINES,
} from './detectFormat';

describe('detectFormat: html', () => {
  it('detects a full HTML document by its prefix', () => {
    expect(detectFormat('<!DOCTYPE html><html lang="ru"><body>текст</body></html>')).toBe('html');
    expect(detectFormat('<!doctype html>\n<p>страница</p>')).toBe('html');
    expect(detectFormat('<html>\n<head><title>t</title></head></html>')).toBe('html');
  });

  it('detects a markup fragment with several tags', () => {
    expect(detectFormat('<p>Первый</p><p>Второй</p><b>третий</b>')).toBe('html');
  });

  it('detects html at exactly the fragment threshold', () => {
    // <p>, </p>, <b> — exactly three fragments.
    expect(detectFormat('<p>Первый</p> и <b>второй')).toBe('html');
  });

  it(`keeps fewer than ${HTML_MIN_TAG_FRAGMENTS} tag fragments plain`, () => {
    // A single paired tag: `<b>` + `</b>` = two fragments, below the threshold.
    expect(detectFormat('выделите <b>жирным</b> при необходимости')).toBe('plain');
  });

  it('keeps angle-bracket prose plain', () => {
    expect(detectFormat('if a < b && c > d { return; }')).toBe('plain');
    expect(detectFormat('функция Vec<T> get_user_data() возвращает данные')).toBe('plain');
    expect(detectFormat('подключите <cmath> и вызовите std::sqrt')).toBe('plain');
  });

  it('keeps a single stray tag-looking fragment plain', () => {
    expect(detectFormat('параграф с одним <span>фрагментом внутри')).toBe('plain');
  });
});

describe('detectFormat: markdown', () => {
  it('detects an ATX heading', () => {
    expect(detectFormat('# Заголовок раздела\n\nобычный текст.')).toBe('markdown');
    expect(detectFormat('### глубокий заголовок')).toBe('markdown');
  });

  it('detects a fenced code block', () => {
    expect(detectFormat('текст:\n```\ncode here\n```')).toBe('markdown');
    expect(detectFormat('пример:\n~~~\ncode\n~~~')).toBe('markdown');
  });

  it(`detects list density at ${MARKDOWN_MIN_LIST_LINES} lines`, () => {
    const list = '- первый\n- второй\n- третий';
    expect(detectFormat(list)).toBe('markdown');
    expect(detectFormat('1. раз\n2) два\n3. три')).toBe('markdown');
  });

  it(`keeps fewer than ${MARKDOWN_MIN_LIST_LINES} list lines plain`, () => {
    expect(detectFormat('- первый пункт\n- второй пункт\nи обычный текст')).toBe('plain');
  });

  it(`detects link density at ${MARKDOWN_MIN_INLINE_LINKS} links`, () => {
    expect(
      detectFormat('см. [доку](https://example.com) и [спеку](https://example.org/spec)'),
    ).toBe('markdown');
  });

  it(`keeps fewer than ${MARKDOWN_MIN_INLINE_LINKS} links plain`, () => {
    expect(detectFormat('см. [доку](https://example.com) подробнее')).toBe('plain');
  });
});

describe('detectFormat: plain', () => {
  it('classifies ordinary prose as plain', () => {
    expect(detectFormat('Вызови getUserData() через API и проверь ответ.')).toBe('plain');
  });

  it('classifies empty and whitespace-only text as plain', () => {
    expect(detectFormat('')).toBe('plain');
    expect(detectFormat('   \n\t  ')).toBe('plain');
  });

  it('ignores a decorative line starting with a dash', () => {
    expect(detectFormat('список изменений:\n- всего один пункт и много текста')).toBe('plain');
  });
});
