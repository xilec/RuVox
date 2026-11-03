import { describe, expect, it } from 'vitest';

import { resolveAddAction, sourceFormatFor } from './addFlow';

const HTML = '<p>Раз <b>два</b></p>';
const PLAIN = 'Раз два';

describe('resolveAddAction', () => {
  it('opens the dialog with the raw HTML when preview is enabled', () => {
    expect(
      resolveAddAction({
        html: HTML,
        plain: PLAIN,
        previewEnabled: true,
      }),
    ).toEqual({ kind: 'preview', text: HTML, plainFallback: PLAIN });
  });

  it('opens the dialog with plain text when only a plain flavor exists', () => {
    expect(
      resolveAddAction({
        html: null,
        plain: PLAIN,
        previewEnabled: true,
      }),
    ).toEqual({ kind: 'preview', text: PLAIN, plainFallback: null });
  });

  it('reports empty when neither flavor has text (preview enabled)', () => {
    expect(
      resolveAddAction({
        html: null,
        plain: '',
        previewEnabled: true,
      }),
    ).toEqual({ kind: 'empty' });
  });

  it('ingests HTML directly when preview is disabled, keeping the plain fallback', () => {
    expect(
      resolveAddAction({
        html: HTML,
        plain: PLAIN,
        previewEnabled: false,
      }),
    ).toEqual({ kind: 'direct-html', html: HTML, plainFallback: PLAIN });
  });

  it('ingests plain text directly when preview is disabled and no HTML flavor exists', () => {
    expect(
      resolveAddAction({
        html: null,
        plain: PLAIN,
        previewEnabled: false,
      }),
    ).toEqual({ kind: 'direct-plain', text: PLAIN });
  });

  it('reports empty when neither flavor has text (preview disabled)', () => {
    expect(
      resolveAddAction({
        html: null,
        plain: '',
        previewEnabled: false,
      }),
    ).toEqual({ kind: 'empty' });
  });

  it('treats whitespace-only flavors as absent', () => {
    expect(
      resolveAddAction({
        html: '   ',
        plain: ' \n ',
        previewEnabled: false,
      }),
    ).toEqual({ kind: 'empty' });
  });

  it('direct-html carries no plain fallback when plain is blank', () => {
    expect(
      resolveAddAction({
        html: HTML,
        plain: '  ',
        previewEnabled: false,
      }),
    ).toEqual({ kind: 'direct-html', html: HTML, plainFallback: null });
  });

  it('preview of auto-detected HTML carries no fallback when plain is blank', () => {
    expect(
      resolveAddAction({
        html: HTML,
        plain: '',
        previewEnabled: true,
      }),
    ).toEqual({ kind: 'preview', text: HTML, plainFallback: null });
  });

  it('preview ignores a whitespace-only HTML flavor and opens with the plain text', () => {
    expect(
      resolveAddAction({
        html: '  ',
        plain: PLAIN,
        previewEnabled: true,
      }),
    ).toEqual({ kind: 'preview', text: PLAIN, plainFallback: null });
  });
});

describe('sourceFormatFor', () => {
  it('uses an explicit ingestion format as recorded', () => {
    expect(sourceFormatFor('## Заголовок', 'plain', undefined)).toBe('plain');
    expect(sourceFormatFor('Просто текст', 'markdown', undefined)).toBe('markdown');
    expect(sourceFormatFor('<p>Раз</p>', 'html', '<p>Раз</p>')).toBe('html');
  });

  it('detects markdown and plain when no format was recorded', () => {
    expect(sourceFormatFor('## Заголовок', undefined, undefined)).toBe('markdown');
    expect(sourceFormatFor('Просто заметка', undefined, undefined)).toBe('plain');
  });

  it('treats a sanitized html source as html even without an explicit format', () => {
    expect(sourceFormatFor('Раз два', undefined, '<p>Раз <b>два</b></p>')).toBe('html');
  });
});
