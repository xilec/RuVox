// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';

import {
  IMPORTABLE_FILE_EXTENSIONS,
  SPA_MIN_SCRIPT_TAGS,
  SPA_MIN_TEXT_CHARS,
  resolveImport,
} from './importFlow';
import type { CodedImportError } from './importFlow';
import type { AddAction } from './addFlow';

const GATE_ON = { previewEnabled: true };
const GATE_OFF = { previewEnabled: false };

/** Assert-and-narrow: every non-throwing resolveImport result is a step the
 *  executor can run, never an error object at runtime. */
function expectAction(fn: () => AddAction): AddAction {
  const action = fn();
  expect(action).not.toHaveProperty('code');
  return action;
}

function expectCoded(fn: () => unknown): CodedImportError {
  try {
    fn();
  } catch (err) {
    // formatError's object branch localizes exactly this shape.
    const coded = err as CodedImportError;
    expect(coded.type).toBe('internal');
    expect(typeof coded.code).toBe('string');
    expect(Array.isArray(coded.params)).toBe(true);
    return coded;
  }
  throw new Error('expected the call to throw a coded import error');
}

/** A server-rendered article body whose extraction clears the SPA threshold. */
function ssrArticle(chars = 900): string {
  return `<html><head><title>t</title></head><body><article>${'Текст статьи про технику и нормализацию речи. '.repeat(Math.ceil(chars / 44))}</article></body></html>`;
}

describe('file imports: extension decides (spec scenario)', () => {
  it('routes .md to markdown with the gate enabled', () => {
    const action = expectAction(() =>
      resolveImport({ kind: 'file', fileName: 'notes.md', text: '# Заголовок' }, GATE_ON),
    );
    expect(action).toEqual({
      kind: 'preview',
      text: '# Заголовок',
      format: 'markdown',
      plainFallback: null,
    });
  });

  it('routes .txt to plain under both gate states', () => {
    const on = expectAction(() =>
      resolveImport({ kind: 'file', fileName: 'заметки.txt', text: 'Просто текст' }, GATE_ON),
    );
    expect(on).toEqual({ kind: 'preview', text: 'Просто текст', format: 'plain', plainFallback: null });

    const off = expectAction(() =>
      resolveImport({ kind: 'file', fileName: 'notes.txt', text: 'plain text' }, GATE_OFF),
    );
    expect(off).toEqual({ kind: 'direct-plain', text: 'plain text', format: 'plain' });
  });

  it('routes .md directly as markdown when the gate is disabled', () => {
    const action = expectAction(() =>
      resolveImport({ kind: 'file', fileName: 'README.md', text: '# Инструкция' }, GATE_OFF),
    );
    expect(action).toEqual({ kind: 'direct-plain', text: '# Инструкция', format: 'markdown' });
  });

  it.each(['page.html', 'PAGE.HTM', 'c:/dir/a.htm'])('routes %s to the html path', (name) => {
    const markup = '<p>Привет из файла</p>';
    const gated = expectAction(() => resolveImport({ kind: 'file', fileName: name, text: markup }, GATE_ON));
    expect(gated).toEqual({ kind: 'preview', text: markup, format: 'html', plainFallback: null });

    const direct = expectAction(() => resolveImport({ kind: 'file', fileName: name, text: markup }, GATE_OFF));
    expect(direct).toEqual({ kind: 'direct-html', html: markup, plainFallback: null });
  });

  it('rejects unsupported extensions with the coded error', () => {
    const err = expectCoded(() =>
      resolveImport({ kind: 'file', fileName: 'photo.PNG', text: 'binary' }, GATE_ON),
    );
    expect(err.code).toBe('import.unsupported_extension');
    expect(err.params).toEqual(['png']);
  });
});

describe('url imports: content-type routing (#241 interim)', () => {
  it('treats text/plain responses as plain text (spec scenario)', () => {
    const body = 'Сервер отдал чистый текст';
    const on = expectAction(() =>
      resolveImport({ kind: 'url', body, contentType: 'text/plain; charset=windows-1251' }, GATE_ON),
    );
    expect(on.kind).toBe('preview');
    if (on.kind === 'preview') expect(on.format).toBe('plain');

    const off = expectAction(() =>
      resolveImport({ kind: 'url', body, contentType: 'text/plain' }, GATE_OFF),
    );
    expect(off).toEqual({ kind: 'direct-plain', text: body, format: 'plain' });
  });

  it('extracts a server-rendered html article through the html path', () => {
    const action = expectAction(() =>
      resolveImport({ kind: 'url', body: ssrArticle(), contentType: 'text/html; charset=utf-8' }, GATE_ON),
    );
    expect(action.kind).toBe('preview');
    if (action.kind === 'preview') expect(action.format).toBe('html');
  });

  it('routes xhtml (+xml) responses to html', () => {
    const action = expectAction(() =>
      resolveImport(
        { kind: 'url', body: `<html><body><p>${'x'.repeat(SPA_MIN_TEXT_CHARS)}</p></body></html>`, contentType: 'application/xhtml+xml' },
        GATE_ON,
      ),
    );
    expect(action.kind).toBe('preview');
    if (action.kind === 'preview') expect(action.format).toBe('html');
  });

  it('sniffs missing content types: markup goes to html, prose goes to plain', () => {
    const htmlish = `<div>${'Загруженный фрагмент разметки без заголовка. '.repeat(20)}</div>`;
    const asHtml = expectAction(() => resolveImport({ kind: 'url', body: htmlish, contentType: null }, GATE_OFF));
    expect(asHtml.kind).toBe('direct-html');

    const prose = 'Ответ выглядит как обычный абзац текста без тегов';
    const asPlain = expectAction(() => resolveImport({ kind: 'url', body: prose, contentType: null }, GATE_OFF));
    expect(asPlain).toEqual({ kind: 'direct-plain', text: prose, format: 'plain' });
  });

  it('reads application/json bodies as plain text rather than failing extraction', () => {
    const json = '{"answer": 42}';
    const action = expectAction(() =>
      resolveImport({ kind: 'url', body: json, contentType: 'application/json' }, GATE_OFF),
    );
    expect(action).toEqual({ kind: 'direct-plain', text: json, format: 'plain' });
  });
});

describe('js-rendered page detection (spec requirement)', () => {
  function spaShell(mountId: string): string {
    return [
      '<!doctype html><html><head><title>App</title>',
      ...Array.from({ length: SPA_MIN_SCRIPT_TAGS }, (_, i) => `<script src="/chunk-${i}.js"></script>`),
      '</head><body>',
      `<div id="${mountId}"></div>`,
      '<noscript>Enable JavaScript</noscript>',
      '</body></html>',
    ].join('\n');
  }

  it('rejects an empty mount-point shell with scripts as spa_unsupported', () => {
    for (const id of ['root', 'app', '__next']) {
      const err = expectCoded(() =>
        resolveImport({ kind: 'url', body: spaShell(id), contentType: 'text/html' }, GATE_ON),
      );
      expect(err.code).toBe('import.spa_unsupported');
    }
  });

  it('flags a page at the threshold boundary (extracted text = limit - 1)', () => {
    // Pins the strict-inequality side of the heuristic: everything below
    // SPA_MIN_TEXT_CHARS is eligible for flagging when scripts + mount
    // point agree; the hydration test above covers the accepted side. The
    // filler is one exact-length letter run so the extracted char count is
    // deterministic.
    const chars = SPA_MIN_TEXT_CHARS - 1;
    const hydratedShell = spaShell('root').replace(
      '<div id="root"></div>',
      `<div id="root"><p>${'А'.repeat(chars)}</p></div>`,
    );
    const err = expectCoded(() =>
      resolveImport({ kind: 'url', body: hydratedShell, contentType: 'text/html' }, GATE_ON),
    );
    expect(err.code).toBe('import.spa_unsupported');
  });

  it('reports empty_page for script-free markup that extracts nothing', () => {
    const err = expectCoded(() =>
      resolveImport(
        { kind: 'url', body: '<html><body></body></html>', contentType: 'text/html' },
        GATE_OFF,
      ),
    );
    expect(err.code).toBe('import.empty_page');
  });

  it('does not flag pages whose server markup already carries the article (hydration)', () => {
    // Full text present despite scripts AND a known mount id — threshold
    // guard wins before any structural check.
    const hydrated = spaShell('root').replace('<noscript>Enable JavaScript</noscript>', '');
    const action = expectAction(() =>
      resolveImport(
        {
          kind: 'url',
          body: hydrated.replace('<div id="root"></div>', `<div id="root">${'<p>' + 'Полный текст уже в разметке сервера. '.repeat(30) + '</p>'}</div>`),
          contentType: 'text/html',
        },
        GATE_ON,
      ),
    );
    expect(action.kind).toBe('preview');
    if (action.kind === 'preview') expect(action.format).toBe('html');
  });

  it('accepts short partial-SSR pages without a mount point (documented false-negative)', () => {
    const partial = [
      '<html><head><script src="/a.js"></script><script src="/b.js"></script></head>',
      `<body><p>${'Короткий статический фрагмент. '.repeat(10)}</p></body></html>`,
    ].join('');
    const action = expectAction(() =>
      resolveImport({ kind: 'url', body: partial, contentType: 'text/html' }, GATE_ON),
    );
    expect(action.kind).toBe('preview');
  });
});

describe('threshold constants stay in sync with the scenario set', () => {
  it('keeps the allowlist aligned with the backend extension list', () => {
    expect([...IMPORTABLE_FILE_EXTENSIONS].sort()).toEqual(['htm', 'html', 'md', 'txt']);
  });

  it('requires more than one script tag before flagging shells', () => {
    expect(SPA_MIN_SCRIPT_TAGS).toBeGreaterThan(1);
    expect(SPA_MIN_TEXT_CHARS).toBeGreaterThan(0);
  });
});
