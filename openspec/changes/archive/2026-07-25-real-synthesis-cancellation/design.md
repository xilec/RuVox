# Design: Real synthesis cancellation

## Current mechanics (why cancel is a lie)

Each entry synthesis runs as its own tokio task (`spawn_synthesis`,
`src-tauri/src/commands/mod.rs:368-430`): normalize → `mark_processing` →
`synthesize_audio` → finalize (WAV→Opus, timestamps) →
`mark_ready_and_emit` → optional autoplay. Requests to ttsd are serialized
through an mpsc channel (capacity 1) consumed by a single `driver_task`
(`src-tauri/src/tts/mod.rs:308-352`); the implicit queue is the set of
tasks waiting on `send()` or on their oneshot reply. `cancel_synthesis`
(mod.rs:660-664) only writes status `pending`; the task keeps running, and
`mark_ready_and_emit` (mod.rs:328-350) applies the late result without
checking the current status — the cancelled entry silently becomes `ready`
again.

## Chosen design: abort + stale guard + targeted kill

### Layer 1 — stale-completion guard (the issue's minimum)

`mark_ready_and_emit`, `set_entry_error`, and the autoplay step re-read the
entry's status and proceed only if it is still `processing`. Otherwise the
late result is dropped: any audio/timestamp files just written for the
entry are deleted, no `entry_updated`→`ready` event is emitted, no
autoplay. This also hardens `regenerate_entry` races, not just cancel.

### Layer 2 — abort registry

`AppState` gains `synthesis_tasks: Mutex<HashMap<EntryId, AbortHandle>>`,
populated in `spawn_synthesis` and cleared when the task finishes (any
outcome). `cancel_synthesis` aborts the task and sets the entry back to
`pending`. Tasks aborted while queued (before their request reaches ttsd)
stop immediately with zero subprocess impact. Aborting a task whose request
is already written to ttsd's stdin leaves an orphaned response in the pipe;
that is protocol-safe: the driver reads exactly one line per request and
the dead oneshot receiver is already handled (`req.reply.send(result)`
ignores the error, `src-tauri/src/tts/mod.rs:331`).

### Layer 3 — kill the subprocess when the cancelled entry reached the TTS stage

`AppState` also tracks which entries have entered the `tts.synthesize()`
await (`synthesize_entered: Mutex<HashSet<EntryId>>`, set just before the
call, cleared after). When `cancel_synthesis` aborts such an entry, it
additionally calls a new `TtsSupervisor::kill_current()`: the current
`Arc<TtsSubprocess>` slot is cleared, `kill_on_drop` terminates the
process, and the next request triggers the existing `ensure_respawned`
(BACKOFFS = 1s/3s/5s, up to 3 attempts, `supervisor.rs:41-45`) with its
background warmup and `model_loading`/`model_loaded` events. Requests from
*other* entries that were in flight or queued are retried transparently by
the existing `with_retry` loop (`supervisor.rs:95-114`).

Known collateral, accepted: if the cancelled entry had entered
`synthesize()` but was still waiting for the channel slot while *another*
entry's request was in flight, the kill also interrupts that other request;
it is then retried (restart + warmup + re-synthesis). Cancellation is a
rare, explicit user action, and the issue explicitly accepts this class of
collateral for the simple option.

## Rejected alternatives

- **Cooperative protocol cancel (`cmd:"cancel"`)** — requires a reader
  thread in ttsd (its main loop is a blocking `for raw_line in sys.stdin`,
  `ttsd/ttsd/main.py:106`), chunk-level cancel checks in `silero.py`, a
  writer/reader split in `driver_task`, and request ids for out-of-order
  replies. That is a redesign of the strict request-response protocol, and
  it contradicts the `ttsd-protocol` requirement "Serialized Request
  Concurrency" (exactly one request in flight). The only gain over the
  hybrid is avoiding the restart of a *different* entry's in-flight request
  — not worth the volume and risk.
- **Naive always-kill** (kill ttsd on every cancel, no abort registry) —
  restarts the model even when the cancelled entry never reached ttsd
  (warmup costs seconds); the abort registry avoids that in the common
  queued case.
- **Guard only** (the issue's stated minimum) — stops the silent `ready`
  resurrection but leaves a cancelled Silero synthesis burning CPU for tens
  of seconds, which is the core complaint of #88.

## Touch points

- `src-tauri/src/commands/mod.rs` — guards in `mark_ready_and_emit` /
  `set_entry_error` / autoplay (with file cleanup); registry population in
  `spawn_synthesis`; new `cancel_synthesis` logic (abort + conditional
  kill); unregister on task completion.
- `src-tauri/src/state.rs` — `synthesis_tasks` and `synthesize_entered`
  fields.
- `src-tauri/src/tts/mod.rs` — active kill support: `TtsSubprocess::kill_now()`
  signals the driver via a watch channel; the driver (on `tokio::select!`)
  SIGKILLs the child even mid-request and answers the in-flight request with
  `TtsError::Died`. (Passive slot clearing is not sufficient: the driver owns
  the `Child` and blocks awaiting the response, so `kill_on_drop` would only
  fire after the synthesis runs out — exactly the CPU burn #88 removes.)
- `src-tauri/src/tts/supervisor.rs` — new `kill_current()` forwarding to
  `kill_now()`; the respawn itself goes through the existing
  `ensure_respawned` path (single-flight, backoff, `ttsd_restarting`,
  warmup).
- No Python / protocol changes.

## Tests

- Unit/integration: stale-guard behavior (completion after cancel does not
  flip status, files removed) — extract the guard decision into a pure
  function over entry status if that keeps it testable without Tauri
  `State`.
- `tests/supervisor.rs`-style integration with the existing mock ttsd:
  `kill_current` drops the process, the next request respawns it and
  succeeds (retry path intact).
- ttsd pytest: untouched.
