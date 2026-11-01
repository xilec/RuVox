## Why

The last regeneration attempt failed with a TeraTTS text error, and the
entry lost its audio: `regenerate_entry` calls `delete_audio` before it spawns
synthesis, so a run that never reaches a successful synthesis leaves the entry
`error` with `audio_path: null` and the previous recording physically removed.
A working recording is destroyed by a failure in the *replacement* work — the
opposite of what "regenerate" promises. Nothing about the new synthesis
requires the old file to be gone first: engines write their output only after a
successful synthesis and the Opus transcode replaces the file in a single
rename.

## What Changes

- `regenerate_entry` stops deleting the current audio/timestamps up front. The
  entry keeps its recording and metadata until the fresh synthesis succeeds,
  and the lifecycle still runs pending → processing → ready.
- A **failed** regeneration keeps the previous recording: the entry returns to
  `ready` (it is playable again), `error_message` carries the failure, and the
  `tts_error` event still fires. An entry that had no recording before the
  regeneration still fails to `error`.
- **Cancelling** a regeneration that had a recording returns the entry to
  `ready` instead of `cancelled`; an entry without a recording keeps today's
  `processing → cancelled` transition.
- Late-result cleanup no longer removes a file the entry still references, so
  a stale completion or failure cannot delete the recording that a
  cancel/failure just restored.
- Orchestration tests pin all three outcomes against the stub engine (success
  replaces the file in place, failure keeps it, cancel keeps it), and the spec
  text for regeneration, cancellation and the status lifecycle is updated.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `ipc-commands`: the "Entry Regeneration Command" requirement becomes
  non-destructive with a failure path that restores `ready`; the "Synthesis
  Cancellation Command" requirement gains the regeneration-cancel outcome and
  the referenced-file exception in the late-result cleanup.
- `queue-lifecycle`: the "Entry status lifecycle" requirement qualifies the
  failure rule for an entry that already has a stored recording.

## Impact

- Affected code: `src-tauri/src/commands/mod.rs` (`regenerate_entry`,
  `cancel_entry`, `apply_error_if_current`, `discard_late_files`),
  `src-tauri/src/commands/orchestration_tests.rs`, and the toast routing in
  `src/lib/notificationBridge.ts`.
- No storage format or IPC payload changes. The frontend needs one adjustment:
  the synthesis toast used to treat every `processing → ready` transition as a
  completion, so a restored recording would announce a green "synthesis
  complete" for a run that failed or was cancelled. The bridge now compares the
  entry's `generation_count` with the value captured when the spinner appeared
  and hides the toast instead when nothing was baked.
- User-visible: a failed or cancelled regeneration no longer costs the user
  the audio they already had.

## Non-goals

- Staging synthesis output under temporary names (fully transactional
  write-rename). The canonical per-entry filenames stay; see design for the
  trade-off.
- Retrying a failed regeneration automatically, or changing engine error
  mapping.
- Changing the explicit `delete_audio` command, cache eviction or the
  `clear_cache` behaviour.
- New frontend error surfaces: the existing toast plus `error_message` are the
  channels; the only frontend change is the toast rule above.
