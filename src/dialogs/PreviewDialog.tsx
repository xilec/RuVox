import { useCallback, useEffect, useRef, useState } from 'react';
import { ActionIcon, Button, Checkbox, Group, Loader, Portal, Switch, Text, Textarea } from '@mantine/core';
import { Rnd } from 'react-rnd';
import classes from './PreviewDialog.module.css';
import { commands } from '../lib/tauri';
import { formatError } from '../lib/errors';

export interface PreviewDialogProps {
  /** Raw clipboard text to preview and optionally edit before synthesis. */
  text: string;
  opened: boolean;
  /**
   * Called when the user confirms synthesis.
   * `finalText` is either the original or the user-edited version.
   * `skipShortTexts` is true when the user checked the "skip for short texts" box.
   * `playWhenReady` reflects the dialog's "Read Now" toggle state.
   */
  onSynthesize: (
    finalText: string,
    skipShortTexts: boolean,
    playWhenReady: boolean,
  ) => void;
  /** Called when the user cancels the dialog. */
  onCancel: () => void;
}

const INITIAL_W = 900;
const INITIAL_H = 620;
const MIN_W = 560;
const MIN_H = 380;
const DEBOUNCE_MS = 1000;

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
  onSynthesize,
  onCancel,
}: PreviewDialogProps) {
  const [editedText, setEditedText] = useState<string>(text);
  const [skipShortTexts, setSkipShortTexts] = useState(false);
  const [playWhenReady, setPlayWhenReady] = useState(true);
  const [normalized, setNormalized] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [syncScroll, setSyncScroll] = useState(false);
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
    setSize({ width: INITIAL_W, height: INITIAL_H });
    setPosition(centeredPosition(INITIAL_W, INITIAL_H));
  }, [opened, text]);

  // Debounced (re-)normalization.  Runs whenever the text under consideration
  // changes — the initial clipboard text on open, or the edited text once
  // the user enters edit mode.
  useEffect(() => {
    if (!opened) return;
    if (!editedText.trim()) {
      setNormalized('');
      setLoading(false);
      return;
    }
    setLoading(true);
    const timer = window.setTimeout(() => {
      commands
        .previewNormalize(editedText)
        .then((result) => setNormalized(result.normalized))
        .catch((err) =>
          setNormalized(`(ошибка нормализации: ${formatError(err)})`),
        )
        .finally(() => setLoading(false));
    }, DEBOUNCE_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [opened, editedText]);

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
    const finalText = editMode ? editedText.trim() : text;
    onSynthesize(finalText || text, skipShortTexts, playWhenReady);
  }

  function handleEdit() {
    setEditMode(true);
  }

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
            <Text className={classes.title}>Предпросмотр нормализации</Text>
            <ActionIcon
              variant="subtle"
              size="sm"
              onClick={onCancel}
              aria-label="Закрыть"
            >
              <IconClose />
            </ActionIcon>
          </header>

          <div className={classes.body}>
            <div className={classes.panes}>
              <div className={classes.paneCol}>
                <Text className={classes.paneLabel}>Оригинал</Text>
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
                <Text className={classes.paneLabel}>После нормализации</Text>
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
                <Checkbox
                  label="Больше не показывать этот диалог"
                  checked={skipShortTexts}
                  onChange={(e) =>
                    setSkipShortTexts(e.currentTarget.checked)
                  }
                />
                <Checkbox
                  label="Синхронный скроллинг"
                  checked={syncScroll}
                  onChange={(e) => setSyncScroll(e.currentTarget.checked)}
                />
              </Group>

              <Group gap="sm" align="center">
                <Switch
                  label="Read Now"
                  checked={playWhenReady}
                  onChange={(e) => setPlayWhenReady(e.currentTarget.checked)}
                />
                <Button variant="default" onClick={onCancel}>
                  Отмена
                </Button>
                {!editMode && (
                  <Button variant="outline" onClick={handleEdit}>
                    Редактировать
                  </Button>
                )}
                <Button onClick={handleSynthesize} disabled={loading}>
                  Синтезировать
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
