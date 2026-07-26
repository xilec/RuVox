# Delta: ttsd-protocol

## MODIFIED Requirements

### Requirement: Auto-Restart on Subprocess Death

The `TtsSupervisor` SHALL detect a dead ttsd when a request fails with
`TtsError::Died` (only `Died` triggers respawn — protocol errors and timeouts
propagate as-is) or when the subprocess was explicitly terminated by
`cancel_synthesis` for an in-flight entry (see the Synthesis Cancellation
Command requirement in `ipc-commands`). On detection it SHALL:

1. Log a warning and emit `ttsd_restarting` (`{}`).
2. Make the respawn single-flight (concurrent callers share one respawn).
3. Try to spawn a fresh ttsd up to 3 times with backoff delays of 1 s, 3 s, 5 s.
4. After a successful respawn, run `warmup` in the background re-emitting
   `model_loading` → `model_loaded` / `model_error`, and retry the failed
   request against the new handle.
5. After all attempts fail, emit `tts_fatal { message }` and surface the spawn
   error to the caller; the next request SHALL trigger a fresh respawn attempt
   so the system can still recover later.

An in-flight request sent to the crashed or terminated process fails; after
an explicit kill for cancellation, in-flight and queued requests belonging
to OTHER entries SHALL be retried transparently via the existing retry loop,
while the cancelled entry's own request is not retried (its task is
aborted). Pending entries whose requests ultimately fail go to the `error`
state via the normal command-error path.

#### Scenario: transparent respawn and retry

- GIVEN a ttsd that crashed mid-session
- WHEN the next request hits `TtsError::Died`
- THEN `ttsd_restarting` is emitted, a new process is spawned within the
  backoff schedule, warmup replays the model lifecycle events, and the
  request is retried against the new process

#### Scenario: respawn after cancellation kill

- GIVEN a ttsd terminated by `cancel_synthesis` for an in-flight entry
- WHEN the next request arrives
- THEN a fresh ttsd is spawned per the same backoff/warmup procedure and
  other entries' requests proceed against the new process

#### Scenario: respawn exhausted

- GIVEN a supervisor whose spawn attempts all fail
- WHEN the third attempt fails
- THEN `tts_fatal` is emitted with the error message and the caller receives
  the spawn error

#### Scenario: protocol errors do not trigger respawn

- GIVEN a live ttsd that responds with an error (e.g. `bad_input`)
- WHEN the request completes with that error
- THEN no respawn is attempted and no `ttsd_restarting` event is emitted
