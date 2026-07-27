// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';

import { resolveIngest } from './ingest';

describe('resolveIngest', () => {
  it('passes plain and markdown text through unchanged with the format', () => {
    const text = '<p>разметка</p> as-is';
    expect(resolveIngest(text, 'plain')).toEqual({
      kind: 'direct',
      text,
      format: 'plain',
    });
    expect(resolveIngest(text, 'markdown')).toEqual({
      kind: 'direct',
      text,
      format: 'markdown',
    });
  });

  it('extracts text and keeps sanitized markup for html', () => {
    const action = resolveIngest(
      '<div><button>Купить</button><p>Вызови <code>API</code></p><script>alert(1)</script></div>',
      'html',
    );
    expect(action).toEqual({
      kind: 'html',
      text: 'Вызови API',
      htmlSource:
        '<div><button>Купить</button><p>Вызови <code>API</code></p></div>',
    });
  });

  it('rejects html markup with no extractable text', () => {
    expect(resolveIngest('<button>Купить</button>', 'html')).toEqual({
      kind: 'reject',
    });
  });
});
