# Delta: queue-lifecycle

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
an entry from `processing` back to `pending`; a synthesis completion or
failure arriving for an entry that is no longer `processing` SHALL be
discarded without changing its status (see the Synthesis Cancellation
Command requirement in `ipc-commands`).

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

#### Scenario: Cancellation returns the entry to pending

- GIVEN an entry in status `processing`
- WHEN `cancel_synthesis` runs
- THEN the entry status becomes `pending` and the entry can be regenerated
  later

#### Scenario: Stale completion does not change status

- GIVEN an entry that left `processing` (cancelled back to `pending`)
- WHEN its late synthesis completion or failure arrives
- THEN the entry status is left unchanged and the late result's files are
  removed
