import {
  ActionIcon,
  Box,
  Group,
  Modal,
  ScrollArea,
  SegmentedControl,
  Stack,
  Text,
  Textarea,
  Tooltip,
  useComputedColorScheme,
} from '@mantine/core';
import { notifications } from '@mantine/notifications';
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
  const [editMode, setEditMode] = useState(false);
  const [editedDraft, setEditedDraft] = useState("");
  const [saving, setSaving] = useState(false);
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

  // Keep draft in sync when entry changes and exit edit mode on selection change.
  useEffect(() => {
    if (entry) {
      setEditedDraft(entry.edited_text ?? entry.original_text);
    }
    setEditMode(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entry?.id]);

  const displayText = entry?.edited_text ?? entry?.original_text ?? '';

  const content = useMemo(() => {
    if (!entry) return null;
    switch (format) {
      case "plain":
        return { __html: escapeHtml(displayText) };
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

  useEffect(() => {
    if (editMode) return;
    if (format !== "markdown" || !containerRef.current) return;
    renderMermaidIn(containerRef.current, colorScheme).catch((e) => {
      // Bad mermaid syntax -- keep the raw <div class="mermaid"> as-is
      console.error("mermaid render error:", e);
    });
  }, [content, format, colorScheme, editMode]);

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
    // editMode swaps rendered DOM for a Textarea — no data-orig-* spans exist,
    // so highlighting must sit out until the user exits edit mode.
    if (editMode) return;

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

        if (format === 'plain') return;
        // TODO(U5): HTML mode uses HtmlCharSpan sentinel (0/0) — highlighting
        // disabled until HTML pipeline emits a proper char-mapping.
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
  // entry.id, format and editMode are intentionally included so we re-subscribe
  // when the viewer switches entry/format, or exits edit mode.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entry?.id, format, editMode]);

  // Keyboard: Esc cancels edit mode.
  useEffect(() => {
    if (!editMode) return;
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        handleCancel();
      }
    }
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editMode, entry]);

  function handleEnterEdit() {
    if (!entry) return;
    setEditedDraft(entry.edited_text ?? entry.original_text);
    setEditMode(true);
  }

  function handleCancel() {
    if (!entry) return;
    setEditedDraft(entry.edited_text ?? entry.original_text);
    setEditMode(false);
  }

  async function handleSave() {
    if (!entry) return;
    setSaving(true);
    try {
      await commands.updateEntryEditedText(entry.id, editedDraft);
      notifications.show({
        color: 'green',
        message: 'Сохранено',
        autoClose: 2000,
      });
      setEditMode(false);
    } catch (err) {
      notifications.show({
        color: 'red',
        title: 'Ошибка сохранения',
        message: String(err),
      });
    } finally {
      setSaving(false);
    }
  }

  if (!entry) {
    return (
      <Stack h="100%">
        <Text className={classes.placeholder}>Нет выбранной записи</Text>
      </Stack>
    );
  }

  return (
    <Stack gap="sm" h="100%">
      <Group justify="space-between" wrap="nowrap">
        <SegmentedControl
          value={format}
          onChange={(v) => setFormat(v as Format)}
          size="xs"
          disabled={editMode}
          data={[
            { label: "Plain", value: "plain" },
            { label: "Markdown", value: "markdown" },
            { label: "HTML", value: "html" },
          ]}
        />
        <Group gap="xs" wrap="nowrap">
          {editMode ? (
            <>
              <Tooltip label="Сохранить">
                <ActionIcon
                  variant="filled"
                  color="green"
                  size="sm"
                  loading={saving}
                  onClick={handleSave}
                  aria-label="Сохранить правки"
                >
                  &#x2713;
                </ActionIcon>
              </Tooltip>
              <Tooltip label="Отмена (Esc)">
                <ActionIcon
                  variant="subtle"
                  color="gray"
                  size="sm"
                  onClick={handleCancel}
                  aria-label="Отменить правки"
                >
                  &#x2715;
                </ActionIcon>
              </Tooltip>
            </>
          ) : (
            <Tooltip label="Редактировать текст">
              <ActionIcon
                variant="subtle"
                color="gray"
                size="sm"
                onClick={handleEnterEdit}
                aria-label="Редактировать"
              >
                &#x270F;
              </ActionIcon>
            </Tooltip>
          )}
        </Group>
      </Group>

      {editMode ? (
        <Textarea
          value={editedDraft}
          onChange={(e) => setEditedDraft(e.currentTarget.value)}
          autosize
          minRows={10}
          style={{ flex: 1 }}
          styles={{ input: { fontFamily: "var(--mantine-font-family)", lineHeight: 1.6 } }}
        />
      ) : (
        <ScrollArea style={{ flex: 1 }}>
          <Box
            ref={containerRef}
            className={classes.content}
            dangerouslySetInnerHTML={content ?? { __html: "" }}
          />
        </ScrollArea>
      )}

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

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/\n/g, "<br>");
}
