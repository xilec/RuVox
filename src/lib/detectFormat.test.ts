import { describe, expect, it } from 'vitest';

import {
  detectFormat,
  MARKDOWN_MIN_INLINE_LINKS,
  MARKDOWN_MIN_LIST_LINES,
} from './detectFormat';

describe('detectFormat: html', () => {
  it('detects a full HTML document by its prefix', () => {
    expect(detectFormat('<!DOCTYPE html><html lang="ru"><body>текст</body></html>')).toBe('html');
    expect(detectFormat('<!doctype html>\n<p>страница</p>')).toBe('html');
    expect(detectFormat('<html>\n<head><title>t</title></head></html>')).toBe('html');
  });

  it('keeps a full HTML document html despite heading-like lines inside', () => {
    expect(detectFormat('<!DOCTYPE html>\n<html><body># notes\n<p>текст</p></body></html>')).toBe('html');
  });

  it('detects a markup fragment delimited by tags', () => {
    expect(detectFormat('<p>Первый</p><p>Второй</p><b>третий</b>')).toBe('html');
    expect(detectFormat('<div>\n  <p>текст</p>\n</div>')).toBe('html');
  });

  it('detects a bare tag-pair snippet as html', () => {
    expect(detectFormat('<b>жирным</b>')).toBe('html');
  });

  it('detects tag-delimited text after trimming invisible edge characters', () => {
    expect(detectFormat('  \n\t<div><p>текст</p></div>\n ')).toBe('html');
    expect(detectFormat('\u200B<span>текст</span>\uFEFF')).toBe('html');
  });

  it('keeps a text starting with a tag but not ending with one non-html', () => {
    // The end boundary is not a tag: an unclosed fragment and a generic
    // parameter both stay out of html.
    expect(detectFormat('<p>раз\n<p>два\n<p>три')).toBe('plain');
    expect(detectFormat('<T> get_user_data() возвращает значение')).toBe('plain');
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

  it('classifies changelog-style prose with placeholder fragments as markdown', () => {
    // Regression: the real CHANGELOG.md carries `<UnlistenFn>` and the
    // `<type>(<module>): <desc>` commit-format line — tag-looking fragments
    // buried in prose, which the boundary rule ignores.
    const changelog = [
      '# Changelog',
      '',
      '## [Unreleased]',
      '',
      '- Commit format: `<type>(<module>): <desc>`.',
      '- Uses the canonical `Promise<UnlistenFn>[]` pattern.',
      '- Another entry.',
    ].join('\n');
    expect(detectFormat(changelog)).toBe('markdown');
  });

  it('classifies list-heavy text with generic-parameter fragments as markdown', () => {
    const text = '- использует Vec<T> внутри\n- возвращает Option<E> наружу\n- хранит HashMap<K> рядом';
    expect(detectFormat(text)).toBe('markdown');
  });
});

describe('detectFormat: plain', () => {
  it('classifies ordinary prose as plain', () => {
    expect(detectFormat('Вызови getUserData() через API и проверь ответ.')).toBe('plain');
  });

  it('classifies empty and whitespace-only text as plain', () => {
    expect(detectFormat('')).toBe('plain');
    expect(detectFormat('   \n\t  ')).toBe('plain');
    expect(detectFormat('\u200B\uFEFF')).toBe('plain');
  });

  it('keeps angle-bracket prose plain', () => {
    expect(detectFormat('if a < b && c > d { return; }')).toBe('plain');
    expect(detectFormat('функция Vec<T> get_user_data() возвращает данные')).toBe('plain');
    expect(detectFormat('подключите <cmath> и вызовите std::sqrt')).toBe('plain');
  });

  it('keeps a single stray tag-looking fragment plain', () => {
    expect(detectFormat('параграф с одним <span>фрагментом внутри')).toBe('plain');
  });

  it('ignores a decorative line starting with a dash', () => {
    expect(detectFormat('список изменений:\n- всего один пункт и много текста')).toBe('plain');
  });
});
