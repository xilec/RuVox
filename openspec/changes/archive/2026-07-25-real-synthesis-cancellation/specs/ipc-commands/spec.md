# Delta: ipc-commands

## MODIFIED Requirements

### Requirement: Synthesis Cancellation Command

The system SHALL provide `cancel_synthesis(id)` which actually stops the
entry's synthesis work and sets the entry status back to `pending`, emitting
`entry_updated`. Cancellation SHALL abort the entry's spawned synthesis task
via a per-entry abort registry. If the cancelled entry had already entered
the TTS stage, the system SHALL additionally terminate the current ttsd
subprocess; recovery then follows the ttsd-protocol auto-restart procedure,
and requests belonging to other entries are retried transparently. A late
completion or failure belonging to a cancelled entry SHALL be discarded: the
entry MUST NOT flip to `ready` or `error`, any audio/timestamp files written
by the late completion SHALL be removed, no further `entry_updated` for that
completion is emitted, and no autoplay starts. A missing entry fails with
`not_found`.

#### Scenario: cancel a queued synthesis

- GIVEN an entry with status `processing` whose request has not yet reached
  ttsd
- WHEN `cancel_synthesis` is invoked
- THEN the synthesis task is aborted, the entry status becomes `pending`,
  `entry_updated` is emitted, and the ttsd subprocess keeps running (no
  restart)

#### Scenario: cancel an in-flight synthesis

- GIVEN an entry with status `processing` whose request is being synthesized
  by ttsd
- WHEN `cancel_synthesis` is invoked
- THEN the synthesis task is aborted, the ttsd subprocess is terminated, the
  supervisor restarts it per the auto-restart procedure, and the entry
  status becomes `pending`

#### Scenario: late completion is discarded

- GIVEN an entry that was cancelled back to `pending` while its request was
  in flight
- WHEN the orphaned request completes
- THEN the entry remains `pending`, the generated audio/timestamp files are
  removed, no `entry_updated` with `ready` is emitted, and no autoplay
  starts

#### Scenario: cancel a missing entry

- GIVEN no entry with the given id
- WHEN `cancel_synthesis` is invoked
- THEN the command fails with `not_found`

### Requirement: Entry Lifecycle Events

The backend SHALL emit `entry_updated` with payload `{ entry: TextEntry }`
whenever an entry is created or any of its fields change: on ingestion
(`pending`), when synthesis starts (`processing`, `normalized_text` set), when
synthesis completes (`ready`, audio/timestamps paths and `duration_sec` set),
when synthesis fails (`error`, `error_message` set), after `delete_audio`,
`regenerate_entry`, `cancel_synthesis`, and after `clear_cache` for each reset
entry. A discarded late completion (after cancellation) SHALL NOT emit
`entry_updated` with `ready` or `error`. The backend SHALL emit
`entry_removed` with payload `{ id }` when an entry is removed from history
by a bulk operation; the frontend MUST drop the entry from local state
without expecting any `entry_updated` follow-up.

#### Scenario: synthesis progress is reflected via entry_updated

- GIVEN a newly ingested entry
- WHEN background synthesis runs to completion
- THEN the frontend receives `entry_updated` with `pending`, then
  `processing`, then `ready` carrying the audio path and duration

#### Scenario: no ready event after cancellation

- GIVEN an entry cancelled back to `pending`
- WHEN its orphaned synthesis completes
- THEN no `entry_updated` carrying `ready` is emitted for that completion

#### Scenario: bulk removal notification

- GIVEN `clear_cache` removed an entry from history
- WHEN the `entry_removed` event arrives
- THEN the payload is `{ id: "<uuid>" }` and no `entry_updated` follows for
  that entry
