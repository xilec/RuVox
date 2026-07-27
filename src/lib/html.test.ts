// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';

import { renderHtml } from './html';
import { extractTextForTts } from './htmlText';

/** Strip all tags — word spans included — leaving only the visible text. */
function stripTags(html: string): string {
  return html.replace(/<[^>]+>/g, '');
}

describe('renderHtml sanitization', () => {
  it('strips <script> tags but keeps surrounding content', () => {
    const out = renderHtml('<p>ok</p><script>alert(1)</script>');
    expect(out).not.toContain('<script');
    expect(out).not.toContain('alert(1)');
    expect(stripTags(out)).toContain('ok');
  });

  it('strips <iframe>, <object> and <embed> tags', () => {
    const out = renderHtml(
      '<p>ok</p>' +
        '<iframe src="https://evil.example"></iframe>' +
        '<object data="evil.swf"></object>' +
        '<embed src="evil.swf">',
    );
    expect(out).not.toContain('<iframe');
    expect(out).not.toContain('<object');
    expect(out).not.toContain('<embed');
    expect(stripTags(out)).toContain('ok');
  });

  it('strips event handler attributes (onclick, onerror)', () => {
    const out = renderHtml(
      '<p onclick="alert(1)">x</p><img src="https://example.com/a.png" onerror="alert(2)">',
    );
    expect(out).not.toContain('onclick');
    expect(out).not.toContain('onerror');
    expect(out).not.toContain('alert');
    expect(stripTags(out)).toContain('x');
  });

  it('strips javascript: hrefs but keeps the link element', () => {
    const out = renderHtml('<a href="javascript:alert(1)">link</a>');
    expect(out).not.toContain('javascript:');
    expect(out).toContain('<a');
    expect(stripTags(out)).toContain('link');
  });

  it('keeps safe markup (p, b, code, pre) intact', () => {
    const out = renderHtml('<p><b>bold</b></p><pre><code>const x = 1;</code></pre>');
    expect(out).toContain('<p><b>');
    expect(out).toContain('<pre><code>');
    const text = stripTags(out);
    expect(text).toContain('bold');
    expect(text).toContain('const x = 1;');
  });

  it('keeps https links with their href', () => {
    const out = renderHtml('<a href="https://example.com">link</a>');
    expect(out).toContain('href="https://example.com"');
    expect(stripTags(out)).toContain('link');
  });

  it('keeps code text inside <pre><code class="language-*>', () => {
    // highlight.js rewrites the inner markup of code blocks; assert the code
    // itself survives regardless of the token spans it adds.
    const out = renderHtml(
      '<pre><code class="language-rust">fn main() {}</code></pre>',
    );
    const text = stripTags(out);
    expect(text).toContain('fn main() {}');
  });
});

describe('renderHtml word spans', () => {
  it('wraps words in data-orig spans matching the extracted text offsets', () => {
    const html = '<p>Вызови <code>API</code></p>';
    const out = renderHtml(html);
    const extracted = extractTextForTts(html);
    expect(extracted).toBe('Вызови API');
    expect(out).toContain(
      '<span data-orig-start="0" data-orig-end="6">Вызови</span>',
    );
    expect(out).toContain(
      '<span data-orig-start="7" data-orig-end="10">API</span>',
    );
  });

  it('does not emit spans for excluded subtrees', () => {
    const out = renderHtml('<div><button>Купить</button><p>Текст</p></div>');
    expect(out).toContain('<button>Купить</button>');
    const buttonSection = out.slice(out.indexOf('<button>'), out.indexOf('</button>'));
    expect(buttonSection).not.toContain('data-orig-start');
  });
});
