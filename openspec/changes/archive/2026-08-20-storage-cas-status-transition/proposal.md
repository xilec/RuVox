## Why

`cancel_entry`, `apply_ready_if_current`, and `apply_error_if_current` (all in
`src-tauri/src/commands/mod.rs`) each perform a **read-decide-write across two
separate `StorageService` lock acquisitions**: they clone the entry via
`get_entry`, decide based on the status, then persist a mutated clone via
`update_entry`. If a synthesis completion lands in the microseconds between the
read and the write, the stale clone is persisted — regressing a `ready`/`error`
entry back to `pending` with no `audio_path`/`timestamps_path` while the on-disk
audio still exists. That is the orphaned-audio corruption from #176 at the
residual µs scale (the seconds-scale UI guard added in the #176 fix closed the
observable gap; this is the last race the guard does not cover).

The window is tiny and currently unobserved in practice, so this is recorded
tech-debt (issue #179) rather than an incident. Filing it as a change keeps the
accepted residual risk an explicit, reviewable decision.

## What Changes

- Add `StorageService::update_entry_if(id, predicate, mutate)`: acquire the write
  lock once, evaluate `predicate(&entry)`, and — only if it returns `true` —
  apply `mutate(&mut entry)` and persist, all under the same lock
  (compare-and-set). Returns `true` when the entry existed and the predicate
  matched (the mutation was applied); `false` otherwise, writing nothing.
- Refactor `cancel_entry` to drive its `processing | pending → pending`
  transition through `update_entry_if`.
- Refactor `apply_ready_if_current` and `apply_error_if_current` to build their
  status predicate (`completion_is_current`, i.e. status `== processing`) and
  pass the mutation as a closure, so the stale-completion guard is atomic.
- Add a unit test pinning `update_entry_if` semantics (predicate-rejected no-op,
  predicate-accepted apply, absent id is a no-op).

## Non-goals

- No change to the status state machine or the set of permitted transitions.
- No change to observable behavior on the non-racy path; the existing command
  tests must stay green.
- No fix to the unrelated candidate-file deletion in `apply_error_if_current`
  (a separate, pre-existing concern out of scope here).

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `storage`: add a requirement that the storage service provides an atomic
  conditional (compare-and-set) entry update so status transitions cannot be
  raced by a concurrent read-decide-write.

## Impact

- `src-tauri/src/storage/service.rs`: new `update_entry_if` method; `update_entry`
  callers there are unaffected.
- `src-tauri/src/commands/mod.rs`: internal refactor of three helpers; their
  public `tauri::command` signatures and the `entry_updated` emit contract are
  unchanged.
- No new dependencies, no cross-process / IPC / schema changes.
