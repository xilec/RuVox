import { Box, ScrollArea, SegmentedControl, Stack, Text } from '@mantine/core';
import { useMemo, useState } from 'react';
import type { TextEntry } from '../lib/tauri';
import { renderMarkdown } from '../lib/markdown';
import { renderHtml } from '../lib/html';
import classes from './TextViewer.module.css';

// TODO(B1/F4): add `format: 'plain' | 'markdown' | 'html'` to TextEntry schema
// so that the selected format is persisted alongside the entry.  Until that
// schema change lands, the format is ephemeral client-side state.
type Format = 'plain' | 'markdown' | 'html';

interface Props {
  entry: TextEntry | null;
}

export function TextViewer({ entry }: Props) {
  const [format, setFormat] = useState<Format>('markdown');

  const text = entry?.edited_text ?? entry?.original_text ?? '';

  const content = useMemo(() => {
    if (!entry) return null;
    switch (format) {
      case 'plain':
        return { __html: escapeHtml(text) };
      case 'html':
        return { __html: renderHtml(text) };
      case 'markdown':
      default:
        return { __html: renderMarkdown(text) };
    }
  }, [entry, text, format]);

  if (!entry) {
    return (
      <Stack h="100%">
        <Text className={classes.placeholder}>Нет выбранной записи</Text>
      </Stack>
    );
  }

  return (
    <Stack gap="sm" h="100%">
      <SegmentedControl
        value={format}
        onChange={(v) => setFormat(v as Format)}
        size="xs"
        data={[
          { label: 'Plain', value: 'plain' },
          { label: 'Markdown', value: 'markdown' },
          { label: 'HTML', value: 'html' },
        ]}
      />
      <ScrollArea style={{ flex: 1 }}>
        <Box
          className={classes.content}
          dangerouslySetInnerHTML={content ?? { __html: '' }}
        />
      </ScrollArea>
    </Stack>
  );
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/\n/g, '<br>');
}
