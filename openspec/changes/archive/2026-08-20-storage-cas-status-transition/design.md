## Context

`StorageService` guards its `HashMap<EntryId, TextEntry>` behind a
`parking_lot::RwLock`. The three status-transition helpers in
`src-tauri/src/commands/mod.rs` currently interleave two acquisitions of that
lock: a `get_entry` read (clone) to decide, then an `update_entry` write that
persists a *separate* clone. Any other thread that mutates the same entry in
between wins the write-order race and its result is then overwritten by the
stale clone. See proposal.md (Why) for the concrete corruption this yields.

The fix is a storage-level compare-and-set so the predicate-check and the
mutation execute under a single write-lock hold. No schema, IPC, or
state-machine change is involved.

## Goals / Non-Goals

**Goals:**
- One method that atomically implements "if status matches, then mutate".
- Re-express `cancel_entry` and the two completion guards through it without
  changing their observable outcomes.

**Non-Goals:**
- Removing or changing `update_entry` (still used elsewhere, e.g. migration,
  `delete_audio`).
- Touching the candidate-file deletion path in `apply_error_if_current`.

## Decisions

**1. Signature: `update_entry_if(id, predicate, mutate) -> bool`.**
Chosen over returning `Result<bool, StorageError>` to match the issue's
proposed shape and keep all three callers uniform. The persistence step is
best-effort and swallows `save_history` errors — the in-memory `HashMap` is the
source of truth (as with `update_entry`), and a failed atomic rename is not a
reason to roll back an already-applied in-memory mutation. `true` means "entry
existed and predicate matched → mutation applied"; `false` means "absent or
predicate rejected → nothing written".

**2. Persist *after* dropping the write guard.**
`save_history` re-acquires a `read` lock on the same `RwLock`. `parking_lot`'s
`RwLock` is not reentrant: holding the write guard while asking for a read guard
would deadlock. So `update_entry_if` drops the guard, then persists. The
in-memory mutation is already committed before the drop, so the saved snapshot
reflects it; the only residual window is between drop and persist, where another
writer may run — that is benign (last-writer-wins on disk, in-memory stays
consistent), and it does not reintroduce the status-transition race.

**3. Rejected alternative — upgradeable `RwLock`.**
Switching to `RwLockUpgradableReadGuard` would avoid the drop, but it only
helps the read→write direction and complicates every other caller of the map.
The drop-then-persist form is simpler and equally correct for this use case.

**4. Rejected alternative — keep helpers as-is, add a per-entry `Mutex`.**
Wrapping each `TextEntry` in its own mutex would serialize per-entry transitions
but spreads locking state across the value type and forces a larger refactor.
The CAS method keeps the locking in one place (`StorageService`).

## Risks / Trade-offs

- [Best-effort persist] → A disk-full `save_history` failure after an applied
  mutation is no longer surfaced as a command error (only `cancel_entry`
  previously propagated it). Mitigation: such failures are IO-panic-class and
  were already swallowed by the ready/error guards; acceptable for a
  tech-debt-level race fix.
- [Late-file deletion ordering] → Unchanged from today: if a `require_processing`
  failure arrives for an entry that just went `ready`, the candidate-file
  cleanup still runs. Out of scope (non-goal); pre-existing.
