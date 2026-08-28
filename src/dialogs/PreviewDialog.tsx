import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  ActionIcon,
  Anchor,
  Button,
  Checkbox,
  Group,
  Loader,
  Popover,
  Portal,
  Select,
  Switch,
  Text,
  Textarea,
} from '@mantine/core';
import { notifications } from '@mantine/notifications';
import { openUrl } from '@tauri-apps/plugin-opener';
import { Rnd } from 'react-rnd';
import classes from './PreviewDialog.module.css';
import { commands } from '../lib/tauri';
import type { EntryFormat } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { useT } from '../lib/i18n';
import { useLocaleStore } from '../stores/locale';
import { previewTextFor } from '../lib/html';
import { detectFormat } from '../lib/detectFormat';

export interface PreviewDialogProps {
  /** Raw clipboard text to preview and optionally edit before synthesis. */
  text: string;
  opened: boolean;
  /**
   * Format preselected in the selector instead of the auto mode. Imports set
   * it (the routed format — text-import spec); clipboard openings omit it so
   * the dialog starts in the auto-detect mode.
   */
  initialFormat?: EntryFormat;
  /**
   * Called when the user confirms synthesis.
   * `finalText` is either the original or the user-edited version.
   * `skipShortTexts` is true when the user checked the "skip for short texts" box.
   * `playWhenReady` reflects the dialog's "Read Now" toggle state.
   * `sourceFormat` is the effective source-format selection: the explicitly
   * chosen value, or the detected one while the selector is in auto mode.
   */
  onSynthesize: (
    finalText: string,
    skipShortTexts: boolean,
    playWhenReady: boolean,
    sourceFormat: EntryFormat,
  ) => void;
  /** Called when the user cancels the dialog. */
  onCancel: () => void;
}

const INITIAL_W = 900;
const INITIAL_H = 620;
const MIN_W = 560;
const MIN_H = 380;
const DEBOUNCE_MS = 1000;

/** Deep link targets for the help affordance — the README normalization
 *  section in the language matching the UI (see the README language policy:
 *  README.md is Russian, README.en.md is its English mirror). */
const README_HELP_URLS = {
  ru: 'https://github.com/xilec/RuVox#Нормализация',
  en: 'https://github.com/xilec/RuVox#normalization',
} as const;

function centeredPosition(w: number, h: number) {
  if (typeof window === 'undefined') return { x: 40, y: 40 };
  return {
    x: Math.max(20, Math.floor((window.innerWidth - w) / 2)),
    y: Math.max(20, Math.floor((window.innerHeight - h) / 2)),
  };
}

function IconClose() {
  return (
    <svg width={16} height={16} viewBox="0 0 24 24" fill="currentColor" aria-hidden>
      <path d="M6.4 5 5 6.4 10.6 12 5 17.6 6.4 19 12 13.4 17.6 19 19 17.6 13.4 12 19 6.4 17.6 5 12 10.6z" />
    </svg>
  );
}

function IconHelp() {
  return (
    <svg width={16} height={16} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} aria-hidden>
      <circle cx="12" cy="12" r="9" />
      <path d="M9.3 9.5a2.7 2.7 0 1 1 3.9 2.4c-.8.4-1.2 1-1.2 1.9v.4" strokeLinecap="round" />
      <circle cx="12" cy="17.2" r="0.6" fill="currentColor" stroke="none" />
    </svg>
  );
}

/**
 * Preview dialog (FF 1.1).
 *
 * Non-modal floating window (react-rnd) rendered via a Mantine Portal so it
 * sits above the app but doesn't block the UI underneath.  The user can:
 *   - drag the window by its header,
 *   - resize from any edge / corner,
 *   - edit the original text (right pane live-renormalizes with a 1 s debounce),
 *   - toggle synchronised scrolling between the two panes.
 * ESC closes (behaves like Cancel).
 */
export function PreviewDialog({
  text,
  opened,
  initialFormat,
  onSynthesize,
  onCancel,
}: PreviewDialogProps) {
  const tt = useT();
  const [editedText, setEditedText] = useState<string>(text);
  // 'auto' is the selector state, not an entry format: the effective format
  // is detected from the content and re-detected while editing (preview-dialog
  // spec, "Source format selection").
  const [sourceFormat, setSourceFormat] = useState<EntryFormat | 'auto'>(
    initialFormat ?? 'auto',
  );
  const [skipShortTexts, setSkipShortTexts] = useState(false);
  const [playWhenReady, setPlayWhenReady] = useState(true);
  const [normalized, setNormalized] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [syncScroll, setSyncScroll] = useState(false);
  const [helpOpened, setHelpOpened] = useState(false);
  const locale = useLocaleStore((s) => s.locale);
  const [position, setPosition] = useState<{ x: number; y: number }>(() =>
    centeredPosition(INITIAL_W, INITIAL_H),
  );
  const [size, setSize] = useState<{ width: number; height: number }>({
    width: INITIAL_W,
    height: INITIAL_H,
  });

  const leftPaneRef = useRef<HTMLPreElement>(null);
  const rightPaneRef = useRef<HTMLPreElement>(null);
  // Guard against sync-scroll ping-pong: setting one pane's scrollTop fires
  // a scroll event on it, which would otherwise re-sync back to the source.
  const syncingRef = useRef(false);

  // Reset per-dialog-open state (including floating-window geometry: centre
  // each time the dialog is opened so it never appears off-screen after a
  // window resize between invocations).
  useEffect(() => {
    if (!opened) return;
    setEditMode(false);
    setSkipShortTexts(false);
    setPlayWhenReady(true);
    setEditedText(text);
    setSourceFormat(initialFormat ?? 'auto');
    setHelpOpened(false);
    setSize({ width: INITIAL_W, height: INITIAL_H });
    setPosition(centeredPosition(INITIAL_W, INITIAL_H));
  }, [opened, text, initialFormat]);

  // The text synthesis will actually use: the edited version, falling back
  // to the original when the edit is empty (preview-dialog spec, "Synthesis
  // confirmation"). Detection runs on that same text so the auto label and
  // the ingest decision always match what will be sent.
  const synthesisText = (editMode ? editedText.trim() : text) || text;
  // Memoized: react-rnd emits position/size updates on every drag/resize
  // frame, and detection runs several regex passes over the full text.
  const effectiveFormat: EntryFormat = useMemo(
    () => (sourceFormat === 'auto' ? detectFormat(synthesisText) : sourceFormat),
    [sourceFormat, synthesisText],
  );
  const formatLabels: Record<EntryFormat, string> = {
    plain: tt('preview.source_format.plain'),
    markdown: tt('preview.source_format.markdown'),
    html: tt('preview.source_format.html'),
  };

  // Debounced (re-)normalization.  Runs whenever the text under consideration
  // changes — the initial clipboard text on open, or the edited text once
  // the user enters edit mode.  With the `html` source format the markup is
  // sanitized and extracted first, so the preview shows what will actually
  // be narrated (preview-dialog spec).
  useEffect(() => {
    if (!opened) return;
    const source = previewTextFor(editedText, effectiveFormat);
    if (!source.trim()) {
      setNormalized('');
      setLoading(false);
      return;
    }
    setLoading(true);
    // Guard against out-of-order responses: a slow preview_normalize for a
    // previous input/format must not overwrite a newer result.
    let stale = false;
    const timer = window.setTimeout(() => {
      commands
        .previewNormalize(source)
        .then((result) => {
          if (!stale) setNormalized(result.normalized);
        })
        .catch((err) => {
          if (!stale)
            setNormalized(tt('preview.normalize_error', [formatError(err)]));
        })
        .finally(() => {
          if (!stale) setLoading(false);
        });
    }, DEBOUNCE_MS);
    return () => {
      stale = true;
      window.clearTimeout(timer);
    };
  }, [opened, editedText, effectiveFormat, tt]);

  // ESC closes the floating window (mantine Modal used to handle this; non-modal
  // react-rnd has no built-in handler, so we bind one manually while opened).
  useEffect(() => {
    if (!opened) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onCancel();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [opened, onCancel]);

  function handleSynthesize() {
    onSynthesize(synthesisText, skipShortTexts, playWhenReady, effectiveFormat);
  }

  function handleEdit() {
    setEditMode(true);
  }

  const openHelpReadme = useCallback(async () => {
    setHelpOpened(false);
    try {
      await openUrl(README_HELP_URLS[locale]);
    } catch (err) {
      notifications.show({
        title: tt('errors.title'),
        message: formatError(err),
        color: 'red',
      });
    }
  }, [locale, tt]);

  const handlePaneScroll = useCallback(
    (side: 'left' | 'right') => {
      if (!syncScroll || syncingRef.current) return;
      const src = (side === 'left' ? leftPaneRef : rightPaneRef).current;
      const dst = (side === 'left' ? rightPaneRef : leftPaneRef).current;
      if (!src || !dst) return;
      const srcRange = src.scrollHeight - src.clientHeight;
      const dstRange = dst.scrollHeight - dst.clientHeight;
      if (srcRange <= 0 || dstRange <= 0) return;
      const target = (src.scrollTop / srcRange) * dstRange;
      if (Math.abs(dst.scrollTop - target) < 1) return;
      syncingRef.current = true;
      dst.scrollTop = target;
      requestAnimationFrame(() => {
        syncingRef.current = false;
      });
    },
    [syncScroll],
  );

  if (!opened) return null;

  return (
    <Portal>
      {/* Viewport-sized, transparent, click-through container.  Rnd's
          internal `position: absolute` is measured from its nearest
          positioned ancestor — if that's <body> with document-level scroll,
          the dialog drifts off-screen.  Anchoring Rnd inside a fixed,
          inset-0 container makes its (x, y) equal to viewport coordinates
          regardless of body overflow. */}
      <div className={classes.viewportContainer}>
        <Rnd
          position={position}
          size={size}
          onDragStop={(_, d) => setPosition({ x: d.x, y: d.y })}
          onResizeStop={(_, __, ref, ___, newPos) => {
            setSize({
              width: ref.offsetWidth,
              height: ref.offsetHeight,
            });
            setPosition(newPos);
          }}
          minWidth={MIN_W}
          minHeight={MIN_H}
          bounds="parent"
          dragHandleClassName={classes.dragHandle}
          enableResizing
          className={classes.rnd}
        >
        <div className={classes.panel}>
          <header className={`${classes.header} ${classes.dragHandle}`}>
            <Text className={classes.title}>{tt('preview.title')}</Text>
            <Group gap="xs" wrap="nowrap">
              <Popover
                opened={helpOpened}
                onChange={setHelpOpened}
                position="bottom-end"
                width={420}
                withArrow
                shadow="md"
                transitionProps={{ duration: 0 }}
              >
                <Popover.Target>
                  <ActionIcon
                    variant="subtle"
                    size="sm"
                    onClick={() => setHelpOpened((o) => !o)}
                    aria-label={tt('preview.explain.help_aria')}
                  >
                    <IconHelp />
                  </ActionIcon>
                </Popover.Target>
                <Popover.Dropdown>
                  <Text size="sm" className={classes.helpText}>
                    {tt('preview.explain.details')}
                  </Text>
                  <Anchor
                    size="sm"
                    href={README_HELP_URLS[locale]}
                    target="_blank"
                    rel="noreferrer"
                    onClick={(e) => {
                      e.preventDefault();
                      void openHelpReadme();
                    }}
                  >
                    {tt('preview.explain.readme_link')}
                  </Anchor>
                </Popover.Dropdown>
              </Popover>
              <ActionIcon
                variant="subtle"
                size="sm"
                onClick={onCancel}
                aria-label={tt('preview.close')}
              >
                <IconClose />
              </ActionIcon>
            </Group>
          </header>

          <div className={classes.body}>
            <Text size="sm" className={classes.explainer}>
              {tt('preview.explain.line')}
            </Text>
            <div className={classes.panes}>
              <div className={classes.paneCol}>
                <Text className={classes.paneLabel}>{tt('preview.original')}</Text>
                {editMode ? (
                  <Textarea
                    classNames={{
                      root: classes.editTextareaRoot,
                      wrapper: classes.editTextareaWrapper,
                      input: classes.editTextareaInput,
                    }}
                    value={editedText}
                    onChange={(e) => setEditedText(e.currentTarget.value)}
                  />
                ) : (
                  <pre
                    ref={leftPaneRef}
                    className={classes.textPane}
                    onScroll={() => handlePaneScroll('left')}
                  >
                    {text}
                  </pre>
                )}
              </div>

              <div className={classes.paneCol}>
                <Text className={classes.paneLabel}>{tt('preview.normalized')}</Text>
                {loading ? (
                  <div className={classes.loaderPane}>
                    <Loader size="sm" />
                  </div>
                ) : (
                  <pre
                    ref={rightPaneRef}
                    className={classes.textPane}
                    onScroll={() => handlePaneScroll('right')}
                  >
                    {normalized}
                  </pre>
                )}
              </div>
            </div>

            <Group
              className={classes.footer}
              justify="space-between"
              gap="sm"
              wrap="wrap"
            >
              <Group gap="md" wrap="wrap">
                <Select
                  size="xs"
                  aria-label={tt('preview.source_format.aria')}
                  data={[
                    {
                      value: 'auto',
                      label: tt('preview.source_format.auto_detected', [
                        formatLabels[effectiveFormat],
                      ]),
                    },
                    { value: 'plain', label: formatLabels.plain },
                    { value: 'markdown', label: formatLabels.markdown },
                    { value: 'html', label: formatLabels.html },
                  ]}
                  value={sourceFormat}
                  onChange={(v) => {
                    if (v === 'auto' || v === 'plain' || v === 'markdown' || v === 'html') {
                      setSourceFormat(v);
                    }
                  }}
                  allowDeselect={false}
                />
                <Checkbox
                  label={tt('preview.dont_show_again')}
                  checked={skipShortTexts}
                  onChange={(e) =>
                    setSkipShortTexts(e.currentTarget.checked)
                  }
                />
                <Checkbox
                  label={tt('preview.sync_scroll')}
                  checked={syncScroll}
                  onChange={(e) => setSyncScroll(e.currentTarget.checked)}
                />
              </Group>

              <Group gap="sm" align="center">
                <Switch
                  label={tt('preview.read_now')}
                  checked={playWhenReady}
                  onChange={(e) => setPlayWhenReady(e.currentTarget.checked)}
                />
                <Button variant="default" onClick={onCancel}>
                  {tt('common.cancel')}
                </Button>
                {!editMode && (
                  <Button variant="outline" onClick={handleEdit}>
                    {tt('preview.edit')}
                  </Button>
                )}
                <Button onClick={handleSynthesize} disabled={loading}>
                  {tt('preview.synthesize')}
                </Button>
              </Group>
            </Group>
          </div>
        </div>
        </Rnd>
      </div>
    </Portal>
  );
}
