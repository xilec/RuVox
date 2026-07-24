# Queue Lifecycle Specification

## Purpose

Covers how text entries enter the queue (clipboard add flow, with or without
the normalization preview dialog), how their synthesis status progresses, and
how the queue is presented in the UI: entry list rendering, sorting, search,
per-entry actions, deletion, and the resizable queue panel.

## Requirements

### Requirement: Clipboard add flow

The system SHALL read text from the system clipboard when the user activates
the **Add** button in the queue panel header, using
`tauri-plugin-clipboard-manager` on the frontend. When the
`preview_dialog_enabled` config flag is on (default `true`), the system SHALL
open the normalization preview dialog instead of adding the entry directly;
otherwise the text SHALL be passed straight to `add_text_entry`. The system
SHALL reject blank or whitespace-only text with an error and MUST NOT create
an entry for it.

#### Scenario: Add with preview dialog enabled

- GIVEN `preview_dialog_enabled` is `true` in the config
- WHEN the user clicks **Add** and the clipboard contains text
- THEN the preview dialog opens with the original and normalized text, and no
  entry is created yet

#### Scenario: Add with preview dialog disabled

- GIVEN `preview_dialog_enabled` is `false`
- WHEN the user clicks **Add** and the clipboard contains text
- THEN a new entry is created and background synthesis starts immediately

#### Scenario: Clipboard read failure

- GIVEN the clipboard cannot be read or contains no text
- WHEN the user clicks **Add**
- THEN an error notification is shown and no entry is created

### Requirement: Entry status lifecycle

The system SHALL move each entry through the statuses `pending` ->
`processing` -> `ready` during background synthesis: the entry is persisted
as `pending`, switched to `processing` once normalization completes, and to
`ready` once the audio file, word timestamps, and `duration_sec` are stored.
On any synthesis failure the system SHALL set the status to `error` and store
a user-visible message in `error_message`. After each status change the
system SHALL emit an `entry_updated` event. When the entry was added with
`play_when_ready = true`, the system SHALL start playback automatically once
the entry reaches `ready`; auto-play failures MUST NOT flip the entry into
`error`. The `playing` status is transient only — the storage layer
normalizes `playing` back to `ready` on save.

#### Scenario: Successful synthesis

- GIVEN a newly added entry with status `pending`
- WHEN normalization and TTS synthesis complete successfully
- THEN the entry status becomes `ready`, `audio_path`, `timestamps_path` and
  `duration_sec` are populated, and an `entry_updated` event is emitted

#### Scenario: Synthesis failure

- GIVEN an entry in status `processing`
- WHEN the TTS engine fails
- THEN the entry status becomes `error`, `error_message` is set, and a
  `tts_error` event is emitted

#### Scenario: Read-now add

- GIVEN an entry added with `play_when_ready = true`
- WHEN the entry reaches status `ready`
- THEN playback of the entry's audio starts automatically

### Requirement: Queue list rendering

The system SHALL display all entries in the left navbar, sorted by
`created_at` descending (newest first). For each entry the list SHALL show:
a preview of the first 60 characters of `original_text` (with an ellipsis
when truncated), a color-coded status badge with a Russian label
(`Ожидание` / `Обработка` / `Готово` / `Играет` / `Ошибка`), and the
duration formatted as `M:SS` once `duration_sec` is available. The `processing`
badge SHALL include a spinner. The list SHALL update live from `entry_updated`
and `entry_removed` events without a full reload. When the queue is empty the
system SHALL show the hint "Скопируйте текст и нажмите Add".

#### Scenario: Entries sorted newest first

- GIVEN entries with different `created_at` timestamps
- WHEN the queue is rendered
- THEN entries appear sorted by `created_at` descending

#### Scenario: Live status update

- GIVEN an entry displayed with status `Обработка`
- WHEN an `entry_updated` event arrives with status `ready`
- THEN the entry's badge switches to `Готово` and the duration appears
  without reloading the list

### Requirement: Queue search

The system SHALL provide a search field above the queue that filters entries
by case-insensitive substring match on `original_text`. The `Ctrl+F` / `Cmd+F`
hotkey SHALL focus the search field, and `Escape` in the field SHALL clear the
query. When no entry matches, the system SHALL show "Ничего не найдено".

#### Scenario: Filtering entries

- GIVEN a non-empty queue and a search query
- WHEN the user types into the search field
- THEN only entries whose `original_text` contains the query
  (case-insensitive) are shown

### Requirement: Per-entry actions

Each queue item SHALL offer a Play action (enabled only for `ready` entries)
that invokes `play_entry`, and a right-click context menu with the items
"Воспроизвести", "Перегенерировать аудио" (disabled while `processing`), and
"Удалить". The currently playing entry SHALL be visually highlighted while
playback is active or paused; the highlight clears on stop or finish. When
the playing entry is scrolled out of view, the system SHALL show a
"К читаемому" button that selects the playing entry and scrolls it into view.

#### Scenario: Play from the queue

- GIVEN an entry with status `ready`
- WHEN the user clicks its Play button
- THEN `play_entry` is invoked and the entry is highlighted as playing

#### Scenario: Jump to playing entry

- GIVEN an entry is playing and the user scrolled it out of the viewport
- WHEN the user clicks "К читаемому"
- THEN the playing entry becomes selected and is scrolled into the center of
  the list

### Requirement: Entry deletion

The system SHALL ask for confirmation ("Удалить запись?") before deleting an
entry. On confirmation the system SHALL invoke `delete_entry`, remove the
entry and its audio file, emit `entry_removed`, and clear the selection if
the deleted entry was selected.

#### Scenario: Delete with confirmation

- GIVEN an entry in the queue
- WHEN the user chooses "Удалить" and confirms the dialog
- THEN the entry disappears from the list and its audio file is removed

#### Scenario: Cancel deletion

- GIVEN the delete confirmation dialog is open
- WHEN the user clicks "Отмена"
- THEN the entry remains in the queue unchanged

### Requirement: Resizable queue panel

The system SHALL let the user resize the queue navbar by dragging its right
border, clamping the width between 180 px and 70% of the window width.

#### Scenario: Drag to resize

- GIVEN the queue navbar at its default width
- WHEN the user drags the right border of the navbar
- THEN the navbar width follows the pointer, never going below 180 px or
  above 70% of the window width
