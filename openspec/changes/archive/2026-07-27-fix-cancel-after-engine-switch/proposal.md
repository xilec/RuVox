# Proposal: Cancel must reach the swapped-out engine's ttsd (#127)

## Summary

A synthesis is running on Silero; the user switches the engine to Piper
mid-synthesis and then cancels the entry. The entry returns to `pending`,
but `EngineSwitcher::kill_current_ttsd()` only reads the *current* engine
slot, where Silero is already gone — the orphaned ttsd keeps burning CPU on
the cancelled synthesis until it finishes (up to the 5-minute timeout).

Keep a weak reference to the most recently built Silero engine in
`EngineSwitcher` so `cancel_synthesis` can also terminate the previous
engine's ttsd after a switch.

## Capabilities

- `ipc-commands` (modified — Synthesis Cancellation Command)

## Non-goals

- No kill-on-switch: swapping the engine does not, by itself, abort an
  in-flight synthesis — only an explicit `cancel_synthesis` does.
- No tracking of more than one swapped-out engine: a double switch
  (Silero → Piper → Silero) while the first synthesis is still in flight
  leaves that first orphan unreachable; this edge is accepted (see
  design.md).

## Approach

`EngineSwitcher` gains a `last_silero: RwLock<Option<Weak<dyn TtsEngine>>>`
slot, populated at construction (when the initial engine is Silero) and on
every Silero build in `apply_config`. `kill_current_ttsd()` kills the
current engine and then, if the weak reference is still alive, the previous
one — a no-op for Piper and for already-dead handles, so the double kill
when both point to the same engine is harmless.
