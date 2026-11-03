import {
  Box,
  Modal,
  ScrollArea,
  SegmentedControl,
  Stack,
  Text,
  useComputedColorScheme,
} from '@mantine/core';
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent as ReactMouseEvent,
} from 'react';
import { notifications } from '@mantine/notifications';
import type { DisplayMode, ReadingWidth, TextEntry, WordTimestamp } from '../lib/tauri';
import { commands, events } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { useT } from '../lib/i18n';
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
import { makeInert } from '../lib/inertContent';
import { copyLinkAddress } from '../lib/viewerCopy';
import {
  canonicalReadingWidth,
  clampReadingScale,
  READING_SCALE_DEFAULT,
  readingZoom,
} from '../lib/reading';
import {
  deriveSourceFormat,
  resolveDisplayMode,
  viewerModesFor,
} from '../lib/viewerDisplay';
import { ReadingScaleControl, ReadingWidthControl } from './ReadingControls';
import { EntryHeader } from './EntryHeader';
import { ViewerContextMenu } from './ViewerContextMenu';
import classes from './TextViewer.module.css';

// Keep links inside the mermaid zoom modal inert, mirroring the delegated
// handler that covers the viewer container. auxclick is included because
// middle click fires auxclick, not click, and would bypass a click-only block.
function blockZoomLinkClick(e: ReactMouseEvent) {
  if ((e.target as HTMLElement).closest("a")) {
    e.preventDefault();
    e.stopPropagation();
  }
}

interface ReadingConfigPatch {
  reading_width?: ReadingWidth;
  reading_scale?: number;
}

interface Props {
  entry: TextEntry | null;
  /** Persisted reading-column width from UIConfig; canonicalized on seed. */
  readingWidth?: ReadingWidth;
  /** Persisted reading-scale percentage from UIConfig; clamped on seed. */
  readingScale?: number;
  /** Reports a persisted reading-typography patch so AppShell's config mirror
   *  follows the toolbar (the Settings dialog refreshes it through onSaved). */
  onReadingConfigChange?: (patch: ReadingConfigPatch) => void;
}

export function TextViewer({
  entry,
  readingWidth: readingWidthProp,
  readingScale: readingScaleProp = READING_SCALE_DEFAULT,
  onReadingConfigChange,
}: Props) {
  const tt = useT();
  // Local mirror of the persisted reading typography: seeded from the config
  // props so the first paint is correct, then updated optimistically by the
  // toolbar and re-synced when Settings saves a new config (AppShell refreshes
  // the values in onSaved).
  const [readingWidth, setReadingWidth] = useState<ReadingWidth>(() =>
    canonicalReadingWidth(readingWidthProp),
  );
  const [readingScale, setReadingScale] = useState(() =>
    clampReadingScale(readingScaleProp),
  );
  // Last values known to be persisted; a rejected latest write rolls back to
  // these instead of to the optimistic value captured at click time.
  const persistedWidth = useRef(canonicalReadingWidth(readingWidthProp));
  const persistedScale = useRef(clampReadingScale(readingScaleProp));

  useEffect(() => {
    const width = canonicalReadingWidth(readingWidthProp);
    persistedWidth.current = width;
    setReadingWidth(width);
  }, [readingWidthProp]);
  useEffect(() => {
    const scale = clampReadingScale(readingScaleProp);
    persistedScale.current = scale;
    setReadingScale(scale);
  }, [readingScaleProp]);
  // Seed from the entry so the first paint already shows the persisted
  // choice: a post-paint effect would render the default view once and could
  // flash the wrong format (e.g. markdown/mermaid before «Исходный»).
  const [displayMode, setDisplayMode] = useState<DisplayMode>(() =>
    entry ? resolveDisplayMode(entry, deriveSourceFormat(entry)) : 'rendered',
  );
  const [zoomedSvg, setZoomedSvg] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const colorScheme = useComputedColorScheme("light");

  // The source format is immutable; the viewer derives the allowed views
  // from it and restores the persisted display choice when another entry is
  // selected or the saved mode changes (e.g. after set_entry_display_mode
  // lands). A plain source always resolves to the verbatim view.
  const sourceFormat = useMemo(
    () => (entry ? deriveSourceFormat(entry) : 'markdown'),
    [entry],
  );

  useEffect(() => {
    setDisplayMode(entry ? resolveDisplayMode(entry, sourceFormat) : 'rendered');
  // Only the derived fields matter; re-running on every entry_updated event
  // would reset an in-flight optimistic switch.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entry?.id, entry?.format, entry?.html_source, entry?.original_text, entry?.display_mode, sourceFormat]);
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
    if (sourceFormat === 'plain' || displayMode === 'plain') {
      // Verbatim view: markup stays visible. Every word is wrapped in a span
      // with data-orig-* so word highlighting works (same as markdown).
      return { __html: plainToWordHtml(displayText) };
    }
    if (sourceFormat === 'html') {
      // HTML-ingested entries render their sanitized source. A legacy entry
      // whose source was inferred from content has no stored markup and
      // falls back to its original text.
      return { __html: renderHtml(entry.html_source ?? displayText) };
    }
    return { __html: renderMarkdown(displayText) };
  }, [entry, displayText, displayMode, sourceFormat]);

  // Read-only viewer (text-display spec): neutralize interactive elements
  // after every content render. Runs post-commit over the mounted DOM, in
  // all display modes.
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    makeInert(container);
    // Table scroll boxes are focusable so a wide table can be scrolled from
    // the keyboard (#364 review). The renderers emit `tabindex`; the localized
    // region name lives here, where the active catalog is available.
    const scrollLabel = tt('viewer.table.scroll_label');
    container.querySelectorAll<HTMLElement>('[data-scroll-x]').forEach((box) => {
      box.setAttribute('role', 'region');
      box.setAttribute('aria-label', scrollLabel);
      box.setAttribute('tabindex', '0');
    });
  }, [content, tt]);

  // Clear highlight state whenever the displayed entry or view changes so
  // stale highlights do not bleed across navigation.
  useEffect(() => {
    activeIdxRef.current = -1;
    if (containerRef.current) {
      clearHighlight(containerRef.current);
    }
  }, [entry?.id, displayMode, sourceFormat]);

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
    if (
      sourceFormat !== "markdown" ||
      displayMode !== "rendered" ||
      !containerRef.current
    ) {
      return;
    }
    renderMermaidIn(containerRef.current, colorScheme).catch((e) => {
      // Bad mermaid syntax -- keep the raw <div class="mermaid"> as-is
      console.error("mermaid render error:", e);
    });
  }, [content, sourceFormat, displayMode, colorScheme]);

  // Ctrl/Cmd+A while focus/selection is inside the viewer should select
  // only the rendered text, not the whole window. Skip when the user is
  // typing in an input/textarea/contentEditable so default behavior wins.
  // Ctrl/Cmd+C on a link copies the link URL instead (viewer-copy-actions
  // spec); other Ctrl/Cmd+C presses keep the default copy behavior.
  // Matching by e.code (physical key), not e.key: under the Russian layout
  // e.key is 'с'/'ф' and the hotkeys would silently stop working.
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (!(e.ctrlKey || e.metaKey)) return;
      const container = containerRef.current;
      if (!container) return;

      if (e.code === 'KeyC') {
        const active = document.activeElement as HTMLElement | null;
        const focusedLink =
          active && container.contains(active)
            ? active.closest('a[href]')
            : null;
        const anchorNode = window.getSelection()?.anchorNode ?? null;
        const anchorEl =
          anchorNode instanceof Element ? anchorNode : anchorNode?.parentElement;
        const selectedLink = anchorEl?.closest('a[href]') ?? null;
        const link =
          focusedLink ??
          (selectedLink && container.contains(selectedLink)
            ? selectedLink
            : null);
        if (link) {
          e.preventDefault();
          void copyLinkAddress(link.getAttribute('href') ?? '');
        }
        return;
      }

      if (e.code !== 'KeyA') return;
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

  // Read-only viewer (text-display spec): block link navigation and
  // <details> toggling; keep the mermaid click-to-zoom behavior.
  // Deps on entry?.id: on first mount there is often no entry yet, so the
  // viewer renders a placeholder without the container — an effect with
  // empty deps would see containerRef.current === null and never attach.
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    // Links and <summary> toggles are inert on any mouse button — middle
    // click fires auxclick, not click, and would bypass a click-only block.
    function blockInert(e: MouseEvent): boolean {
      const target = e.target as HTMLElement;
      if (target.closest("a") || target.closest("summary")) {
        e.preventDefault();
        e.stopPropagation();
        return true;
      }
      return false;
    }

    function handleClick(e: MouseEvent) {
      if (blockInert(e)) return;
      const target = e.target as HTMLElement;
      const mermaidDiv = target.closest<HTMLElement>(".mermaid");
      if (!mermaidDiv) return;
      const svg = mermaidDiv.querySelector("svg");
      if (!svg) return;
      setZoomedSvg(svg.outerHTML);
    }

    container.addEventListener("click", handleClick);
    container.addEventListener("auxclick", blockInert);
    return () => {
      container.removeEventListener("click", handleClick);
      container.removeEventListener("auxclick", blockInert);
    };
  }, [entry?.id]);

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

        // Every rendered mode emits data-orig-* word spans (HTML mode gets
        // them from annotateHtmlWords over the sanitized source).
        // Exception: an entry whose html source was inferred from content has
        // no stored markup, so its whitespace-collapsed fallback spans do not
        // match WordTimestamp.original_pos — highlighting would be misleading.
        if (
          displayMode === 'rendered' &&
          sourceFormat === 'html' &&
          !entry.html_source
        ) {
          return;
        }

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
  // entry.id and the view are intentionally included so we re-subscribe when
  // the viewer switches entry or renders the source differently.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entry?.id, displayMode, sourceFormat]);

  // Optimistic local switch; persist on the entry and revert on failure.
  function handleDisplayModeChange(v: string) {
    if (!entry) return;
    const next = v as DisplayMode;
    if (next === displayMode) return;
    const prev = displayMode;
    setDisplayMode(next);
    commands.setEntryDisplayMode(entry.id, next).catch((err) => {
      setDisplayMode(prev);
      notifications.show({ title: tt('errors.title'), message: formatError(err), color: 'red' });
    });
  }

  // Persisted reading-typography writes are serialized through one chain and
  // tagged with a monotonic per-field request id (#364 review). Two quick
  // clicks used to race: a rejection that settled after a newer write rolled
  // the control back to a value the user had already moved past. The chain
  // keeps backend writes in click order; the id turns a superseded rejection
  // into a no-op instead of a rollback.
  const readingWriteChain = useRef<Promise<void>>(Promise.resolve());
  const widthRequestId = useRef(0);
  const scaleRequestId = useRef(0);

  function persistReadingPatch(
    patch: ReadingConfigPatch,
    isLatest: () => boolean,
    onSuccess: () => void,
    onReject: () => void,
  ) {
    readingWriteChain.current = readingWriteChain.current.then(async () => {
      try {
        await commands.updateConfig(patch);
        onSuccess();
      } catch (err) {
        if (!isLatest()) return;
        onReject();
        notifications.show({
          title: tt('errors.title'),
          message: formatError(err),
          color: 'red',
        });
      }
    });
  }

  // Optimistic reading-typography update: the toolbar is the fast path, so
  // each click applies immediately and persists through update_config; only a
  // rejected latest write reverts the control and surfaces a localized toast.
  function handleReadingWidthChange(next: ReadingWidth) {
    if (next === readingWidth) return;
    const requestId = (widthRequestId.current += 1);
    setReadingWidth(next);
    persistReadingPatch(
      { reading_width: next },
      () => requestId === widthRequestId.current,
      () => {
        persistedWidth.current = next;
        onReadingConfigChange?.({ reading_width: next });
      },
      () => setReadingWidth(persistedWidth.current),
    );
  }

  function handleReadingScaleChange(next: number) {
    const scale = clampReadingScale(next);
    if (scale === readingScale) return;
    const requestId = (scaleRequestId.current += 1);
    setReadingScale(scale);
    persistReadingPatch(
      { reading_scale: scale },
      () => requestId === scaleRequestId.current,
      () => {
        persistedScale.current = scale;
        onReadingConfigChange?.({ reading_scale: scale });
      },
      () => setReadingScale(persistedScale.current),
    );
  }

  if (!entry) {
    return (
      <Stack h="100%">
        <Text className={classes.placeholder}>{tt('viewer.no_entry')}</Text>
      </Stack>
    );
  }

  return (
    <Stack gap={0} style={{ height: '100%', minHeight: 0 }}>
      {/* Entry identity and metadata above the scroll area (#353), so the
          title, note and status stay visible while reading. The format
          switch moved here from the old toolbar; the reading controls
          (#354/#357) stay next to it, in the same meta row, and remain
          available in every display mode. */}
      <EntryHeader
        key={entry.id}
        entry={entry}
        controls={
          <>
            {viewerModesFor(sourceFormat).includes('rendered') && (
              <SegmentedControl
                name="viewer_display_mode"
                value={displayMode}
                onChange={handleDisplayModeChange}
                size="xs"
                data={[
                  {
                    label:
                      sourceFormat === 'html'
                        ? tt('viewer.display_mode.html')
                        : tt('viewer.display_mode.markdown'),
                    value: 'rendered',
                  },
                  { label: tt('viewer.display_mode.plain'), value: 'plain' },
                ]}
              />
            )}
            <ReadingWidthControl
              name="viewer_reading_width"
              value={readingWidth}
              onChange={handleReadingWidthChange}
            />
            <ReadingScaleControl
              value={readingScale}
              onChange={handleReadingScaleChange}
            />
          </>
        }
      />

      <ScrollArea style={{ flex: 1 }}>
        <Box
          ref={containerRef}
          className={classes.content}
          data-reading-width={readingWidth}
          style={{
            '--ruvox-reading-scale': String(readingZoom(readingScale)),
          }}
          dangerouslySetInnerHTML={content ?? { __html: "" }}
        />
      </ScrollArea>

      <ViewerContextMenu containerRef={containerRef} />

      <Modal
        opened={zoomedSvg !== null}
        onClose={() => setZoomedSvg(null)}
        size="xl"
        title={tt('viewer.mermaid_zoom_title')}
        styles={{ body: { overflowX: "auto" } }}
      >
        {zoomedSvg && (
          // The modal is portaled outside containerRef, so the delegated
          // read-only click handler does not reach it: block link navigation
          // inside the zoomed SVG here (mermaid `click` directives emit real
          // <a> elements under securityLevel 'loose').
          <Box
            onClick={blockZoomLinkClick}
            onAuxClick={blockZoomLinkClick}
            dangerouslySetInnerHTML={{ __html: zoomedSvg }}
            style={{ display: "flex", justifyContent: "center" }}
          />
        )}
      </Modal>
    </Stack>
  );
}

