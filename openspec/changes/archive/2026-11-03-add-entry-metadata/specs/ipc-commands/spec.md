## ADDED Requirements

### Requirement: Entry Metadata Commands

The backend SHALL expose two commands that persist the entry's editable
metadata:

- `set_entry_title(id, title: Option<String>)` — the display title;
- `set_entry_note(id, note: Option<String>)` — the free-form note.

Two commands are required instead of one patch command because a Tauri
`Option<String>` argument cannot distinguish "leave unchanged" from "clear":
`null` and an absent value both deserialize to `None`. A single metadata patch
would therefore make clearing a field ambiguous.

Both commands SHALL normalize their input before storing it: trim leading and
trailing whitespace; treat an empty or whitespace-only value as absent
(`None`); cap the title at 200 characters and the note at 2000 characters
(counted in Unicode scalar values, never splitting a character), re-trimming
the end after the cap. The stored value SHALL NOT otherwise be rewritten
(no whitespace collapsing, no Markdown processing).

The write SHALL be applied atomically through the storage layer's conditional
update (`StorageService::update_entry_if`) so a concurrent synthesis completion
that lands between the command's read and write cannot be rolled back, and the
entry SHALL be persisted. After a successful write the backend SHALL emit
`entry_updated` with the updated entry. The command SHALL NOT touch
`original_text`, `normalized_text`, the source `format`, or any audio and
timestamp fields: notes and titles are metadata and never feed the TTS
pipeline.

An unknown entry id SHALL be rejected with a typed `not_found`-style
`CommandError` and SHALL emit no event. A malformed id SHALL be rejected like
the other entry commands. The commands SHALL be registered in the Tauri
handler and wrapped in `src/lib/tauri.ts`.

#### Scenario: Title is trimmed and persisted

- **GIVEN** a stored entry
- **WHEN** the frontend calls `invoke("set_entry_title", { id, title: "  Новое имя  " })`
- **THEN** `entry.title` is `"Новое имя"`, `entry_updated` carries the updated
  entry, and a reload from `history.json` keeps the value

#### Scenario: Empty title clears the stored value

- **GIVEN** an entry whose `title` is `"Старое имя"`
- **WHEN** the frontend calls `set_entry_title` with `""`, `"   "` or `null`
- **THEN** `entry.title` is `null` and the UI falls back to the title derived
  from `original_text`

#### Scenario: Limits are enforced in characters

- **GIVEN** an entry and a 250-character title and a 2100-character note
- **WHEN** `set_entry_title` and `set_entry_note` are called
- **THEN** the stored title has at most 200 characters, the stored note at most
  2000, and neither was truncated mid-character

#### Scenario: Note editing never touches synthesis data

- **GIVEN** a ready entry with `normalized_text` set and audio on disk
- **WHEN** `set_entry_note` changes the note
- **THEN** `normalized_text`, `status`, `audio_path` and `timestamps_path` are
  unchanged and no synthesis is started

#### Scenario: Unknown entry is rejected

- **GIVEN** no entry with the given id
- **WHEN** `set_entry_title` or `set_entry_note` is called
- **THEN** the command rejects with a typed error and emits no `entry_updated`

#### Scenario: Concurrent completion is not rolled back

- **GIVEN** an entry that finishes synthesis while the user saves a title
- **WHEN** the title command applies its mutation
- **THEN** the persisted entry carries both the new title and the completed
  status/audio fields; neither write reverts the other
