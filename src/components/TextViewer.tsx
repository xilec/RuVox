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
import type { TextEntry, WordTimestamp } from '../lib/tauri';
import { commands, events } from '../lib/tauri';
import { renderMarkdown } from '../lib/markdown';
import { renderHtml } from '../lib/html';
import { renderMermaidIn } from '../lib/mermaid';
import {
  findActiveTimestamp,
  applyHighlight,
  clearHighlight,
} from '../lib/wordHighlight';
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
  const [zoomedSvg, setZoomedSvg] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const colorScheme = useComputedColorScheme('light');

  // Timestamps for the currently playing entry, cached to avoid re-fetching on
  // every playback_position event.
  const timestampsRef = useRef<WordTimestamp[]>([]);
  // Entry id for which timestamps are cached; used to detect entry change.
  const playingEntryIdRef = useRef<string | null>(null);
  // Index of the currently highlighted word, kept in a ref to avoid triggering
  // re-renders on every position event.
  const activeIdxRef = useRef<number>(-1);

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

  // Clear highlight state whenever the displayed entry or format changes so
  // stale highlights do not bleed across navigation.
  useEffect(() => {
    activeIdxRef.current = -1;
    if (containerRef.current) {
      clearHighlight(containerRef.current);
    }
  }, [entry?.id, format]);

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

  // Subscribe to playback events for word highlighting.
  useEffect(() => {
    let unlistenStarted: (() => void) | null = null;
    let unlistenPosition: (() => void) | null = null;
    let unlistenStopped: (() => void) | null = null;
    let unlistenFinished: (() => void) | null = null;
    let unlistenPaused: (() => void) | null = null;

    function resetHighlight() {
      activeIdxRef.current = -1;
      playingEntryIdRef.current = null;
      timestampsRef.current = [];
      if (containerRef.current) {
        clearHighlight(containerRef.current);
      }
    }

    events
      .playbackStarted(async ({ entry_id }) => {
        // Fetch timestamps for the new entry. If the entry has no timestamps
        // file yet (synthesis still running), the command returns [].
        try {
          const ts = await commands.getTimestamps(entry_id);
          timestampsRef.current = ts;
          playingEntryIdRef.current = entry_id;
          activeIdxRef.current = -1;
        } catch {
          // Non-fatal: playback continues without highlighting.
          timestampsRef.current = [];
          playingEntryIdRef.current = entry_id;
          activeIdxRef.current = -1;
        }
      })
      .then((fn) => {
        unlistenStarted = fn;
      });

    events
      .playbackPosition(({ position_sec, entry_id }) => {
        const container = containerRef.current;
        if (!container) return;

        // Only highlight when the viewer is showing the playing entry.
        if (!entry || entry.id !== entry_id) return;

        // If we do not have timestamps for this entry yet, skip silently.
        if (playingEntryIdRef.current !== entry_id) return;

        const timestamps = timestampsRef.current;
        if (timestamps.length === 0) return;

        // Plain mode has no data-orig-* spans — highlighting is skipped.
        if (format === 'plain') return;

        // TODO(U5): HTML mode uses HtmlCharSpan sentinel (0/0) which maps all
        // words to the same position. Highlighting is disabled for HTML mode
        // until a proper char-mapping is implemented in the HTML pipeline.
        if (format === 'html') return;

        const newIdx = findActiveTimestamp(timestamps, position_sec);
        const prevIdx = activeIdxRef.current;

        if (newIdx === prevIdx) return;

        activeIdxRef.current = newIdx;
        applyHighlight(container, timestamps, newIdx, prevIdx);
      })
      .then((fn) => {
        unlistenPosition = fn;
      });

    events
      .playbackStopped(resetHighlight)
      .then((fn) => {
        unlistenStopped = fn;
      });

    events
      .playbackFinished(resetHighlight)
      .then((fn) => {
        unlistenFinished = fn;
      });

    events
      .playbackPaused(() => {
        // Keep the highlight visible while paused; do not reset.
      })
      .then((fn) => {
        unlistenPaused = fn;
      });

    return () => {
      unlistenStarted?.();
      unlistenPosition?.();
      unlistenStopped?.();
      unlistenFinished?.();
      unlistenPaused?.();
    };
  // entry.id and format are intentionally included: if the viewer switches to
  // a different entry or format we need the position handler to re-evaluate.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entry?.id, format]);

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
