# Proposal: Harden cancel_entry against non-processing entries

## Why

`cancel_entry` (`src-tauri/src/commands/mod.rs`) flips the entry status to
`pending` unconditionally. The queue-lifecycle spec sanctions only the
`processing → pending` transition. If `cancel_synthesis` is invoked for an
entry that is no longer `processing` (a race: synthesis finished while the
UI context menu was open — the frontend side was fixed in #174, but the
command remains callable over IPC), a `ready`/`error`/`playing` entry
silently regresses to `pending`, orphaning its audio from the state
machine (playback requires `ready`; the entry would be re-synthesized on
the next trigger).

## What changes

- `cancel_entry` becomes status-aware: for a `ready`, `playing`, or
  `error` entry it fails with `synthesis_error` and changes nothing —
  mirroring the existing error style of `play_entry` ("entry is not
  ready" → `playback_error`). `pending` stays allowed: cancellation is
  idempotent for a queued/idle entry (existing #129 semantics), and a
  just-added entry briefly sits in `pending` with its synthesis task
  already registered — cancelling must still abort it.
- The `processing` path is unchanged: abort the task, remove registry
  keys, flip to `pending`.
- Spec: the Synthesis Cancellation Command requirement in `ipc-commands`
  gains the guard clause and a scenario per non-processing status.

## Non-goals

- No UI changes (the menu already disables the item outside `processing`
  since #174).
- No changes to the stale-completion guard or ttsd kill logic.

Issue: #176
