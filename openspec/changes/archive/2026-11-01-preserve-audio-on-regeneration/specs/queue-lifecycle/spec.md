# Delta: queue-lifecycle

## MODIFIED Requirements

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
