## Why

The viewer opens with the Plain/Markdown/HTML switch and no document identity.
The queue identifies an entry by a 60-character slice of `original_text`, the
full title is never shown, and there is nowhere to record what the entry is
about or when it was added. The header adds that identity: an editable title,
an editable note, and the metadata of the current recording (date, status,
engine, voice), while the queue switches to the title as the primary identity
and search understands titles and notes.

## What Changes

- `TextEntry` gains `title: Option<String>` and `note: Option<String>`, both
  `#[serde(default)]`, so existing `history.json` files load unchanged (no
  migration). A new entry's `title` is seeded in the shared storage path —
  `StorageService::add_entry_with_source`, which the UI commands and the tray
  both go through — from the first usable line of `original_text`: Markdown
  heading markers and list bullets are stripped, whitespace is collapsed,
  the result is trimmed and capped at 200 characters; nothing usable stores
  `None`. The frontend mirrors the derivation (`src/lib/entryTitle.ts`) for
  legacy entries with `title: null`, locked by a shared case table on both
  sides.
- Two commands, `set_entry_title(id, title: Option<String>)` and
  `set_entry_note(id, note: Option<String>)` (two commands instead of an
  ambiguous `Option<Option<String>>` patch): trim, empty → `None`, title
  capped at 200 and note at 2000 characters, persisted atomically through
  `StorageService::update_entry_if` (the #363 pattern, so a concurrent
  synthesis completion is never rolled back), then `entry_updated` is emitted.
  Registered in `lib.rs` and wrapped in `src/lib/tauri.ts`.
- `EntryHeader` + `EntryHeader.module.css` inside `TextViewer`, above the
  scroll area, so the identity stays visible while reading:
  - title line, click-to-edit with a pencil affordance on hover/focus;
    Enter/blur saves, Esc cancels, an empty value stores `None` and falls back
    to the derived title;
  - note line below it, dimmed, clamped to two lines with an expand toggle,
    «Добавить заметку» when empty, editing in an autosize textarea with
    Ctrl+Enter/Save and Esc/Cancel;
  - meta row: date added (local time from the naive UTC `created_at`), status
    badge, engine and voice from `generation` (no snapshot → «не озвучено» /
    «—»); the format `SegmentedControl` moves to the right of the meta row,
    with the reading width/scale controls (#354/#357) next to it so they stay
    available in every display mode;
  - optimistic edits with rollback and a red toast on IPC failure; an incoming
    `entry_updated` (e.g. a status change) never discards an open draft.
- Queue items show `title` (or the shared derived fallback) instead of the
  60-character `original_text` slice, one line with an ellipsis and the full
  title in the native tooltip. Queue search matches title, note and
  `original_text`, case-insensitively.
- `displayEngine`/`displayVoice` and the date formatter move out of
  `GenerationParamsDialog` into shared modules (`src/lib/generationDisplay.ts`,
  `src/lib/datetime.ts`); the voice label becomes engine-specific (Piper voice
  names, Silero speaker names, TeraTTS ids, stored id as the fallback). The
  status badge mapping moves to `src/lib/entryStatus.ts`, shared by the queue
  and the header.
- RU/EN catalog entries for the header, note affordances and accessible names.
- Tests: storage round-trip and seeding, command trim/limits/persist/rollback,
  title derivation parity, header edit flows and draft survival, queue title
  and search, legacy fallback, and that a note never enters the TTS input.
- OpenSpec deltas for `storage`, `ipc-commands`, `queue-lifecycle`,
  `text-display` and `ui`, CHANGELOG `[Unreleased]`.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `storage`: "History File Schema" gains `title` and `note` with their
  defaulting rules; "Entry CRUD" gains title seeding on add; "Atomic
  Conditional Update" is the mechanism for metadata edits.
- `ipc-commands`: a new "Entry Metadata Commands" requirement covers
  `set_entry_title` / `set_entry_note`, their normalization, limits, atomic
  persistence and `entry_updated` emission.
- `queue-lifecycle`: "Queue list rendering" shows the title; "Queue search"
  matches title, note and `original_text`.
- `text-display`: a new "Viewer entry header" requirement covers the title,
  the note, the metadata row, the relocation of the format switch and the
  optimistic-edit rules.
- `ui`: "Text viewer reading controls" move into the header meta row without
  losing availability; "Recording parameters dialog" uses the shared
  engine-specific voice labels; "UI localization layer" covers the new
  RU/EN catalog entries. There is no separate `localization` capability in
  `openspec/specs/`; localization is a requirement of `ui`.

## Impact

- Affected code: `src-tauri/src/storage/{schema,service,title}.rs`,
  `src-tauri/src/storage/mod.rs`, `src-tauri/src/commands/mod.rs`,
  `src-tauri/src/commands/orchestration_tests.rs`, `src-tauri/src/lib.rs`,
  `src/lib/{tauri,entryTitle,generationDisplay,datetime,entryStatus,sileroVoices}.ts`,
  `src/components/{EntryHeader,TextViewer,QueueList}.tsx` and their CSS
  modules, `src/dialogs/{GenerationParamsDialog,Settings}.tsx`,
  `src/i18n/{ru,en}.ts`, tests, `openspec/`, `CHANGELOG.md`.
- `history.json` gains two optional fields; histories written by earlier
  builds parse unchanged and render through the derived fallback. No
  migration.
- User-visible: an editable title and note in the viewer, entry metadata in
  the header, titles in the queue, and search over title/note/text.
- Does not change synthesis input, playback, the display-mode semantics or
  per-entry actions (they stay in the context menu).

## Non-goals

- Action buttons in the header (play, regenerate, export stay in the context
  menu).
- Editing `original_text` or the synthesized text; rich-text/Markdown notes
  (plain text with newlines only).
- Tags, folders, sorting by title, title from an HTML `<title>`, per-entry
  voice selection (the voice is read-only here).
- Reorganizing the Settings dialog or changing reading-typography behavior.
