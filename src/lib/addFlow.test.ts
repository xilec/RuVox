import { describe, expect, it } from 'vitest';

import { resolveAddAction } from './addFlow';

const HTML = '<p>Раз <b>два</b></p>';
const PLAIN = 'Раз два';

describe('resolveAddAction', () => {
  it('opens the dialog with the raw HTML and the html selector when preview is enabled', () => {
    expect(
      resolveAddAction({
        html: HTML,
        plain: PLAIN,
        previewEnabled: true,
        defaultFormat: 'plain',
      }),
    ).toEqual({ kind: 'preview', text: HTML, format: 'html', plainFallback: PLAIN });
  });

  it('opens the dialog with plain text and the configured default format', () => {
    expect(
      resolveAddAction({
        html: null,
        plain: PLAIN,
        previewEnabled: true,
        defaultFormat: 'markdown',
      }),
    ).toEqual({ kind: 'preview', text: PLAIN, format: 'markdown', plainFallback: null });
  });

  it('reports empty when neither flavor has text (preview enabled)', () => {
    expect(
      resolveAddAction({
        html: null,
        plain: '',
        previewEnabled: true,
        defaultFormat: 'plain',
      }),
    ).toEqual({ kind: 'empty' });
  });

  it('ingests HTML directly when preview is disabled, keeping the plain fallback', () => {
    expect(
      resolveAddAction({
        html: HTML,
        plain: PLAIN,
        previewEnabled: false,
        defaultFormat: 'plain',
      }),
    ).toEqual({ kind: 'direct-html', html: HTML, plainFallback: PLAIN });
  });

  it('ingests plain text directly when preview is disabled and no HTML flavor exists', () => {
    expect(
      resolveAddAction({
        html: null,
        plain: PLAIN,
        previewEnabled: false,
        defaultFormat: 'plain',
      }),
    ).toEqual({ kind: 'direct-plain', text: PLAIN });
  });

  it('reports empty when neither flavor has text (preview disabled)', () => {
    expect(
      resolveAddAction({
        html: null,
        plain: '',
        previewEnabled: false,
        defaultFormat: 'plain',
      }),
    ).toEqual({ kind: 'empty' });
  });

  it('treats whitespace-only flavors as absent', () => {
    expect(
      resolveAddAction({
        html: '   ',
        plain: ' \n ',
        previewEnabled: false,
        defaultFormat: 'plain',
      }),
    ).toEqual({ kind: 'empty' });
  });

  it('direct-html carries no plain fallback when plain is blank', () => {
    expect(
      resolveAddAction({
        html: HTML,
        plain: '  ',
        previewEnabled: false,
        defaultFormat: 'plain',
      }),
    ).toEqual({ kind: 'direct-html', html: HTML, plainFallback: null });
  });

  it('preview of auto-detected HTML carries no fallback when plain is blank', () => {
    expect(
      resolveAddAction({
        html: HTML,
        plain: '',
        previewEnabled: true,
        defaultFormat: 'plain',
      }),
    ).toEqual({ kind: 'preview', text: HTML, format: 'html', plainFallback: null });
  });

  it('preview ignores a whitespace-only HTML flavor and opens with the plain text', () => {
    expect(
      resolveAddAction({
        html: '  ',
        plain: PLAIN,
        previewEnabled: true,
        defaultFormat: 'markdown',
      }),
    ).toEqual({ kind: 'preview', text: PLAIN, format: 'markdown', plainFallback: null });
  });
});
