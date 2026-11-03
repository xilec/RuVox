import {
  ActionIcon,
  Badge,
  Button,
  Group,
  Loader,
  Stack,
  Text,
  Textarea,
  TextInput,
} from '@mantine/core';
import { notifications } from '@mantine/notifications';
import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type FocusEvent,
  type ReactNode,
} from 'react';
import type { TextEntry } from '../lib/tauri';
import { commands } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { useT } from '../lib/i18n';
import {
  capCodePoints,
  deriveTitle,
  entryDisplayTitle,
  TITLE_MAX_CHARS,
  trimWhiteSpace,
} from '../lib/entryTitle';
import { ABSENT, displayEngine, displayVoice } from '../lib/generationDisplay';
import { formatNaiveUtcDateTime } from '../lib/datetime';
import { STATUS_KEY, statusBadgeColor } from '../lib/entryStatus';
import { useLocaleStore } from '../stores/locale';
import { IconPencil } from './icons';
import classes from './EntryHeader.module.css';

/** Maximum note length; mirrors the backend command limit. */
const NOTE_MAX_CHARS = 2000;

/**
 * Normalize a title draft the way `set_entry_title` does: trim Unicode
 * whitespace, cap at the code-point limit, and store `null` when the value is
 * empty or equals the derived fallback — "cleared" and "back to derived" are
 * the same stored state.
 */
function normalizeTitleDraft(draft: string, entry: TextEntry): string | null {
  const next = capCodePoints(trimWhiteSpace(draft), TITLE_MAX_CHARS);
  if (next === '') return null;
  return next === deriveTitle(entry.original_text) ? null : next;
}

/** Normalize a note draft: trim the edges and cap at the backend limit. */
function normalizeNoteDraft(draft: string): string | null {
  return capCodePoints(trimWhiteSpace(draft), NOTE_MAX_CHARS) || null;
}

interface EntryHeaderProps {
  entry: TextEntry;
  /** Viewer-level controls (format switch, reading width/scale) rendered at
   *  the right end of the metadata row: #353 moved the format switch here and
   *  #354/#357 keep their controls next to it. */
  controls?: ReactNode;
}

/**
 * Viewer header above the reading area (issue #353): editable title and note,
 * plus the entry's metadata — date added, status, and the engine/voice that
 * produced the current recording.
 *
 * Edits are optimistic: the value renders immediately, and a rejection rolls
 * it back to the persisted one with the red error toast. Each field tags its
 * writes with a monotonic request id, so a superseded write that settles last
 * can neither roll back a newer edit nor release its override (the same guard
 * as the reading controls, #364). Opening a draft is never disturbed by
 * incoming `entry_updated` events (e.g. a status change), because drafts live
 * in local state and are seeded only when editing starts.
 */
export function EntryHeader({ entry, controls }: EntryHeaderProps) {
  const tt = useT();
  const locale = useLocaleStore((s) => s.locale);

  // Optimistic overrides: `undefined` means "render the persisted value".
  // They are released once the matching `entry_updated` lands — see the
  // guarded effects below.
  const [titleOverride, setTitleOverride] = useState<string | null | undefined>(
    undefined,
  );
  const [noteOverride, setNoteOverride] = useState<string | null | undefined>(
    undefined,
  );
  const [titleEditing, setTitleEditing] = useState(false);
  const [titleDraft, setTitleDraft] = useState('');
  const [noteEditing, setNoteEditing] = useState(false);
  const [noteDraft, setNoteDraft] = useState('');
  const [noteExpanded, setNoteExpanded] = useState(false);
  const [noteOverflowing, setNoteOverflowing] = useState(false);
  const noteRef = useRef<HTMLDivElement>(null);
  // Set when editing ends programmatically (Enter/Esc/Save), so the blur that
  // follows the unmount does not commit a second time.
  const skipTitleBlur = useRef(false);
  const skipNoteBlur = useRef(false);
  // Per-field write bookkeeping: `requestId` names the newest edit,
  // `settledId` the edit that settled last, `pending` counts the unsettled
  // writes. A superseded write may settle after a newer one, so it must
  // neither roll the newer override back nor release it when its stale echo
  // lands out of order.
  const titleRequestId = useRef(0);
  const titleSettledId = useRef(0);
  const [titlePending, setTitlePending] = useState(0);
  const noteRequestId = useRef(0);
  const noteSettledId = useRef(0);
  const [notePending, setNotePending] = useState(0);
  // The edit rows, so Tab between a field and its Save/Cancel buttons does not
  // look like leaving the editor (blur commits otherwise).
  const titleRowRef = useRef<HTMLDivElement>(null);
  const noteEditRef = useRef<HTMLDivElement>(null);
  // Keyboard exits hand focus back to the control that opened the editor.
  const titleButtonRef = useRef<HTMLButtonElement>(null);
  const noteEditButtonRef = useRef<HTMLButtonElement>(null);
  const restoreTitleFocus = useRef(false);
  const restoreNoteFocus = useRef(false);

  const storedTitle = titleOverride !== undefined ? titleOverride : entry.title;
  const storedNote = noteOverride !== undefined ? noteOverride : entry.note;
  const displayTitle = entryDisplayTitle({
    title: storedTitle,
    original_text: entry.original_text,
  });

  // Release an optimistic value only when the entry has caught up with it and
  // the newest edit is the one that settled. A superseded write can settle
  // last and echo its older value; clearing the override while that write is
  // still out would flash it back over the edit the user actually made.
  useEffect(() => {
    if (titlePending > 0 || titleSettledId.current !== titleRequestId.current) {
      return;
    }
    setTitleOverride((current) =>
      current !== undefined && entry.title === current ? undefined : current,
    );
  }, [entry.title, titlePending]);
  useEffect(() => {
    if (notePending > 0 || noteSettledId.current !== noteRequestId.current) {
      return;
    }
    setNoteOverride((current) =>
      current !== undefined && entry.note === current ? undefined : current,
    );
  }, [entry.note, notePending]);
  // A collapsed note starts collapsed again when its text changes.
  useEffect(() => {
    setNoteExpanded(false);
  }, [entry.note]);

  // Measure whether the two-line clamp hides text; an expanded element is not
  // clamped and cannot report overflow, so keep the last measured value.
  useLayoutEffect(() => {
    const el = noteRef.current;
    if (el === null || noteExpanded) return;
    const measure = () => {
      setNoteOverflowing(el.scrollHeight > el.clientHeight + 1);
    };
    measure();
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [storedNote, noteExpanded]);

  // Latest render values for the unmount cleanup: a `[]` effect's cleanup
  // closure cannot read state. `TextViewer` keys this header by entry id, so
  // switching entries unmounts it without a blur — commit an open draft on the
  // way out (fire-and-forget, errors ignored) instead of losing the edit. Esc
  // already closed the editor, and a draft equal to the stored value needs no
  // write.
  const latest = useRef({
    entry,
    titleEditing,
    titleDraft,
    noteEditing,
    noteDraft,
  });
  useEffect(() => {
    latest.current = { entry, titleEditing, titleDraft, noteEditing, noteDraft };
  }, [entry, titleEditing, titleDraft, noteEditing, noteDraft]);
  useEffect(
    () => () => {
      const snapshot = latest.current;
      if (snapshot.titleEditing) {
        const next = normalizeTitleDraft(snapshot.titleDraft, snapshot.entry);
        if (next !== snapshot.entry.title) {
          void commands.setEntryTitle(snapshot.entry.id, next).catch(() => {});
        }
      }
      if (snapshot.noteEditing) {
        const next = normalizeNoteDraft(snapshot.noteDraft);
        if (next !== snapshot.entry.note) {
          void commands.setEntryNote(snapshot.entry.id, next).catch(() => {});
        }
      }
    },
    [],
  );

  // Enter/Esc end the edit without a pointer move; hand focus back to the
  // control that opened the editor instead of dropping it on the document.
  useEffect(() => {
    if (titleEditing || !restoreTitleFocus.current) return;
    restoreTitleFocus.current = false;
    titleButtonRef.current?.focus();
  }, [titleEditing]);
  useEffect(() => {
    if (noteEditing || !restoreNoteFocus.current) return;
    restoreNoteFocus.current = false;
    noteEditButtonRef.current?.focus();
  }, [noteEditing]);

  function startTitleEdit() {
    setTitleDraft(storedTitle ?? entryDisplayTitle(entry));
    skipTitleBlur.current = false;
    setTitleEditing(true);
  }

  function finishTitleEdit() {
    skipTitleBlur.current = true;
    setTitleEditing(false);
  }

  function commitTitle() {
    const next = normalizeTitleDraft(titleDraft, entry);
    finishTitleEdit();
    if (next === (titleOverride !== undefined ? titleOverride : entry.title)) {
      return;
    }
    const requestId = (titleRequestId.current += 1);
    setTitleOverride(next);
    setTitlePending((pending) => pending + 1);
    void commands.setEntryTitle(entry.id, next).then(
      () => {
        titleSettledId.current = requestId;
        setTitlePending((pending) => pending - 1);
      },
      (err) => {
        titleSettledId.current = requestId;
        setTitlePending((pending) => pending - 1);
        // A superseded edit keeps its hands off the newer override.
        if (requestId !== titleRequestId.current) return;
        setTitleOverride(undefined);
        notifications.show({
          title: tt('errors.title'),
          message: formatError(err),
          color: 'red',
        });
      },
    );
  }

  function handleTitleBlur(event: FocusEvent<HTMLInputElement>) {
    if (skipTitleBlur.current) {
      skipTitleBlur.current = false;
      return;
    }
    const next = event.relatedTarget;
    if (next instanceof Node && titleRowRef.current?.contains(next)) return;
    commitTitle();
  }

  function startNoteEdit() {
    setNoteDraft(storedNote ?? '');
    skipNoteBlur.current = false;
    setNoteEditing(true);
  }

  function finishNoteEdit() {
    skipNoteBlur.current = true;
    setNoteEditing(false);
  }

  function commitNote() {
    const next = normalizeNoteDraft(noteDraft);
    finishNoteEdit();
    if (next === (noteOverride !== undefined ? noteOverride : entry.note)) return;
    const requestId = (noteRequestId.current += 1);
    setNoteOverride(next);
    setNotePending((pending) => pending + 1);
    void commands.setEntryNote(entry.id, next).then(
      () => {
        noteSettledId.current = requestId;
        setNotePending((pending) => pending - 1);
      },
      (err) => {
        noteSettledId.current = requestId;
        setNotePending((pending) => pending - 1);
        // A superseded edit keeps its hands off the newer override.
        if (requestId !== noteRequestId.current) return;
        setNoteOverride(undefined);
        notifications.show({
          title: tt('errors.title'),
          message: formatError(err),
          color: 'red',
        });
      },
    );
  }

  function handleNoteBlur(event: FocusEvent<HTMLTextAreaElement>) {
    if (skipNoteBlur.current) {
      skipNoteBlur.current = false;
      return;
    }
    const next = event.relatedTarget;
    if (next instanceof Node && noteEditRef.current?.contains(next)) return;
    commitNote();
  }

  const generation = entry.generation;
  const processing = entry.status === 'processing';

  return (
    <div className={classes.header} data-entry-header>
      <Stack gap={4}>
        <div className={classes.titleRow} ref={titleRowRef}>
          {titleEditing ? (
            <>
              <TextInput
                className={classes.titleInput}
                value={titleDraft}
                onChange={(e) => setTitleDraft(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    restoreTitleFocus.current = true;
                    commitTitle();
                  } else if (e.key === 'Escape') {
                    e.preventDefault();
                    restoreTitleFocus.current = true;
                    finishTitleEdit();
                  }
                }}
                onBlur={handleTitleBlur}
                placeholder={tt('header.title.placeholder')}
                aria-label={tt('header.title.edit_aria')}
                maxLength={TITLE_MAX_CHARS}
                size="sm"
                autoFocus
              />
              <Group gap={4} wrap="nowrap">
                <Text size="xs" c="dimmed" className={classes.editHint}>
                  {tt('header.title.kbd_hint')}
                </Text>
                <Button
                  size="compact-xs"
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={commitTitle}
                >
                  {tt('common.save')}
                </Button>
                <Button
                  size="compact-xs"
                  variant="default"
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={finishTitleEdit}
                >
                  {tt('common.cancel')}
                </Button>
              </Group>
            </>
          ) : (
            <>
              <button
                ref={titleButtonRef}
                type="button"
                className={classes.titleButton}
                onClick={startTitleEdit}
                title={displayTitle}
                aria-label={displayTitle}
              >
                {displayTitle}
              </button>
              <ActionIcon
                className={classes.pencil}
                variant="subtle"
                color="gray"
                size="sm"
                onClick={startTitleEdit}
                aria-label={tt('header.title.edit')}
                title={tt('header.title.edit')}
              >
                <IconPencil />
              </ActionIcon>
            </>
          )}
        </div>

        {noteEditing ? (
          <Stack gap={4} className={classes.noteEdit} ref={noteEditRef}>
            <Textarea
              value={noteDraft}
              onChange={(e) => setNoteDraft(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === 'Escape') {
                  e.preventDefault();
                  restoreNoteFocus.current = true;
                  finishNoteEdit();
                } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
                  e.preventDefault();
                  restoreNoteFocus.current = true;
                  commitNote();
                }
              }}
              onBlur={handleNoteBlur}
              placeholder={tt('header.note.placeholder')}
              aria-label={tt('header.note.edit_aria')}
              autosize
              minRows={2}
              maxRows={6}
              maxLength={NOTE_MAX_CHARS}
              size="sm"
              autoFocus
            />
            <Group gap={4} justify="flex-end" wrap="nowrap">
              <Text size="xs" c="dimmed" className={classes.noteHint}>
                {tt('header.note.kbd_hint')}
              </Text>
              <Button
                size="compact-xs"
                onMouseDown={(e) => e.preventDefault()}
                onClick={commitNote}
              >
                {tt('common.save')}
              </Button>
              <Button
                size="compact-xs"
                variant="default"
                onMouseDown={(e) => e.preventDefault()}
                onClick={finishNoteEdit}
              >
                {tt('common.cancel')}
              </Button>
            </Group>
          </Stack>
        ) : storedNote ? (
          <div className={classes.noteRow}>
            <Text
              ref={noteRef}
              size="sm"
              c="dimmed"
              className={noteExpanded ? classes.noteExpanded : classes.note}
              title={noteExpanded ? undefined : storedNote}
            >
              {storedNote}
            </Text>
            {noteOverflowing && (
              <Button
                variant="subtle"
                size="compact-xs"
                className={classes.noteToggle}
                onClick={() => setNoteExpanded((v) => !v)}
                aria-label={tt(
                  noteExpanded ? 'header.note.collapse' : 'header.note.expand',
                )}
                title={tt(
                  noteExpanded ? 'header.note.collapse' : 'header.note.expand',
                )}
              >
                {tt(
                  noteExpanded ? 'header.note.collapse' : 'header.note.expand',
                )}
              </Button>
            )}
            <ActionIcon
              ref={noteEditButtonRef}
              className={classes.pencil}
              variant="subtle"
              color="gray"
              size="sm"
              onClick={startNoteEdit}
              aria-label={tt('header.note.edit')}
              title={tt('header.note.edit')}
            >
              <IconPencil />
            </ActionIcon>
          </div>
        ) : (
          <Button
            ref={noteEditButtonRef}
            variant="subtle"
            size="compact-xs"
            color="gray"
            className={classes.addNote}
            onClick={startNoteEdit}
          >
            {tt('header.note.add')}
          </Button>
        )}

        <div className={classes.metaRow}>
          <Group className={classes.meta} gap={6} wrap="wrap" align="center">
            <Text size="xs" c="dimmed">
              {formatNaiveUtcDateTime(entry.created_at, locale)}
            </Text>
            <Text size="xs" c="dimmed" aria-hidden>
              ·
            </Text>
            <Badge
              size="xs"
              color={statusBadgeColor(entry.status)}
              leftSection={processing ? <Loader size={8} color="blue" /> : null}
            >
              {tt(STATUS_KEY[entry.status])}
            </Badge>
            <Text size="xs" c="dimmed" aria-hidden>
              ·
            </Text>
            <Text size="xs" c="dimmed">
              {generation
                ? displayEngine(generation.engine, tt)
                : tt('header.not_narrated')}
            </Text>
            <Text size="xs" c="dimmed" aria-hidden>
              ·
            </Text>
            <Text size="xs" c="dimmed">
              {generation
                ? displayVoice(generation.engine, generation.voice, tt)
                : ABSENT}
            </Text>
          </Group>
          {controls && <div className={classes.controls}>{controls}</div>}
        </div>
      </Stack>
    </div>
  );
}
