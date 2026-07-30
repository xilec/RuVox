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
import { notifications } from '@mantine/notifications';
import type { EntryFormat, TextEntry, WordTimestamp } from '../lib/tauri';
import { commands, events } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { renderMarkdown } from '../lib/markdown';
import { renderHtml } from '../lib/html';
import { renderMermaidIn } from '../lib/mermaid';
import {
  findActiveTimestamp,
  applyHighlight,
  clearHighlight,
  debugAssertSortedTimestamps,
} from '../lib/wordHighlight';
import { plainToWordHtml } from '../lib/plainTextHtml';
import classes from './TextViewer.module.css';

// Entries without a persisted format render in the viewer default mode.
const DEFAULT_FORMAT: EntryFormat = "markdown";

interface Props {
  entry: TextEntry | null;
}

export function TextViewer({ entry }: Props) {
  const [format, setFormat] = useState<EntryFormat>(DEFAULT_FORMAT);
  const [zoomedSvg, setZoomedSvg] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const colorScheme = useComputedColorScheme("light");

  // Restore the persisted display mode when another entry is selected or
  // the entry's saved format changes (e.g. after set_entry_format lands).
  useEffect(() => {
    setFormat(entry?.format ?? DEFAULT_FORMAT);
  }, [entry?.id, entry?.format]);

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
        // HTML-ingested entries render their sanitized source; entries that
        // only have plain text (e.g. toggled to HTML manually) fall back to
        // the original text.
        return { __html: renderHtml(entry.html_source ?? displayText) };
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
        debugAssertSortedTimestamps(ts);
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

    void events
      .playbackStarted(async ({ entry_id }) => {
        try {
          const ts = await commands.getTimestamps(entry_id);
          debugAssertSortedTimestamps(ts);
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

    void events
      .playbackPosition(({ position_sec, entry_id }) => {
        const container = containerRef.current;
        if (!container) return;

        if (!entry || entry.id !== entry_id) return;
        if (playingEntryIdRef.current !== entry_id) return;

        const timestamps = timestampsRef.current;
        if (timestamps.length === 0) return;

        // All three display modes emit data-orig-* word spans (HTML mode
        // gets them from annotateHtmlWords over the sanitized source).
        // Exception: a plain-text entry manually toggled to HTML renders a
        // whitespace-collapsed fallback, whose span offsets do not match
        // WordTimestamp.original_pos — highlighting would be misleading.
        if (format === 'html' && !entry.html_source) return;

        const newIdx = findActiveTimestamp(timestamps, position_sec);
        const prevIdx = activeIdxRef.current;

        if (newIdx === prevIdx) return;

        activeIdxRef.current = newIdx;
        applyHighlight(container, timestamps, newIdx, prevIdx);
      })
      .then((fn) => {
        unlistenPosition = fn;
      });

    void events
      .playbackStopped(resetHighlight)
      .then((fn) => {
        unlistenStopped = fn;
      });

    void events
      .playbackFinished(resetHighlight)
      .then((fn) => {
        unlistenFinished = fn;
      });

    void events
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

  // Optimistic local switch; persist on the entry and revert on failure.
  function handleFormatChange(v: string) {
    if (!entry) return;
    const next = v as EntryFormat;
    const prev = format;
    setFormat(next);
    commands.setEntryFormat(entry.id, next).catch((err) => {
      setFormat(prev);
      notifications.show({ title: 'Ошибка', message: formatError(err), color: 'red' });
    });
  }

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
          onChange={handleFormatChange}
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

