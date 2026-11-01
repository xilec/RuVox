# Delta: ipc-commands

## MODIFIED Requirements

### Requirement: Entry Regeneration Command

The system SHALL provide `regenerate_entry(id, play_when_ready)` which re-runs
background synthesis for the stored `original_text` with the current config
(speaker/voice, sample rate) while preserving the entry's existing audio and
timestamps until the fresh synthesis succeeds. The command SHALL set
`was_regenerated: true` and `error_message: null`, move the entry to
`pending` (from there the lifecycle runs `processing` → `ready` as for a new
entry), emit `entry_updated`, and start the background synthesis. Nothing
SHALL delete the current audio, timestamps or generation snapshot up front,
and the fresh audio SHALL be published atomically: engines write a staged
`<id>.next.wav`, the Opus encoder writes `<id>.opus.part` and renames it over
`<id>.opus` only after a complete encode, and the WAV fallback renames the
staged file over `<id>.wav`. A crash, a full disk, an aborted task or an
encode failure therefore leaves the entry's previous recording intact under
the name it references. The word timestamps SHALL be saved only after the
audio is published, so a failed publish cannot pair new timings with old
audio.

When the fresh synthesis succeeds the entry becomes `ready` with the new
audio, timestamps, `duration_sec` and generation snapshot, replacing the
previous artifacts in place. When it fails, the entry's previous recording
SHALL remain: an entry that still has a stored recording (`audio_path` set)
SHALL return to `ready` with `error_message` carrying the failure and a
`tts_error` event emitted for that failure — including failure classes other
than an engine error (`EmptyText`, `PipelinePanic`, `StoreFailed`), since a
restored `ready` entry otherwise looks indistinguishable from a successful
regeneration; an entry without a recording SHALL fail to
`error` as usual. When `play_when_ready` is `true` and regeneration succeeds,
the fresh audio SHALL start playing (the same autoplay rule as the initial
synthesis); when it is `false`, the regenerated audio is only cached. If the
entry is playing, playback SHALL be stopped first. Regeneration of an entry
with status `processing` SHALL be rejected with `synthesis_error` to avoid
racing the in-flight task.

#### Scenario: regenerate a ready entry

- GIVEN a `ready` entry and a changed `speaker` in the config
- WHEN `regenerate_entry` is invoked with `play_when_ready: false`
- THEN the entry keeps its audio and timestamps while the new synthesis runs,
  `entry_updated` is emitted with `was_regenerated: true`, and the fresh
  synthesis replaces the artifacts and advances the entry to `ready`

#### Scenario: failed regeneration keeps the previous audio

- GIVEN a `ready` entry with stored audio and an engine that rejects the text
- WHEN `regenerate_entry` is invoked
- THEN the audio and timestamps files are still on disk, the entry status is
  `ready` again, `error_message` carries the engine message, and a
  `tts_error` event is emitted

#### Scenario: failed first synthesis still errors

- GIVEN an entry that was never synthesized successfully (no stored recording)
- WHEN its synthesis fails
- THEN the entry status becomes `error`, since there is no previous audio to
  fall back to

#### Scenario: failed publish keeps the previous audio

- GIVEN a `ready` entry with a stored recording whose regeneration synthesized
  audio but could not publish it (encode rejected the WAV, full disk, an
  interrupted task)
- WHEN the task unwinds
- THEN the previous audio and timestamps are untouched, the entry returns to
  `ready` with `error_message`, and a `tts_error` event is emitted; a staged
  `<id>.next.wav` or `<id>.opus.part` left by the run is removed

#### Scenario: a WAV fallback recording is never overwritten in place

- GIVEN a `ready` entry whose stored recording is the WAV codec fallback
- WHEN its regeneration starts
- THEN the engine writes a staged WAV and the stored `<id>.wav` keeps its
  content until the fresh audio is renamed over it, so a failed or interrupted
  run leaves the previous recording intact

#### Scenario: failure before the TTS stage is reported

- GIVEN a `ready` entry with stored audio whose text normalizes to an empty
  string
- WHEN `regenerate_entry` runs
- THEN the entry returns to `ready` with its audio untouched, `error_message`
  carries the normalization failure, and a `tts_error` event is emitted

#### Scenario: regenerate with play-when-ready autoplays

- GIVEN a `ready` entry
- WHEN `regenerate_entry` is invoked with `play_when_ready: true`
- THEN after the fresh audio reaches `ready` it starts playing without
  further user action

#### Scenario: regenerate during synthesis is rejected

- GIVEN an entry with status `processing`
- WHEN `regenerate_entry` is invoked
- THEN the command fails with `type: "synthesis_error"` and the in-flight
  synthesis continues

#### Scenario: regenerate a cancelled entry

- GIVEN a `cancelled` entry
- WHEN `regenerate_entry` is invoked
- THEN synthesis restarts from `pending` and the entry reaches `ready` with
  fresh audio

### Requirement: Synthesis Cancellation Command

The system SHALL provide `cancel_synthesis(id)` which actually stops the
entry's synthesis work and sets the entry status to `cancelled`, emitting
`entry_updated` — except when the entry still holds a playable recording from
before a regeneration, in which case the status SHALL return to `ready`: the
recording was never removed, and `cancelled` is terminal for playback, so it
would orphan the entry's audio. Cancellation SHALL abort the entry's spawned
synthesis task via a per-entry abort registry. If the cancelled entry had
already entered the TTS stage, the system SHALL additionally terminate the
active engine's in-flight work — the ttsd subprocess for the Silero sidecar,
the synthesis cancellation flag for the in-process engines — so nothing keeps
synthesizing a cancelled request; recovery then follows the ttsd-protocol
auto-restart procedure, and requests belonging to other entries are retried
transparently. If the active engine was switched while the cancelled entry's
synthesis was in flight, cancellation SHALL also terminate the previous
engine's in-flight work — the engine that is actually running the entry's
request — so the orphaned process does not keep consuming CPU. A late
completion or failure belonging to a cancelled entry SHALL be discarded: the
entry MUST NOT flip to `ready` or `error`, any audio/timestamp files written
by the late completion SHALL be removed unless the entry still references that
file (regeneration keeps the canonical per-entry file names, so a restored
recording and a late result can share a name), no further `entry_updated` for
that completion is emitted, and no autoplay starts. A missing entry fails with
`not_found`. Cancelling a `pending` entry SHALL succeed (the entry is queued or
idle; a just-added entry may already have its synthesis task registered while
still in `pending`, and cancellation must abort it). An entry whose status is
`ready`, `playing`, `error`, or `cancelled` SHALL be rejected with
`synthesis_error` without changing its status or touching the synthesis
registries. A `cancelled` entry can return to the pipeline only through
`regenerate_entry`, which restarts synthesis from scratch.

#### Scenario: cancel a queued synthesis

- GIVEN a newly added entry with status `processing` whose request has not yet
  reached ttsd
- WHEN `cancel_synthesis` is invoked
- THEN the synthesis task is aborted, the entry status becomes `cancelled`,
  `entry_updated` is emitted, and the ttsd subprocess keeps running (no
  restart)

#### Scenario: cancel an in-flight synthesis

- GIVEN an entry with status `processing` whose request is being synthesized
  by ttsd
- WHEN `cancel_synthesis` is invoked
- THEN the synthesis task is aborted, the ttsd subprocess is terminated, the
  supervisor restarts it per the auto-restart procedure, and the entry
  status becomes `cancelled`

#### Scenario: cancel an in-flight synthesis on an in-process engine

- GIVEN an entry with status `processing` whose request is being synthesized
  by the in-process Piper engine
- WHEN `cancel_synthesis` is invoked
- THEN the chunked synthesis loop stops before the next chunk, any partial
  audio at the output path is removed, and the entry status becomes
  `cancelled`

#### Scenario: cancel after an engine switch

- GIVEN an entry with status `processing` whose request is being synthesized
  by ttsd on the Silero engine
- WHEN the active engine is switched to Piper and `cancel_synthesis` is then
  invoked for that entry
- THEN the synthesis task is aborted, the entry status becomes `cancelled`,
  and the swapped-out Silero engine's ttsd subprocess is terminated

#### Scenario: cancel a regeneration restores the previous audio

- GIVEN a `ready` entry with stored audio whose regeneration is in flight
- WHEN `cancel_synthesis` is invoked
- THEN the synthesis task is aborted, the audio and timestamps files are
  untouched, and the entry status returns to `ready`

#### Scenario: late completion is discarded

- GIVEN an entry that was cancelled while its request was in flight
- WHEN the orphaned request completes
- THEN the entry remains `cancelled` (or `ready` with its previous audio when
  it was a cancelled regeneration), files the entry still references are
  kept, files it does not reference are removed, no further `entry_updated` is
  emitted for the late completion, and no autoplay starts

#### Scenario: cancel a missing entry

- GIVEN no entry with the given id
- WHEN `cancel_synthesis` is invoked
- THEN the command fails with `not_found`

#### Scenario: cancel an idle entry

- GIVEN an entry with status `pending` and no synthesis in flight
- WHEN `cancel_synthesis` is invoked
- THEN the command succeeds, the entry becomes `cancelled`, and
  `entry_updated` is emitted

#### Scenario: cancel a terminal entry

- GIVEN an entry with status `ready`, `playing`, `error`, or `cancelled`
- WHEN `cancel_synthesis` is invoked
- THEN the command fails with `synthesis_error`, the entry status is left
  unchanged, and no `entry_updated` is emitted
