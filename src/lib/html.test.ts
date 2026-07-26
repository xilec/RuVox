// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';

import { renderHtml } from './html';

describe('renderHtml sanitization', () => {
  it('strips <script> tags but keeps surrounding content', () => {
    const out = renderHtml('<p>ok</p><script>alert(1)</script>');
    expect(out).not.toContain('<script');
    expect(out).not.toContain('alert(1)');
    expect(out).toContain('<p>ok</p>');
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
    expect(out).toContain('<p>ok</p>');
  });

  it('strips event handler attributes (onclick, onerror)', () => {
    const out = renderHtml(
      '<p onclick="alert(1)">x</p><img src="https://example.com/a.png" onerror="alert(2)">',
    );
    expect(out).not.toContain('onclick');
    expect(out).not.toContain('onerror');
    expect(out).not.toContain('alert');
    expect(out).toContain('<p>x</p>');
  });

  it('strips javascript: hrefs but keeps the link element', () => {
    const out = renderHtml('<a href="javascript:alert(1)">link</a>');
    expect(out).not.toContain('javascript:');
    expect(out).toContain('<a');
    expect(out).toContain('link</a>');
  });

  it('keeps safe markup (p, b, code, pre) intact', () => {
    const out = renderHtml('<p><b>bold</b></p><pre><code>const x = 1;</code></pre>');
    expect(out).toContain('<p><b>bold</b></p>');
    expect(out).toContain('<pre><code>');
    expect(out).toContain('const x = 1;');
    expect(out).toContain('</code></pre>');
  });

  it('keeps https links with their href', () => {
    const out = renderHtml('<a href="https://example.com">link</a>');
    expect(out).toContain('href="https://example.com"');
    expect(out).toContain('link</a>');
  });

  it('keeps code text inside <pre><code class="language-*>', () => {
    // highlight.js rewrites the inner markup of code blocks; assert the code
    // itself survives regardless of the token spans it adds.
    const out = renderHtml(
      '<pre><code class="language-rust">fn main() {}</code></pre>',
    );
    const text = out.replace(/<[^>]+>/g, '');
    expect(text).toContain('fn main() {}');
  });
});
