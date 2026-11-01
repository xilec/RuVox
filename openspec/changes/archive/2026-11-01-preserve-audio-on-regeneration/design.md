## Context

`regenerate_entry` was written as "drop the current audio, then synthesize":
it calls `StorageService::delete_audio` (which also clears `audio_path`,
`timestamps_path`, `duration_sec`, `generation` and resets the status to
`pending`) *before* spawning the synthesis task. Everything after that point
can fail — normalization, engine load, the text itself, a cancelled task — and
the user is left with no recording, even though the failure happened in the
work that was supposed to *replace* a perfectly good file.

The incident: regeneration on TeraTTS failed with `text_invalid_tags`, the
entry became `error` with `audio_path: null`, and the previous recording was
already gone from the audio directory.

Two facts shape the fix:

- The synthesis output path is deterministic per entry (`<id>.wav`, then the
  Opus transcode renames it to `<id>.opus`), and every engine writes its WAV
  only after a complete synthesis. The only moment the stored recording is
  replaced is that final rename.
- Startup reconciliation already repairs a crash: an entry with an
  `audio_path` whose file exists is restored to `ready`, and only a missing
  file resets the metadata (`storage::service`). So "entry still points at a
  recording that exists" is a state the app knows how to trust.

## Goals / Non-Goals

**Goals:**

- A regeneration that fails in any stage leaves the previous recording on
  disk, playable, with its metadata intact.
- Cancelling a regeneration returns the entry to a playable state.
- Late results of a cancelled task never delete a file the entry still uses.
- No storage/format/IPC change; the frontend only needs the synthesis toast to
  stop announcing a completion when no new audio was baked.

**Non-Goals:**

- Fully transactional staging of synthesis output under temporary names.
- Automatic retry, engine error mapping, or the explicit `delete_audio`
  command / eviction / `clear_cache` paths.
- New UI surfaces for the failure: the `tts_error` toast and `error_message`
  remain the channels.

## Decisions

### D1. Stop deleting before synthesis; publish the new audio atomically

`regenerate_entry` keeps the artifacts and metadata, sets `was_regenerated`
and clears `error_message`, moves the entry to `pending` explicitly (it used
to be a side effect of `delete_audio`), emits `entry_updated` and spawns the
task.

Publication is staged, because the canonical per-entry names are also the
names a previous recording may occupy:

- Engines write their WAV to `<id>.next.wav`, never to `<id>.wav`.
- `encode_wav_to_opus` (through `encode_wav_to_opus_atomic`) writes
  `<id>.opus.part` and renames it over `<id>.opus` only after a complete
  encode; a crash, a full disk or an aborted task leaves the previous Opus
  untouched and the `.part` file is removed on failure.
- When the encoder rejects the WAV, the staged file is renamed over `<id>.wav`
  — the WAV fallback still publishes in one step, and a rename failure makes
  the synthesis fail with `StoreFailed` instead of leaving the entry pointing
  at stale audio.
- The timestamps are saved *after* the audio is published: a failed publish
  cannot pair new word timings with old audio. A timestamp-save failure stays
  best-effort (`None`), as before.

The reviewer's crash-window analysis is the reason for the staging: without
it, an interrupted Opus encode or a WAV fallback entry being overwritten by
the engine would leave a truncated file under the name the entry references,
which startup reconciliation would then promote to `ready`.

Rejected: keeping the delete but restoring from a backup copy. It doubles the
I/O on every regeneration and adds a restore path that itself can fail.
Rejected: staging only for WAV-backed entries. The branch has one
publication path, not two; a conditional would need the entry's current
`audio_path` inside `synthesize_audio` for no behavioral gain.

### D2. A failed regeneration returns the entry to `ready`, not `error`

`apply_error_if_current` still stores the message, but it now picks the status
from the entry's own state: a stored `audio_path` means the entry owns a
playable recording, so it returns to `ready`; without one (a first synthesis, a
`cancelled` restart) it becomes `error` as before. The check is a field read,
deliberately not a filesystem probe, so the storage write lock stays free of
I/O.

A `ready` entry with an error message looks healthy in the UI (the queue badge
keys on the status and nothing renders `error_message`), so the worker emits
`tts_error` for every applied failure that leaves a playable entry — not only
`TtsFailed`. The red toast is the only in-app signal that the regeneration did
nothing; a first synthesis without audio still lands on `error`, where the
queue already shows the failure, so it keeps its old event rule.

The status flip is the one place where the frontend needed a matching rule: the
synthesis toast reacted to `processing → ready` as "done". `entry_updated`
carries `generation_count`, which increments only when fresh audio lands, so the
bridge remembers the value at spinner time and hides the toast when the ready
event did not move it — covering both the failed and the cancelled path without
a new event or payload field.

Rejected: `error` status with `audio_path` preserved. The data would survive
but the entry would be unplayable (`play_entry` requires `ready`), so the user
would still lose the recording in every way that matters.

Rejected: a dedicated `regeneration_failed` status. It would push a new state
into the storage model, the frontend status mapping, the toast lifecycle and
every exhaustive `match` for a case fully described by "ready + error
message".

### D3. Cancelling a regeneration restores `ready` too

Same field rule in `cancel_entry`'s atomic transition: `processing`/`pending`
with an `audio_path` → `ready`, without one → `cancelled`. A `cancelled` entry
is terminal for playback, so parking a still-valid recording there would
orphan it from the state machine — the exact hazard the function's own doc
comment warns about.

### D4. Late-result cleanup keeps files the entry references

`discard_late_files` now skips a candidate name equal to the entry's current
`audio_path`/`timestamps_path`. With canonical per-entry names, a late result
and a restored recording can share a name, so the guard is the only way to
tell them apart.

Rejected: staging bytes under `<id>.pending.*` so a late result's files are
unambiguously its own. That is the fully transactional solution, but it
threads a second filename set through synthesis, timestamp saving, the
transcode and the entry update, and leaves its own orphan class after a crash.
Revisit if a late completion ever overwrites a restored recording in practice.

### D5. The remaining race, accepted

A synthesis that succeeds between a cancel and its stale-result check already
renamed its Opus over the stored file; the guard then keeps that file because
the entry references it. The entry therefore holds the *new* audio even though
the regeneration was reported as cancelled. The file is valid and is what the
entry points at, so this is a cosmetic inconsistency of a rare interleaving,
not data loss — the opposite direction from the bug being fixed.

A narrower variant of the same race: when the abort lands exactly while the
blocking encode is running, `spawn_blocking` finishes (aborting the future does
not stop it) and may publish the new Opus after the cancel restored `ready`.
The timestamps save runs *after* that await, so it never happens — the entry
then has the new audio with the previous timestamps, and word highlighting
drifts until the next regeneration.

Two mitigations were considered and not taken: a `CancelFlag` checked before the
rename (the pattern Piper's engine already uses for its chunk loop) would shrink
the window from the whole encode to a single syscall but not close it, and only
a two-file transactional publish (journal, or one directory rename) eliminates
it. Against request cancellation every other path publishes audio and
timestamps together; a process crash between the two writes has a mirror-image
window in any order, which no journal-free scheme avoids. The chosen order
(audio first) is the better one — a publish failure is more likely than a crash
in that instruction window, and the old order had the opposite window.

When the late result is discarded after such a publish, the timestamps go with
the audio they describe (`entry_kept_the_new_audio` in `apply_ready_if_current`):
the canonical `<id>.timestamps.json` name is always "referenced", so keeping it
would pair old audio with new timings. Wrong highlighting is worse than none.

## Risks / Trade-offs

- During regeneration the entry is `pending`/`processing` with an `audio_path`,
  so it is visible in history but not playable until the outcome is known. The
  old code had the same non-playable window with no audio at all.
- `export_audio` and the eviction scan use `get_audio_path`, so they can see
  the old recording during the regeneration window — harmless: it is a real,
  playable file and eviction recomputes from storage.
- `error_message` on a `ready` entry is a new state combination; it is
  rendered only through the error toast today, and the next regeneration
  clears it. Any future "show entry error" surface must gate on the message
  rather than on `status == error`.
- Crash mid-regeneration leaves `pending`/`processing` with a valid file;
  startup reconciliation restores `ready` (verified behaviour, pinned by
  storage tests).

## Tests

- Orchestration test (real command, `StubEngine`): failed regeneration keeps
  the file bytes, restores `ready`, sets `error_message`, emits `tts_error`
  and `entry_updated`.
- Orchestration test: cancelling a regenerating entry (blocked engine)
  restores `ready` and keeps the file.
- Unit test for `discard_late_files`: a referenced name survives, an
  unreferenced one is removed, and the transient `<id>.next.wav` /
  `<id>.opus.part` names are removed too.
- Frontend unit test for the synthesis toast (`notificationBridge.test.ts`): a
  moved `generation_count` announces the completion, an unchanged one hides the
  spinner, an `error` transition hides it.
- Audio unit tests: `encode_wav_to_opus_atomic` keeps the previously published
  `.opus` on an encode failure, removes its `.part` file, and leaves no `.part`
  after success.
- Orchestration test: a WAV-backed entry's recording survives a failed
  regeneration and the staged WAV is cleaned up.
- Orchestration test: a normalization failure (empty text) on a regeneration
  restores `ready`, keeps the audio, and emits `tts_error`.
- Unit test: `finalize_audio_files` publishes the staged WAV over `<id>.wav`
  when the encoder rejects it, and reports `StoreFailed` otherwise.

## Migration Plan

None — no persisted schema or payload change. Entries that already lost their
audio stay as they are; the behavior applies to the next regeneration.
