// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { extractTextForTts, annotateHtmlWords } from './htmlText';

function annotate(html: string): { container: HTMLElement; text: string } {
  const container = document.createElement('div');
  container.innerHTML = html;
  const text = annotateHtmlWords(container);
  return { container, text };
}

describe('extractTextForTts', () => {
  it('extracts paragraphs with inline code and keeps word offsets consistent', () => {
    const html = '<p>Вызови <code>getUserData()</code> через <b>API</b></p>';
    expect(extractTextForTts(html)).toBe('Вызови getUserData() через API');
  });

  it('excludes chrome element subtrees', () => {
    const html =
      '<nav>Меню</nav><button>Купить</button><script>var x = 1;</script><p>Текст</p>';
    expect(extractTextForTts(html)).toBe('Текст');
  });

  it('separates block elements with newlines', () => {
    expect(extractTextForTts('<p>Раз</p><p>Два</p>')).toBe('Раз\nДва');
  });

  it('emits newlines for br and list items', () => {
    expect(extractTextForTts('<div>строка<br>ещё</div>')).toBe('строка\nещё');
    expect(extractTextForTts('<ul><li>первый</li><li>второй</li></ul>')).toBe(
      'первый\nвторой',
    );
  });

  it('extracts table cells line by line (cells are block-level)', () => {
    const html = '<table><tr><td>a1</td><td>b1</td></tr><tr><td>a2</td><td>b2</td></tr></table>';
    expect(extractTextForTts(html)).toBe('a1\nb1\na2\nb2');
  });

  it('collapses inline whitespace including NBSP', () => {
    expect(extractTextForTts('<p>a  b   c</p>')).toBe('a b c');
    expect(extractTextForTts('<p>a\n\t b</p>')).toBe('a b');
  });

  it('decodes HTML entities', () => {
    expect(extractTextForTts('<p>a &amp; b &lt;c&gt;</p>')).toBe('a & b <c>');
  });

  it('returns an empty string for whitespace-only content', () => {
    expect(extractTextForTts('<p>   </p><div></div>')).toBe('');
  });

  it('concatenates adjacent inline elements without injecting spaces', () => {
    expect(extractTextForTts('<p>Вызови<code>API</code></p>')).toBe('ВызовиAPI');
  });
});

describe('annotateHtmlWords', () => {
  it('produces the same text as extractTextForTts for the same document', () => {
    const html =
      '<p>Вызови <code>getUserData()</code> через <b>API</b></p><ul><li>раз</li><li>два</li></ul>';
    expect(annotate(html).text).toBe(extractTextForTts(html));
  });

  it('wraps words in spans with codepoint offsets into the extracted text', () => {
    const { container } = annotate('<p>Вызови <code>getUserData()</code> через <b>API</b></p>');
    const spans = Array.from(container.querySelectorAll<HTMLElement>('span[data-orig-start]'));
    const byText = new Map(spans.map((s) => [s.textContent, s]));
    expect(byText.get('Вызови')?.dataset.origStart).toBe('0');
    expect(byText.get('Вызови')?.dataset.origEnd).toBe('6');
    expect(byText.get('getUserData()')?.dataset.origStart).toBe('7');
    expect(byText.get('getUserData()')?.dataset.origEnd).toBe('20');
    expect(byText.get('через')?.dataset.origStart).toBe('21');
    expect(byText.get('API')?.dataset.origStart).toBe('27');
    expect(byText.get('API')?.dataset.origEnd).toBe('30');
  });

  it('keeps spans inside their original inline elements', () => {
    const { container } = annotate('<p>x <b>API</b></p>');
    const bold = container.querySelector('b span[data-orig-start]');
    expect(bold?.textContent).toBe('API');
  });

  it('counts astral characters as one codepoint', () => {
    const { container, text } = annotate('<p>😀 ok</p>');
    expect(text).toBe('😀 ok');
    const ok = Array.from(container.querySelectorAll<HTMLElement>('span[data-orig-start]')).find(
      (s) => s.textContent === 'ok',
    );
    expect(ok?.dataset.origStart).toBe('2');
    expect(ok?.dataset.origEnd).toBe('4');
  });

  it('preserves original whitespace in the DOM (no reformatting of pre)', () => {
    const { container } = annotate('<pre>a  b\nc</pre>');
    expect(container.textContent).toBe('a  b\nc');
  });

  it('does not annotate excluded subtrees', () => {
    const { container } = annotate('<div><button>Купить</button><p>Текст</p></div>');
    expect(container.querySelector('button')?.textContent).toBe('Купить');
    expect(container.querySelectorAll('button span').length).toBe(0);
  });
});
