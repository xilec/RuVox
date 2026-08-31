## MODIFIED Requirements

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
normalizes `playing` back to `ready` on save. `cancel_synthesis` SHALL move
an entry from `processing` or `pending` to `cancelled`; `cancelled` is
terminal until the entry is regenerated (which restarts the lifecycle at
`pending`). A synthesis completion or failure arriving for an entry that is
no longer `processing` SHALL be discarded without changing its status (see
the Synthesis Cancellation Command requirement in `ipc-commands`).

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

#### Scenario: Cancellation marks the entry cancelled

- GIVEN an entry in status `processing`
- WHEN `cancel_synthesis` runs
- THEN the entry status becomes `cancelled` and the entry can be regenerated
  later, which restarts its synthesis from scratch

#### Scenario: Stale completion does not change status

- GIVEN an entry that left `processing` (cancelled to `cancelled`)
- WHEN its late synthesis completion or failure arrives
- THEN the entry status is left unchanged and the late result's files are
  removed

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

#### Scenario: Entries sorted newest first

- GIVEN entries with different `created_at` timestamps
- WHEN the queue is rendered
- THEN entries appear sorted by `created_at` descending

#### Scenario: Live status update

- GIVEN an entry displayed with status `Обработка`
- WHEN an `entry_updated` event arrives with status `ready`
- THEN the entry's badge switches to `Готово` and the duration appears
  without reloading the list
