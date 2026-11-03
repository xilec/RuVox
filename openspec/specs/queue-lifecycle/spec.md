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
a user-visible message in `error_message` — except when the entry already
holds a stored recording from an earlier synthesis (a failed regeneration):
such an entry SHALL return to `ready` with `error_message` set and its audio,
timestamps and generation snapshot untouched, because that recording is still
valid and `error` would make it unplayable. Regeneration starts the same
lifecycle at `pending` but is non-destructive: the existing recording stays
until the fresh synthesis replaces it. After each status change the
system SHALL emit an `entry_updated` event. When the entry was added with
`play_when_ready = true`, the system SHALL start playback automatically once
the entry reaches `ready`; auto-play failures MUST NOT flip the entry into
`error`. The `playing` status is transient only — the storage layer
normalizes `playing` back to `ready` on save. `cancel_synthesis` SHALL move
an entry from `processing` or `pending` to `cancelled`, or back to `ready`
when the entry still has a stored recording from before a regeneration;
`cancelled` is terminal until the entry is regenerated (which restarts the
lifecycle at `pending`). A synthesis completion or failure arriving for an
entry that is no longer `processing` SHALL be discarded without changing its
status (see the Synthesis Cancellation Command requirement in
`ipc-commands`).

#### Scenario: Successful synthesis

- GIVEN a newly added entry with status `pending`
- WHEN normalization and TTS synthesis complete successfully
- THEN the entry status becomes `ready`, `audio_path`, `timestamps_path` and
  `duration_sec` are populated, and an `entry_updated` event is emitted

#### Scenario: Synthesis failure

- GIVEN an entry without a stored recording, in status `processing`
- WHEN the TTS engine fails
- THEN the entry status becomes `error`, `error_message` is set, and a
  `tts_error` event is emitted

#### Scenario: Failed regeneration keeps the entry ready

- GIVEN a `ready` entry with stored audio whose regeneration is `processing`
- WHEN the TTS engine fails
- THEN the entry status returns to `ready`, its audio, timestamps and
  generation snapshot are unchanged, `error_message` is set, and a
  `tts_error` event is emitted

#### Scenario: Read-now add

- GIVEN an entry added with `play_when_ready = true`
- WHEN the entry reaches status `ready`
- THEN playback of the entry's audio starts automatically

#### Scenario: Cancellation moves the entry to cancelled

- GIVEN an entry without a stored recording, in status `processing`
- WHEN `cancel_synthesis` runs
- THEN the entry status becomes `cancelled` and the entry can be regenerated
  later, which restarts its synthesis from scratch

#### Scenario: Cancelling a regeneration keeps the entry ready

- GIVEN a `ready` entry with stored audio whose regeneration is in flight
- WHEN `cancel_synthesis` runs
- THEN the entry status returns to `ready` and its recording stays playable

#### Scenario: Stale completion does not change status

- GIVEN an entry that left `processing` (cancelled to `cancelled`, or a
  cancelled regeneration restored to `ready`)
- WHEN its late synthesis completion or failure arrives
- THEN the entry status is left unchanged, files the entry still references
  are kept, and the late result's other files are removed

### Requirement: Queue list rendering

The system SHALL display all entries in the left navbar, sorted by
`created_at` descending (newest first). For each entry the list SHALL show:
a preview of the first 60 characters of `original_text` (with an ellipsis
when truncated), a color-coded status badge with a Russian label
(`Ожидание` / `Обработка` / `Готово` / `Играет` / `Ошибка` / `Отменено`),
and the duration formatted as `M:SS` once `duration_sec` is available. The
`processing` badge SHALL include a spinner. The list SHALL update live from
`entry_updated` and `entry_removed` events without a full reload. When the
queue is empty the system SHALL show the hint "Скопируйте текст и нажмите
Add".

A `processing` entry SHALL additionally show an indeterminate progress line
and the elapsed-time text «Синтез… M:SS» (for example «Синтез… 0:42»), and a
`pending` entry SHALL show «В очереди»; neither shows a percentage, ETA or
chunk stage. The elapsed time SHALL be counted from the moment the frontend
observed the entry become `processing`, and the progress line SHALL disappear
when the entry reaches any other status. Because the persisted entry carries
no start timestamp, the elapsed time SHALL start over after an application
restart.

#### Scenario: Entries sorted newest first

- GIVEN entries with different `created_at` timestamps
- WHEN the queue is rendered
- THEN entries appear sorted by `created_at` descending

#### Scenario: Live status update

- GIVEN an entry displayed with status `Обработка`
- WHEN an `entry_updated` event arrives with status `ready`
- THEN the entry's badge switches to `Готово` and the duration appears
  without reloading the list

#### Scenario: Processing row shows elapsed time

- GIVEN an entry with status `processing`
- WHEN its card has been rendered for 42 seconds
- THEN the card shows «Синтез… 0:42» and the indeterminate progress line

#### Scenario: Pending row shows no timer

- GIVEN an entry with status `pending`
- WHEN its card is rendered
- THEN the card shows «В очереди» and no elapsed-time text

#### Scenario: Terminal status removes the progress line

- GIVEN a card showing the elapsed time for a `processing` entry
- WHEN `entry_updated` reports `ready`, `error` or `cancelled` for that entry
- THEN the progress line and the elapsed-time text disappear and the
  one-second interval stops

#### Scenario: Restart restarts the elapsed time

- GIVEN an entry that was `processing` when the application restarted
- WHEN the queue loads and renders its card
- THEN the elapsed time counts from 0 again, because the persisted entry
  carries no start timestamp

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
"Воспроизвести", "Перегенерировать аудио" (disabled while `processing`),
"Отменить синтез" (enabled only while `processing`, invoking
`cancel_synthesis` without a confirmation dialog — cancellation is
non-destructive: the entry returns to `pending` and can be regenerated),
and
"Удалить". The currently playing entry SHALL be visually highlighted while
playback is active or paused; the highlight clears on stop or finish. When
the playing entry is scrolled out of view, the system SHALL show a
"К читаемому" button that selects the playing entry and scrolls it into view.

#### Scenario: Play from the queue

- GIVEN an entry with status `ready`
- WHEN the user clicks its Play button
- THEN `play_entry` is invoked and the entry is highlighted as playing

#### Scenario: Cancel a running synthesis from the menu

- GIVEN an entry with status `processing`
- WHEN the user opens its context menu and clicks "Отменить синтез"
- THEN `cancel_synthesis` is invoked for that entry

#### Scenario: Cancel item is unavailable outside processing

- GIVEN an entry with any status other than `processing`
- WHEN the user opens its context menu
- THEN the "Отменить синтез" item is disabled

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

### Requirement: Persisted failure reason

The system SHALL persist the latest synthesis failure reason in the entry's
`error_message` field exactly as the engine or pipeline reported it — no
parsing, translation, truncation or classification — and SHALL keep the field
`null` for entries whose last synthesis succeeded. Because the field lives in
the entry history, it SHALL survive an application restart; no transient
event is required to recover it. A regeneration attempt SHALL clear
`error_message` when it starts, so the UI can distinguish an in-flight retry
from the failure it replaces. A failed regeneration SHALL store the failure
reason while keeping the previous recording playable (see the Entry status
lifecycle requirement).

#### Scenario: failure reason survives a restart

- GIVEN an entry whose synthesis failed with the engine message
  "модель не загружена"
- WHEN the application restarts and loads the queue
- THEN the entry still carries status `error` and `error_message`
  "модель не загружена"

#### Scenario: starting a regeneration clears the stored reason

- GIVEN an entry with status `error` or `ready` and `error_message` set
- WHEN `regenerate_entry` is invoked
- THEN the entry moves to `pending` with `error_message: null` and emits
  `entry_updated`

#### Scenario: a successful regeneration leaves no failure reason

- GIVEN an entry whose failed regeneration is retried
- WHEN the fresh synthesis reaches `ready`
- THEN `error_message` is `null`

### Requirement: Inline synthesis cancel

A `pending` or `processing` queue card SHALL show an inline «Отменить»
action that is visible without hovering and invokes `cancel_synthesis` for
that entry exactly once per activation. The action SHALL be disabled while
its command is in flight. A rejected command SHALL surface the existing
localized red error notification. This action is in addition to the
"Отменить синтез" context-menu item, whose behavior is unchanged.

#### Scenario: Cancel a running synthesis from the card

- GIVEN an entry with status `processing`
- WHEN the user clicks its inline «Отменить» action
- THEN `cancel_synthesis` is invoked exactly once for that entry

#### Scenario: Cancel a queued synthesis from the card

- GIVEN an entry with status `pending`
- WHEN the user clicks its inline «Отменить» action
- THEN `cancel_synthesis` is invoked exactly once for that entry

#### Scenario: Cancel action is disabled while the command is in flight

- GIVEN a card showing the inline «Отменить» action
- WHEN its `cancel_synthesis` command has not settled yet
- THEN the action is disabled and a second click does not invoke the command
  again

#### Scenario: Cancel failure surfaces the localized toast

- GIVEN a card with a visible inline «Отменить» action
- WHEN `cancel_synthesis` rejects
- THEN the existing localized red error notification is shown and the action
  becomes enabled again

