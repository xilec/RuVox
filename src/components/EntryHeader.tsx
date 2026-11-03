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
  type ReactNode,
} from 'react';
import type { TextEntry } from '../lib/tauri';
import { commands } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { useT } from '../lib/i18n';
import { entryDisplayTitle, TITLE_MAX_CHARS } from '../lib/entryTitle';
import { ABSENT, displayEngine, displayVoice } from '../lib/generationDisplay';
import { formatNaiveUtcDateTime } from '../lib/datetime';
import { STATUS_KEY, statusBadgeColor } from '../lib/entryStatus';
import { useLocaleStore } from '../stores/locale';
import { IconPencil } from './icons';
import classes from './EntryHeader.module.css';

/** Maximum note length; mirrors the backend command limit. */
const NOTE_MAX_CHARS = 2000;

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
 * produced the current recording. Edits are optimistic; a rejected IPC call
 * rolls the value back and shows the red error toast. Opening a draft is
 * never disturbed by incoming `entry_updated` events (e.g. a status change),
 * because drafts live in local state and are seeded only when editing starts.
 */
export function EntryHeader({ entry, controls }: EntryHeaderProps) {
  const tt = useT();
  const locale = useLocaleStore((s) => s.locale);

  // Optimistic overrides: `undefined` means "render the persisted value".
  // They are cleared once the matching `entry_updated` lands.
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

  const storedTitle = titleOverride !== undefined ? titleOverride : entry.title;
  const storedNote = noteOverride !== undefined ? noteOverride : entry.note;
  const displayTitle = entryDisplayTitle({
    title: storedTitle,
    original_text: entry.original_text,
  });

  // Drop the optimistic override once the persisted value catches up.
  useEffect(() => {
    setTitleOverride((current) =>
      current !== undefined && entry.title === current ? undefined : current,
    );
  }, [entry.title]);
  useEffect(() => {
    setNoteOverride((current) =>
      current !== undefined && entry.note === current ? undefined : current,
    );
  }, [entry.note]);
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

  function startTitleEdit() {
    setTitleDraft(storedTitle ?? '');
    skipTitleBlur.current = false;
    setTitleEditing(true);
  }

  function finishTitleEdit() {
    skipTitleBlur.current = true;
    setTitleEditing(false);
  }

  function commitTitle() {
    const next = titleDraft.trim().slice(0, TITLE_MAX_CHARS).trimEnd() || null;
    const previous = titleOverride;
    finishTitleEdit();
    if (next === (previous !== undefined ? previous : entry.title)) return;
    setTitleOverride(next);
    commands.setEntryTitle(entry.id, next).catch((err) => {
      setTitleOverride(previous);
      notifications.show({
        title: tt('errors.title'),
        message: formatError(err),
        color: 'red',
      });
    });
  }

  function handleTitleBlur() {
    if (skipTitleBlur.current) {
      skipTitleBlur.current = false;
      return;
    }
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
    const next = noteDraft.trim().slice(0, NOTE_MAX_CHARS).trimEnd() || null;
    const previous = noteOverride;
    finishNoteEdit();
    if (next === (previous !== undefined ? previous : entry.note)) return;
    setNoteOverride(next);
    commands.setEntryNote(entry.id, next).catch((err) => {
      setNoteOverride(previous);
      notifications.show({
        title: tt('errors.title'),
        message: formatError(err),
        color: 'red',
      });
    });
  }

  function handleNoteBlur() {
    if (skipNoteBlur.current) {
      skipNoteBlur.current = false;
      return;
    }
    commitNote();
  }

  const generation = entry.generation;
  const processing = entry.status === 'processing';

  return (
    <div className={classes.header}>
      <Stack gap={4}>
        <div className={classes.titleRow}>
          {titleEditing ? (
            <>
              <TextInput
                className={classes.titleInput}
                value={titleDraft}
                onChange={(e) => setTitleDraft(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    commitTitle();
                  } else if (e.key === 'Escape') {
                    e.preventDefault();
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
          <Stack gap={4} className={classes.noteEdit}>
            <Textarea
              value={noteDraft}
              onChange={(e) => setNoteDraft(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === 'Escape') {
                  e.preventDefault();
                  finishNoteEdit();
                } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
                  e.preventDefault();
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
