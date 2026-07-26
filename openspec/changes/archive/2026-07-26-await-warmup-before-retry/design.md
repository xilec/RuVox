# Design: Await warmup before retry

## Root cause

`ensure_respawned` (`src-tauri/src/tts/supervisor.rs:120-184`) installs the
fresh `TtsSubprocess` and kicks off `spawn_warmup` as a fire-and-forget
background task (events `model_loading` → `model_loaded` / `model_error`).
`with_retry` (supervisor.rs:90-114) then immediately loops and retries the
failed request against the new handle. Real Silero ttsd answers `synthesize`
with `model_not_loaded` until warmup finishes, so the retry fails. The mock
ttsd in `tests/supervisor.rs` needs no model, which is why the suite never
caught this. The stale comment in `spawn_warmup` ("the next synthesize will
retrigger model load on the Python side") is factually wrong — ttsd does not
auto-load.

## Chosen design: per-generation readiness signal in the handle slot

- The supervisor slot becomes `RwLock<Option<LiveHandle>>` where
  `LiveHandle { proc: Arc<TtsSubprocess>, ready: watch::Receiver<WarmupState> }`
  and `WarmupState` is `WarmingUp | Ready | Failed`.
- `ensure_respawned`, after a successful spawn, creates the watch channel in
  `WarmingUp`, spawns the warmup task (unchanged event emissions; it flips
  the state to `Ready` or `Failed` at the end), and installs the
  `LiveHandle`.
- `with_retry`, after obtaining the current `LiveHandle` (existing or
  freshly respawned), awaits `ready` while it is `WarmingUp` before running
  the operation. The receiver is per-generation, so a waiter never confuses
  an old generation's readiness with the current one.
- After `Failed`, operations proceed anyway and surface ttsd's own error
  (e.g. `model_not_loaded`) — no infinite wait, honest error propagation.
- Initial spawn (`TtsSupervisor::spawn`) installs the handle with state
  `Ready`, preserving today's startup semantics exactly (the app's explicit
  startup warmup is unaffected).

This covers all callers, not just the retried request: a queued entry whose
request hits `Died` from the killed process, and a brand-new request
arriving while the fresh process is still warming up, both wait for the same
generation's readiness instead of failing.

## Rejected alternatives

- **Retry loop on `model_not_loaded` in `with_retry`** — turns a state
  problem into polling: needs a retry budget/delay policy, blurs the line
  between a transient not-ready and a genuine model load failure, and still
  emits spurious errors for brand-new requests. Waiting on a readiness
  signal is deterministic.
- **Global model-ready gate at the command layer** — pushes TTS-lifecycle
  knowledge up into `commands/`, duplicates what the supervisor already
  tracks, and does not help non-command callers of the supervisor.
- **Await warmup inside `ensure_respawned`** — would hold the single-flight
  `respawn_lock` across the whole model load (seconds), serializing
  unrelated requests behind the lock and stalling the `tts_fatal` path;
  readiness must be observable without the lock.

## Touch points

- `src-tauri/src/tts/supervisor.rs` — `LiveHandle` + `WarmupState`, slot
  type, `ensure_respawned` (install + spawn warmup with state flip),
  `with_retry` (await readiness), `spawn_warmup` (state broadcast; fix the
  stale comment), `kill_current` unchanged in behavior.
- No changes to `TtsSubprocess`, the driver, Python, or the event payloads.

## Tests

- Integration (`tests/supervisor.rs`): extend the sleepy-mock family with a
  mock that reports `model_not_loaded` until its `warmup` is called — assert
  that a request issued right after a kill/crash succeeds without an
  explicit warmup call from the client (i.e. the retry waited).
- Keep the existing respawn/second-chance/fatal tests green; add a unit
  test that operations are not attempted while `WarmingUp` (e.g. via the
  recording emitter ordering or a counting mock).
