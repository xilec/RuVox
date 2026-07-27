# Design: Reach the swapped-out engine on cancel

## Context

`EngineSwitcher::apply_config` swaps `slot.engine` and drops the old
`Arc<dyn TtsEngine>`. An in-flight `synthesize` future still holds its own
`Arc` clone, so the old `TtsSupervisor` (and its ttsd child) stays alive
until the request settles. When no reference remains, dropping the last
`Arc<TtsSubprocess>` closes the driver channel; the driver task exits and
`kill_on_drop` SIGKILLs the child — so an *idle* swapped-out engine cleans
itself up and only the *in-flight* case leaks CPU.

## Decision

Store `last_silero: RwLock<Option<Weak<dyn TtsEngine>>>` in
`EngineSwitcher`:

- Set in `new()` when `initial_kind == Silero` (the startup-built engine is
  otherwise unreachable after the first switch).
- Reset in `apply_config` each time a Silero engine is built
  (`Arc::downgrade`).
- `kill_current_ttsd()` kills the current engine, then upgrades the weak
  reference and kills that engine too when it is still alive.

`Weak` (not `Arc`) is load-bearing: a strong reference would keep the
swapped-out supervisor — and its idle ttsd child, via the driver's
`kill_on_drop` semantics — alive forever, trading a CPU leak for a permanent
process leak. When the in-flight synthesis finishes and drops its `Arc`,
the weak reference dies on its own.

## Rejected alternatives

- **Kill the old engine on swap (`apply_config` calls
  `old_engine.kill_current()`).** Aborts work the user never asked to
  cancel, and is actively harmful: the killed handle fails in-flight
  requests with `TtsError::Died`, and the orphaned supervisor's retry loop
  would *respawn* a fresh ttsd and re-run the synthesis — more waste, not
  less.
- **Strong `previous_engine` slot.** Pins the old supervisor and its ttsd
  child alive for the rest of the session even when nothing is running
  (see above).
- **Track every swapped-out engine (Vec of weak refs).** Covers the
  double-switch-mid-synthesis edge (Silero → Piper → Silero while the
  first synthesis still runs), but adds bookkeeping for a scenario that
  requires two engine switches inside one synthesis window. One slot
  covers the reported case; the edge is documented as a non-goal.

## Testing

Unit test in `switcher.rs`: a fake `TtsEngine` reporting
`EngineKind::Silero` with a kill counter is installed as the initial
engine; after `apply_config("piper", …)` swaps it out,
`kill_current_ttsd()` must reach the fake (counter increments) while the
caller still holds its `Arc` — mirroring the in-flight synthesis holding
the engine alive.
