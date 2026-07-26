# Tasks: Real synthesis cancellation

## Implementation

- [x] `src-tauri/src/state.rs`: add `synthesis_tasks:
  Mutex<HashMap<EntryId, AbortHandle>>` and `synthesize_entered:
  Mutex<HashSet<EntryId>>` to `AppState`.
- [x] `src-tauri/src/commands/mod.rs` `spawn_synthesis`: register the task's
  `AbortHandle`; mark the entry in `synthesize_entered` around the
  `tts.synthesize()` await; unregister both on any task outcome.
- [x] `src-tauri/src/commands/mod.rs`: stale-completion guard in
  `mark_ready_and_emit`, `set_entry_error`, and the autoplay step — proceed
  only if the entry is still `processing`; otherwise delete the freshly
  written audio/timestamp files and skip events/autoplay. Extract the guard
  decision into a testable pure function if that avoids Tauri `State` in
  tests.
- [x] `src-tauri/src/commands/mod.rs` `cancel_synthesis`: abort the task via
  the registry, set the entry to `pending`, emit `entry_updated`; if the
  entry was in `synthesize_entered`, call `TtsSupervisor::kill_current()`.
- [x] `src-tauri/src/tts/supervisor.rs`: add `kill_current()` — clear the
  current subprocess slot so `kill_on_drop` terminates ttsd; the next
  request respawns via the existing `ensure_respawned` path (with
  `ttsd_restarting` and warmup events).

## Tests

- [x] Guard unit tests: a late completion/failure for a non-`processing`
  entry changes no status and removes the late files.
- [x] `cancel_synthesis` unit tests: missing entry → `not_found`; cancel
  sets `pending` and aborts the registered task.
- [x] Integration test next to `src-tauri/tests/supervisor.rs` (mock ttsd):
  `kill_current` drops the process and the next request transparently
  respawns + succeeds.

## Validation

- [x] `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml` green.
- [x] `nix develop -c just lint` green.
- [x] `nix develop -c pnpm dlx @fission-ai/openspec@1.6.0 validate
  real-synthesis-cancellation --strict` green.
