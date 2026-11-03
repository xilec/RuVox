import { describe, expect, it } from 'vitest';
import type { EntryFormat } from './tauri';
import type { ViewerEntry } from './viewerDisplay';
import {
  deriveSourceFormat,
  resolveDisplayMode,
  viewerModesFor,
} from './viewerDisplay';

function entry(
  fields: Partial<ViewerEntry> & Pick<ViewerEntry, 'original_text'>,
): ViewerEntry {
  return {
    format: null,
    display_mode: null,
    html_source: null,
    ...fields,
  };
}

describe('deriveSourceFormat', () => {
  const markdownText = '# Заголовок\n\nобычный текст';
  const plainText = 'Просто заметка без разметки';
  const htmlText = '<p>Первый</p><p>Второй</p><b>третий</b>';
  const storedHtml = '<p>Первый</p><p>Второй</p><b>третий</b>';

  const cases: ReadonlyArray<{
    name: string;
    input: ViewerEntry;
    expected: EntryFormat;
  }> = [
    {
      name: 'explicit markdown is the source format',
      input: entry({ format: 'markdown', original_text: markdownText }),
      expected: 'markdown',
    },
    {
      name: 'explicit html with stored markup is the source format',
      input: entry({
        format: 'html',
        html_source: storedHtml,
        original_text: 'Первый Второй третий',
      }),
      expected: 'html',
    },
    {
      name: 'explicit plain is trusted even when the text shows markdown signals',
      input: entry({ format: 'plain', original_text: markdownText }),
      expected: 'plain',
    },
    {
      name: 'legacy null format derives markdown from the text',
      input: entry({ original_text: markdownText }),
      expected: 'markdown',
    },
    {
      name: 'legacy null format derives plain from the text',
      input: entry({ original_text: plainText }),
      expected: 'plain',
    },
    {
      name: 'legacy null format derives html from the text',
      input: entry({ original_text: htmlText }),
      expected: 'html',
    },
    {
      name: 'stale html without stored markup clamps to the detected markdown',
      input: entry({ format: 'html', original_text: markdownText }),
      expected: 'markdown',
    },
    {
      name: 'stale html without stored markup clamps to plain',
      input: entry({ format: 'html', original_text: plainText }),
      expected: 'plain',
    },
    {
      name: 'stale markdown on an html entry is clamped by the stored markup',
      input: entry({
        format: 'markdown',
        html_source: storedHtml,
        original_text: 'Первый Второй третий',
      }),
      expected: 'html',
    },
    {
      name: 'stale plain on an html entry is clamped by the stored markup',
      input: entry({
        format: 'plain',
        html_source: storedHtml,
        original_text: 'Первый Второй третий',
      }),
      expected: 'html',
    },
  ];

  for (const { name, input, expected } of cases) {
    it(name, () => {
      expect(deriveSourceFormat(input)).toBe(expected);
    });
  }
});

describe('viewerModesFor', () => {
  it('offers the rendered and verbatim views for markdown', () => {
    expect(viewerModesFor('markdown')).toEqual(['rendered', 'plain']);
  });

  it('offers the rendered and verbatim views for html', () => {
    expect(viewerModesFor('html')).toEqual(['rendered', 'plain']);
  });

  it('offers only the verbatim view for plain', () => {
    expect(viewerModesFor('plain')).toEqual(['plain']);
  });
});

describe('resolveDisplayMode', () => {
  it('defaults to the rendered view when nothing is persisted', () => {
    expect(resolveDisplayMode(entry({ original_text: '# H' }), 'markdown')).toBe(
      'rendered',
    );
  });

  it('restores a persisted verbatim choice', () => {
    expect(
      resolveDisplayMode(
        entry({ display_mode: 'plain', original_text: '# H' }),
        'markdown',
      ),
    ).toBe('plain');
    expect(
      resolveDisplayMode(
        entry({ display_mode: 'rendered', original_text: '# H' }),
        'html',
      ),
    ).toBe('rendered');
  });

  it('clamps a plain source to the verbatim view', () => {
    expect(
      resolveDisplayMode(
        entry({ display_mode: 'rendered', original_text: 'просто текст' }),
        'plain',
      ),
    ).toBe('plain');
  });
});
