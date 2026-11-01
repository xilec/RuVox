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

### D1. Stop deleting before synthesis; the successful transcode replaces the file

`regenerate_entry` keeps the artifacts and metadata, sets `was_regenerated`
and clears `error_message`, moves the entry to `pending` explicitly (it used
to be a side effect of `delete_audio`), emits `entry_updated` and spawns the
task. On success the worker's `replace_wav_with_opus` rename swaps the stored
file and `apply_ready_if_current` records the new duration/generation/count;
on any failure before that rename the previous file was never touched.

Rejected: keeping the delete but restoring from a backup copy. It doubles the
I/O on every regeneration and adds a restore path that itself can fail.

### D2. A failed regeneration returns the entry to `ready`, not `error`

`apply_error_if_current` still stores the message and still emits the
`tts_error` event, but it now picks the status from the entry's own state: a
stored `audio_path` means the entry owns a playable recording, so it returns
to `ready`; without one (a first synthesis, a `cancelled` restart) it becomes
`error` as before. The check is a field read, deliberately not a filesystem
probe, so the storage write lock stays free of I/O.

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
  unreferenced one is removed.
- Frontend unit test for the synthesis toast (`notificationBridge.test.ts`): a
  moved `generation_count` announces the completion, an unchanged one hides the
  spinner, an `error` transition hides it.

## Migration Plan

None — no persisted schema or payload change. Entries that already lost their
audio stay as they are; the behavior applies to the next regeneration.
