import { describe, expect, it } from 'vitest';

import { plainToWordHtml } from './plainTextHtml';

describe('plainToWordHtml', () => {
  it('returns an empty string for empty input', () => {
    expect(plainToWordHtml('')).toBe('');
  });

  it('wraps each word in a data-orig-* span with correct offsets', () => {
    expect(plainToWordHtml('привет мир')).toBe(
      '<span data-orig-start="0" data-orig-end="6">привет</span>' +
        ' ' +
        '<span data-orig-start="7" data-orig-end="10">мир</span>',
    );
  });

  it('renders markdown-like markup verbatim (headings are not interpreted)', () => {
    expect(plainToWordHtml('# Заголовок')).toBe(
      '<span data-orig-start="0" data-orig-end="1">#</span>' +
        ' ' +
        '<span data-orig-start="2" data-orig-end="11">Заголовок</span>',
    );
  });

  it('renders bold and code markers verbatim across lines, with correct offsets', () => {
    expect(plainToWordHtml('**жирный** и `код`')).toBe(
      '<span data-orig-start="0" data-orig-end="10">**жирный**</span>' +
        ' ' +
        '<span data-orig-start="11" data-orig-end="12">и</span>' +
        ' ' +
        '<span data-orig-start="13" data-orig-end="18">`код`</span>',
    );
  });

  it('escapes raw HTML instead of interpreting it', () => {
    expect(plainToWordHtml('<b>bold</b>')).toBe(
      '<span data-orig-start="0" data-orig-end="11">&lt;b&gt;bold&lt;/b&gt;</span>',
    );
  });

  it('joins lines with <br> and keeps offsets relative to the original text', () => {
    expect(plainToWordHtml('foo\nbar')).toBe(
      '<span data-orig-start="0" data-orig-end="3">foo</span>' +
        '<br>' +
        '<span data-orig-start="4" data-orig-end="7">bar</span>',
    );
  });

  it('keeps correct offsets on the second line with Cyrillic text', () => {
    expect(plainToWordHtml('раз\nдва три')).toBe(
      '<span data-orig-start="0" data-orig-end="3">раз</span>' +
        '<br>' +
        '<span data-orig-start="4" data-orig-end="7">два</span>' +
        ' ' +
        '<span data-orig-start="8" data-orig-end="11">три</span>',
    );
  });

  it('emits a trailing <br> for a trailing newline', () => {
    expect(plainToWordHtml('foo\n')).toBe(
      '<span data-orig-start="0" data-orig-end="3">foo</span><br>',
    );
  });

  it('tracks offsets in codepoints across lines with astral characters', () => {
    // '🌍' is one codepoint (two UTF-16 units): the second line must start at
    // codepoint 2 (emoji + \n), not 3.
    expect(plainToWordHtml('🌍\nмир')).toBe(
      '<span data-orig-start="0" data-orig-end="1">🌍</span>' +
        '<br>' +
        '<span data-orig-start="2" data-orig-end="5">мир</span>',
    );
  });
});
