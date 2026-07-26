# Proposal: Await warmup before retrying after a ttsd restart (#128)

## Summary

When ttsd dies mid-session (crash, or an explicit kill from
`cancel_synthesis`), the supervisor respawns it and `with_retry` retries the
failed request immediately — while the post-respawn warmup (model load) is
still running in the background. The real Silero ttsd rejects `synthesize`
with `model_not_loaded` until warmup completes, so the "transparent retry"
fails for the real engine and the entry lands in `error` despite a
successful recovery.

Make requests wait for the post-respawn warmup before they are sent to the
fresh process: the supervisor's handle slot carries a per-generation
readiness signal fed by the background warmup task, and `with_retry` awaits
it (state != warming-up) before issuing an operation against that handle.

## Capabilities

- `ttsd-protocol` (modified) — Auto-Restart on Subprocess Death

## Non-goals

- No change to startup semantics: before the initial warmup completes, an
  early `synthesize` keeps today's behavior (no waiting at supervisor
  level).
- No ttsd protocol or Python changes.
- No change to the backoff schedule, single-flight respawn, or event names
  (`ttsd_restarting`, `model_loading`, `model_loaded`, `model_error`,
  `tts_fatal`).

## Approach

See design.md: per-generation readiness receiver stored alongside the
subprocess handle; rejected alternatives (retry-loop on `model_not_loaded`,
global model-ready gate) recorded there.
