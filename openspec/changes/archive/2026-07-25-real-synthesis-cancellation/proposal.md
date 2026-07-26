# Proposal: Real synthesis cancellation (#88)

## Summary

Today `cancel_synthesis` is a cancellation in name only: it flips the entry
back to `pending` while the in-flight ttsd request runs to completion, and
the completion handler then silently resurrects the entry to `ready` with
fresh audio. For long Silero syntheses this wastes tens of seconds of CPU
and produces exactly the result the user asked to discard.

Make cancellation real, in three layers:

1. **Stale-completion guard.** The completion and error paths
   (`mark_ready_and_emit`, `set_entry_error`, autoplay) only apply their
   result when the entry is still `processing`. A completion arriving for an
   entry that is no longer `processing` is discarded together with its
   freshly written audio files.
2. **Abort queued work.** `AppState` keeps a registry of
   `entry_id → tokio::task::AbortHandle` for spawned synthesis tasks.
   `cancel_synthesis` aborts the task, so entries that never reached ttsd
   stop immediately without touching the subprocess.
3. **Kill in-flight work.** An entry that already entered the TTS stage is
   tracked; cancelling it additionally drops the current ttsd subprocess
   (`kill_on_drop`), freeing the CPU immediately. The supervisor's existing
   auto-restart (3 attempts, 1/3/5 s backoff) brings a fresh instance up,
   and other queued/in-flight requests are retried by the existing
   `with_retry` logic.

## Capabilities

- `ipc-commands` (modified) — the `cancel_synthesis` command contract
- `queue-lifecycle` (modified) — status transitions on cancel, stale
  completion handling
- `ttsd-protocol` (modified) — restart may also be triggered by cancellation

## Non-goals

- No ttsd protocol changes: no `cancel` request, no reader thread, no
  request ids (the cooperative-cancel redesign was evaluated and rejected —
  see design.md).
- Piper in-process synthesis is not interruptible mid-call; for Piper the
  abort + stale guard applies (the blocking call runs out, its result is
  discarded). True mid-inference Piper cancellation is out of scope.
- No changes to the retry/backoff policy itself.

## Approach

See design.md for the rejected alternatives (protocol-level cooperative
cancel; naive always-kill) and the exact touch points.
