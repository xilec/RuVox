# Delta: ipc-commands

## MODIFIED Requirements

### Requirement: Synthesis Cancellation Command

The system SHALL provide `cancel_synthesis(id)` which actually stops the
entry's synthesis work and sets the entry status back to `pending`, emitting
`entry_updated`. Cancellation SHALL abort the entry's spawned synthesis task
via a per-entry abort registry. If the cancelled entry had already entered
the TTS stage, the system SHALL additionally terminate the current ttsd
subprocess; recovery then follows the ttsd-protocol auto-restart procedure,
and requests belonging to other entries are retried transparently. If the
active engine was switched while the cancelled entry's synthesis was in
flight, cancellation SHALL also terminate the previous engine's ttsd
subprocess — the engine that is actually running the entry's request — so
the orphaned process does not keep consuming CPU. A late
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

#### Scenario: cancel after an engine switch

- GIVEN an entry with status `processing` whose request is being synthesized
  by ttsd on the Silero engine
- WHEN the active engine is switched to Piper and `cancel_synthesis` is then
  invoked for that entry
- THEN the synthesis task is aborted, the entry status becomes `pending`,
  and the swapped-out Silero engine's ttsd subprocess is terminated

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
