import { describe, expect, it } from 'vitest';

import { renderMarkdown } from './markdown';

describe('renderMarkdown data-orig-* offsets', () => {
  it('tracks codepoint offsets with astral characters before a word', () => {
    // '🌍' is one codepoint (two UTF-16 units): "мир" must start at
    // codepoint 2, matching the Rust char_map contract.
    expect(renderMarkdown('🌍 мир')).toContain(
      '<span data-orig-start="2" data-orig-end="5">мир</span>',
    );
  });

  it('keeps distinct codepoint positions for repeated fragments after an astral char', () => {
    // Codepoints: 🌍(0) space(1) мир(2-5) \n(5) \n(6) мир(7-10).
    const html = renderMarkdown('🌍 мир\n\nмир');
    expect(html).toContain('<span data-orig-start="2" data-orig-end="5">мир</span>');
    expect(html).toContain('<span data-orig-start="7" data-orig-end="10">мир</span>');
  });

  it('counts astral characters in the skipped region between text tokens', () => {
    // The emoji sits inside the fenced code block, i.e. in the region
    // skipped between text tokens (source.slice(searchFrom, pos)) — a
    // different accounting line than token content. Codepoints before
    // "мир": ```(3) \n(1) 🌍(1) \n(1) ```(3) \n\n(2) = 11, so "мир"
    // starts at 11 (a UTF-16 count would give 12).
    const html = renderMarkdown('```\n🌍\n```\n\nмир');
    expect(html).toContain('<span data-orig-start="11" data-orig-end="14">мир</span>');
  });
});

describe('renderMarkdown table scrolling', () => {
  it('wraps tables in a keyboard-focusable scroll box', () => {
    const html = renderMarkdown('| a | b |\n| --- | --- |\n| 1 | 2 |');
    expect(html).toContain('<div data-scroll-x tabindex="0">');
    expect(html).toContain('<table>');
    expect(html).toContain('</table>\n</div>');
  });

  it('keeps headers, cells and both tables intact', () => {
    const html = renderMarkdown(
      '| Заголовок | Второй |\n| --- | --- |\n| ячейка | 1 |\n\n' +
        '| Ещё | Таблица |\n| --- | --- |\n| вторая | 2 |',
    );
    // Highlighting wraps every word in a span, so compare the structure with
    // the data-orig spans stripped out.
    const plain = html.replace(/<span[^>]*>/g, '').replace(/<\/span>/g, '');
    expect(plain.match(/<div data-scroll-x tabindex="0">/g)).toHaveLength(2);
    expect(plain.match(/<table>/g)).toHaveLength(2);
    expect(plain).toContain('<th>Заголовок</th>');
    expect(plain).toContain('<th>Второй</th>');
    expect(plain).toContain('<td>ячейка</td>');
    expect(plain).toContain('<td>вторая</td>');
  });
});
