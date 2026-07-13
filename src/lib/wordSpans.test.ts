import { describe, expect, it } from 'vitest';

import { escapeHtml, wrapWordsWithOrigPos } from './wordSpans';

describe('escapeHtml', () => {
  it('escapes &, <, >, and " in that order without double-escaping', () => {
    expect(escapeHtml('&<>"')).toBe('&amp;&lt;&gt;&quot;');
  });

  it('escapes a literal "&lt;" sequence to "&amp;lt;" (no re-parsing)', () => {
    expect(escapeHtml('&lt;')).toBe('&amp;lt;');
  });

  it('does not escape single quotes', () => {
    expect(escapeHtml("it's")).toBe("it's");
  });

  it('passes through plain text unchanged', () => {
    expect(escapeHtml('привет мир')).toBe('привет мир');
  });
});

describe('wrapWordsWithOrigPos', () => {
  it('returns an empty string for empty input', () => {
    expect(wrapWordsWithOrigPos('', 0)).toBe('');
  });

  it('wraps a single word with startOffset 0', () => {
    expect(wrapWordsWithOrigPos('hello', 0)).toBe(
      '<span data-orig-start="0" data-orig-end="5">hello</span>',
    );
  });

  it('shifts offsets by startOffset', () => {
    expect(wrapWordsWithOrigPos('hello', 100)).toBe(
      '<span data-orig-start="100" data-orig-end="105">hello</span>',
    );
  });

  it('wraps multiple space-separated words, preserving the separating space', () => {
    expect(wrapWordsWithOrigPos('foo bar', 0)).toBe(
      '<span data-orig-start="0" data-orig-end="3">foo</span>' +
        ' ' +
        '<span data-orig-start="4" data-orig-end="7">bar</span>',
    );
  });

  it('preserves leading and trailing whitespace runs as-is (unwrapped)', () => {
    const out = wrapWordsWithOrigPos('  foo  ', 0);
    expect(out).toBe(
      '  ' + '<span data-orig-start="2" data-orig-end="5">foo</span>' + '  ',
    );
  });

  it('treats tabs and newlines as whitespace separators', () => {
    const out = wrapWordsWithOrigPos('foo\tbar\nbaz', 0);
    expect(out).toBe(
      '<span data-orig-start="0" data-orig-end="3">foo</span>' +
        '\t' +
        '<span data-orig-start="4" data-orig-end="7">bar</span>' +
        '\n' +
        '<span data-orig-start="8" data-orig-end="11">baz</span>',
    );
  });

  it('returns the (escaped) whitespace unchanged when input is whitespace-only', () => {
    expect(wrapWordsWithOrigPos('   ', 5)).toBe('   ');
  });

  it('computes offsets against the raw source, not the escaped/rendered word', () => {
    // The word itself contains characters that expand under escapeHtml
    // (`<b>` -> `&lt;b&gt;`, 4 raw chars -> 8 escaped chars). The
    // data-orig-* attributes must reflect the 4 raw characters.
    const out = wrapWordsWithOrigPos('<b> ok', 0);
    expect(out).toBe(
      '<span data-orig-start="0" data-orig-end="3">&lt;b&gt;</span>' +
        ' ' +
        '<span data-orig-start="4" data-orig-end="6">ok</span>',
    );
  });

  it('computes correct codepoint-width offsets for Cyrillic (BMP) text', () => {
    const out = wrapWordsWithOrigPos('привет мир', 0);
    expect(out).toBe(
      '<span data-orig-start="0" data-orig-end="6">привет</span>' +
        ' ' +
        '<span data-orig-start="7" data-orig-end="10">мир</span>',
    );
  });

  it('documents current UTF-16 code-unit indexing for astral (surrogate-pair) characters', () => {
    // '🚀' is a single Unicode codepoint but occupies two UTF-16 code units.
    // wrapWordsWithOrigPos indexes via plain JS string ops (UTF-16 units), so
    // the emoji "word" is reported as spanning 2 positions (data-orig-end -
    // data-orig-start === 2), not 1 codepoint, and "mir" is pushed one
    // position further out than a codepoint-based scheme would place it.
    // This diverges from the Rust pipeline's char_map, which is indexed by
    // Unicode codepoints (see src-tauri/src/pipeline/tracked_text.rs). This
    // test pins down current behavior; it is not an assertion that the
    // offset is semantically correct.
    const text = '🚀 mir';
    expect(text.length).toBe(6); // 2 UTF-16 units for the emoji + 1 space + 3 for "mir"
    const out = wrapWordsWithOrigPos(text, 0);
    expect(out).toBe(
      '<span data-orig-start="0" data-orig-end="2">🚀</span>' +
        ' ' +
        '<span data-orig-start="3" data-orig-end="6">mir</span>',
    );
  });

  it('accumulates offsets correctly across multiple words with multi-char gaps', () => {
    const out = wrapWordsWithOrigPos('a  bb   ccc', 10);
    expect(out).toBe(
      '<span data-orig-start="10" data-orig-end="11">a</span>' +
        '  ' +
        '<span data-orig-start="13" data-orig-end="15">bb</span>' +
        '   ' +
        '<span data-orig-start="18" data-orig-end="21">ccc</span>',
    );
  });
});
