import {
  Box,
  Group,
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
import { wrapWordsWithOrigPos } from '../lib/wordSpans';
import classes from './TextViewer.module.css';

// TODO(B1/F4): add `format: "plain" | "markdown" | "html"` to TextEntry schema
// so that the selected format is persisted alongside the entry.  Until that
// schema change lands, the format is ephemeral client-side state.
type Format = "plain" | "markdown" | "html";

interface Props {
  entry: TextEntry | null;
}

export function TextViewer({ entry }: Props) {
  const [format, setFormat] = useState<Format>("markdown");
  const [zoomedSvg, setZoomedSvg] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const colorScheme = useComputedColorScheme("light");

  // Timestamps for the currently playing entry, cached to avoid re-fetching on
  // every playback_position event.
  const timestampsRef = useRef<WordTimestamp[]>([]);
  // Entry id for which timestamps are cached; used to detect entry change.
  const playingEntryIdRef = useRef<string | null>(null);
  // Index of the currently highlighted word, kept in a ref to avoid triggering
  // re-renders on every position event.
  const activeIdxRef = useRef<number>(-1);

  const displayText = entry?.original_text ?? '';

  const content = useMemo(() => {
    if (!entry) return null;
    switch (format) {
      case "plain":
        // Wrap each word in a span with data-orig-* so word-highlighting
        // works in plain mode (same approach as markdown).
        return { __html: plainToWordHtml(displayText) };
      case "html":
        return { __html: renderHtml(displayText) };
      case "markdown":
      default:
        return { __html: renderMarkdown(displayText) };
    }
  }, [entry, displayText, format]);

  // Clear highlight state whenever the displayed entry or format changes so
  // stale highlights do not bleed across navigation.
  useEffect(() => {
    activeIdxRef.current = -1;
    if (containerRef.current) {
      clearHighlight(containerRef.current);
    }
  }, [entry?.id, format]);

  // Prefetch timestamps as soon as the entry has them on disk. Otherwise
  // the highlight pipeline depends on `playback_started` arriving after
  // the listener is registered, but tauri `listen()` is async and
  // autoplay emits `playback_started` inside the same task that emits
  // `entry_updated` — the started event can race the subscription, so
  // highlight never starts until Stop+Play (or a re-subscribe via entry
  // switch) re-fires `playback_started`.
  useEffect(() => {
    if (!entry?.id || !entry.timestamps_path) {
      timestampsRef.current = [];
      playingEntryIdRef.current = null;
      return;
    }
    let cancelled = false;
    commands
      .getTimestamps(entry.id)
      .then((ts) => {
        if (cancelled) return;
        timestampsRef.current = ts;
        playingEntryIdRef.current = entry.id;
      })
      .catch(() => {
        if (cancelled) return;
        timestampsRef.current = [];
        playingEntryIdRef.current = null;
      });
    return () => {
      cancelled = true;
    };
  }, [entry?.id, entry?.timestamps_path]);

  useEffect(() => {
    if (format !== "markdown" || !containerRef.current) return;
    renderMermaidIn(containerRef.current, colorScheme).catch((e) => {
      // Bad mermaid syntax -- keep the raw <div class="mermaid"> as-is
      console.error("mermaid render error:", e);
    });
  }, [content, format, colorScheme]);

  // Ctrl/Cmd+A while focus/selection is inside the viewer should select
  // only the rendered text, not the whole window. Skip when the user is
  // typing in an input/textarea/contentEditable so default behavior wins.
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (!(e.ctrlKey || e.metaKey) || e.key !== 'a') return;
      const container = containerRef.current;
      if (!container) return;
      const active = document.activeElement as HTMLElement | null;
      if (
        active &&
        (active.tagName === 'INPUT' ||
          active.tagName === 'TEXTAREA' ||
          active.isContentEditable)
      ) {
        return;
      }
      const sel = window.getSelection();
      if (!sel || !sel.focusNode || !container.contains(sel.focusNode)) return;
      e.preventDefault();
      const range = document.createRange();
      range.selectNodeContents(container);
      sel.removeAllRanges();
      sel.addRange(range);
    }
    document.addEventListener('keydown', handleKey);
    return () => document.removeEventListener('keydown', handleKey);
  }, []);

  // Click-to-zoom: when user clicks a rendered mermaid SVG, show it in a modal.
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    function handleClick(e: MouseEvent) {
      const target = e.target as HTMLElement;
      const mermaidDiv = target.closest<HTMLElement>(".mermaid");
      if (!mermaidDiv) return;
      const svg = mermaidDiv.querySelector("svg");
      if (!svg) return;
      setZoomedSvg(svg.outerHTML);
    }

    container.addEventListener("click", handleClick);
    return () => container.removeEventListener("click", handleClick);
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
        try {
          const ts = await commands.getTimestamps(entry_id);
          timestampsRef.current = ts;
          playingEntryIdRef.current = entry_id;
          activeIdxRef.current = -1;
        } catch {
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

        if (!entry || entry.id !== entry_id) return;
        if (playingEntryIdRef.current !== entry_id) return;

        const timestamps = timestampsRef.current;
        if (timestamps.length === 0) return;

        // Plain mode now emits data-orig-* word spans, so highlighting works.
        // HTML mode still uses HtmlCharSpan sentinel (0/0) — disabled below.
        // TODO(U5): emit a proper char-mapping from the HTML pipeline.
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
        // Keep highlight visible while paused; do not reset.
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
  // entry.id and format are intentionally included so we re-subscribe when
  // the viewer switches entry/format.
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
    <Stack gap="sm" style={{ height: '100%', minHeight: 0 }}>
      <Group justify="space-between" wrap="nowrap">
        <SegmentedControl
          value={format}
          onChange={(v) => setFormat(v as Format)}
          size="xs"
          data={[
            { label: "Plain", value: "plain" },
            { label: "Markdown", value: "markdown" },
            { label: "HTML", value: "html" },
          ]}
        />
      </Group>

      <ScrollArea style={{ flex: 1 }}>
        <Box
          ref={containerRef}
          className={classes.content}
          dangerouslySetInnerHTML={content ?? { __html: "" }}
        />
      </ScrollArea>

      <Modal
        opened={zoomedSvg !== null}
        onClose={() => setZoomedSvg(null)}
        size="xl"
        title="Mermaid diagram"
        styles={{ body: { overflowX: "auto" } }}
      >
        {zoomedSvg && (
          <Box
            dangerouslySetInnerHTML={{ __html: zoomedSvg }}
            style={{ display: "flex", justifyContent: "center" }}
          />
        )}
      </Modal>
    </Stack>
  );
}

function plainToWordHtml(s: string): string {
  // Split on newlines so we can insert <br> between lines while still
  // wrapping each word in a data-orig-* span.  Offsets track the position of
  // each line within the original source text.
  const lines = s.split('\n');
  const parts: string[] = [];
  let offset = 0;
  for (let i = 0; i < lines.length; i += 1) {
    parts.push(wrapWordsWithOrigPos(lines[i], offset));
    offset += lines[i].length + 1; // +1 for the consumed \n
    if (i < lines.length - 1) parts.push('<br>');
  }
  return parts.join('');
}
