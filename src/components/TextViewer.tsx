import {
  Box,
  Modal,
  ScrollArea,
  SegmentedControl,
  Stack,
  Text,
  useComputedColorScheme,
} from '@mantine/core';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { TextEntry } from '../lib/tauri';
import { renderMarkdown } from '../lib/markdown';
import { renderMermaidIn } from '../lib/mermaid';
import classes from './TextViewer.module.css';

type Format = 'plain' | 'markdown';

interface Props {
  entry: TextEntry | null;
}

export function TextViewer({ entry }: Props) {
  const [format, setFormat] = useState<Format>('markdown');
  const [zoomedSvg, setZoomedSvg] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const colorScheme = useComputedColorScheme('light');

  const text = entry?.edited_text ?? entry?.original_text ?? '';

  const content = useMemo(() => {
    if (!entry) return null;
    if (format === 'plain') return { __html: escapeHtml(text) };
    return { __html: renderMarkdown(text) };
  }, [entry, text, format]);

  useEffect(() => {
    if (format !== 'markdown' || !containerRef.current) return;
    renderMermaidIn(containerRef.current, colorScheme).catch((e) => {
      // Bad mermaid syntax — keep the raw <div class="mermaid"> as-is
      console.error('mermaid render error:', e);
    });
  }, [content, format, colorScheme]);

  // Click-to-zoom: when user clicks a rendered mermaid SVG, show it in a modal.
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    function handleClick(e: MouseEvent) {
      const target = e.target as HTMLElement;
      const mermaidDiv = target.closest<HTMLElement>('.mermaid');
      if (!mermaidDiv) return;
      const svg = mermaidDiv.querySelector('svg');
      if (!svg) return;
      setZoomedSvg(svg.outerHTML);
    }

    container.addEventListener('click', handleClick);
    return () => container.removeEventListener('click', handleClick);
  }, []);

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
        ]}
      />
      <ScrollArea style={{ flex: 1 }}>
        <Box
          ref={containerRef}
          className={classes.content}
          dangerouslySetInnerHTML={content ?? { __html: '' }}
        />
      </ScrollArea>

      <Modal
        opened={zoomedSvg !== null}
        onClose={() => setZoomedSvg(null)}
        size="xl"
        title="Mermaid diagram"
        styles={{ body: { overflowX: 'auto' } }}
      >
        {zoomedSvg && (
          <Box
            dangerouslySetInnerHTML={{ __html: zoomedSvg }}
            style={{ display: 'flex', justifyContent: 'center' }}
          />
        )}
      </Modal>
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
