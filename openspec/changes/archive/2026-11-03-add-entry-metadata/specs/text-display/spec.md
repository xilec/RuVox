## ADDED Requirements

### Requirement: Viewer entry header

The viewer SHALL render an entry header above the scrollable reading area, so
the entry's identity and metadata stay visible while the text scrolls. The
header SHALL contain, from top to bottom:

- the title line;
- the note line;
- a metadata row with the date added, the status badge, the engine and the
  voice of the current recording, and — at its right end — the format display
  switch when the entry's source format offers rendered views.

The title shown SHALL be the stored `title`, or — when it is `null` — the title
derived from `original_text` by the same algorithm the backend uses for
seeding, so legacy entries look identical to freshly added ones. The title line
SHALL be click-to-edit: clicking it or its pencil affordance (visible on hover
or keyboard focus) opens a single-line input seeded with the stored title and
the derived title as placeholder. `Enter` and blur SHALL save, `Esc` SHALL
cancel without persisting, and an empty or whitespace-only value SHALL store
`null` and render the derived fallback again instead of an empty line. Saving
SHALL be optimistic: the new value renders immediately and a rejected IPC call
reverts it and shows the localized red error toast.

The note SHALL render dimmed under the title, clamped to two lines with an
expand/collapse toggle when the text overflows the clamp. An entry without a
note SHALL show the «Добавить заметку» affordance (EN: "Add a note"). Editing
SHALL use an autosize textarea with `Ctrl+Enter`/Save committing, `Esc`/Cancel
discarding, and the same optimistic-write-with-rollback rule as the title. The
note is plain text with newlines; it SHALL NOT be rendered as Markdown/HTML and
SHALL NOT be sent to TTS.

An incoming `entry_updated` for the displayed entry (for example a status
change or a synthesis completion) SHALL NOT discard an open title or note draft.
The draft is applied or cancelled only by the user's own commit/cancel action.
A successful persisted update SHALL be reflected in the header; when no draft
is open, an externally changed note or title SHALL render the new value.

The metadata row SHALL show the date added as the local-time rendering of the
naive UTC `created_at` in the app language, a localized status badge, and the
engine and voice from the `generation` snapshot. The engine and voice labels
SHALL be engine-specific: Piper voices by their localized names, Silero
speakers by their speaker name, TeraTTS voices by their stored id, and an
unknown id by the stored id. An entry without a `generation` snapshot SHALL
show the explicit placeholders «не озвучено» for the engine and «—» for the
voice rather than an empty or guessed value.

The format `SegmentedControl` SHALL move from the old toolbar into the right end
of the metadata row; it SHALL NOT be rendered for a plain source that offers no
rendered view. The reading-typography controls (#354/#357) SHALL remain next to
it in the viewer header and SHALL stay available in every display mode,
including a plain source.

#### Scenario: Legacy entry renders the derived title

- **GIVEN** an entry with `title: null` and `original_text` starting with `## Установка`
- **WHEN** the viewer renders the header
- **THEN** the title line shows `Установка` and no explicit title is stored

#### Scenario: Title editing is keyboard-complete

- **GIVEN** an entry displayed in the viewer
- **WHEN** the user opens the title editor, types a value and presses `Enter`
- **THEN** `set_entry_title` persists the trimmed value, the header shows it
  immediately, and pressing `Esc` instead leaves the stored title untouched

#### Scenario: Clearing the title returns to the derived fallback

- **GIVEN** an entry with a stored title
- **WHEN** the user opens the editor, clears the input and saves
- **THEN** `null` is persisted and the header shows the derived title

#### Scenario: Note editing and clearing

- **GIVEN** an entry without a note
- **WHEN** the user clicks «Добавить заметку», types text and presses `Ctrl+Enter`
- **THEN** `set_entry_note` persists the trimmed note and the header shows the
  clamped note; clearing the textarea and saving removes it and restores the
  «Добавить заметку» state

#### Scenario: Draft survives an entry update

- **GIVEN** the note editor is open with an unsaved draft
- **WHEN** an `entry_updated` event changes the entry's status
- **THEN** the draft text and the open editor are preserved

#### Scenario: Failed save rolls back with a toast

- **GIVEN** an entry whose title is `"Старое"`
- **WHEN** the user saves a new title and the IPC call rejects
- **THEN** the header shows `"Старое"` again and a red localized error toast is
  shown

#### Scenario: Metadata row for an entry without audio

- **GIVEN** an entry whose `generation` is `null`
- **WHEN** the header renders
- **THEN** the metadata row shows the local date added, the status badge,
  «не озвучено» and «—»

#### Scenario: Metadata row for a recorded entry

- **GIVEN** a ready entry whose `generation` records engine `silero_native` and
  voice `xenia`
- **WHEN** the header renders
- **THEN** the metadata row shows `Готово`, `Silero (нативный)` and `Xenia`

#### Scenario: Reading controls stay available after the switch moves

- **GIVEN** a plain-text entry with no display switch
- **WHEN** the viewer renders
- **THEN** the reading width and scale controls are present in the header and
  the format switch is absent
