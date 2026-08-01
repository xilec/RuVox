# Proposal: SIGTERM before SIGKILL in ttsd shutdown escalation (#92)

## Summary

When ttsd does not exit within 5 s of a `shutdown` request, the driver task
escalates straight to SIGKILL (`start_kill`). SIGKILL gives the Python
process no chance to run cleanup (flush logs, release resources, `atexit`
handlers). Insert the conventional middle step: SIGTERM, a short grace
period, then SIGKILL only if the process is still alive.

## Capabilities

- `ttsd-protocol` (modified)

## Non-goals

- The `kill_now()` path used by `cancel_synthesis` keeps its immediate
  SIGKILL — cancellation is an explicit, latency-sensitive user action.
- No changes to the 5 s clean-exit window, the shutdown request schema, or
  ttsd itself.

## Approach

In the shutdown timeout branch of `driver_task`
(`src-tauri/src/tts/mod.rs:411-415`): `libc::kill(pid, SIGTERM)` (libc is
already a dependency), wait up to 2 s, then SIGKILL if still alive.
