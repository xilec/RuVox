# ttsd JSON Protocol Specification

## Purpose

Defines the NDJSON protocol between the Rust backend and the long-lived Python
`ttsd` subprocess that wraps the Silero TTS model: transport and framing, the
`warmup` / `synthesize` / `shutdown` request/response schemas, error codes,
startup behavior, request serialization, and the supervisor's auto-restart
policy. Implemented by `src-tauri/src/tts/mod.rs` and
`src-tauri/src/tts/supervisor.rs` on the Rust side, and by `ttsd/ttsd/protocol.py`
and `ttsd/ttsd/main.py` on the Python side.

## Requirements

### Requirement: NDJSON Transport and Framing

Communication SHALL use the subprocess's `stdin`/`stdout` with newline-delimited
JSON: each request is a single UTF-8 JSON object terminated by `\n`, and each
response is a single UTF-8 JSON object terminated by `\n`. ttsd SHALL flush
`stdout` after every response write. The Rust side spawns ttsd with piped
stdin/stdout/stderr and `kill_on_drop` enabled.

#### Scenario: single request/response exchange
- GIVEN a running ttsd subprocess
- WHEN the Rust side writes `{"cmd":"warmup"}\n` to stdin
- THEN exactly one JSON line is read back from stdout in response

#### Scenario: response is flushed immediately
- GIVEN ttsd has produced a response object
- WHEN it writes the line to stdout
- THEN `sys.stdout.flush()` is called so the Rust side never waits on a buffered pipe

### Requirement: Serialized Request Concurrency

Exactly one request SHALL be in flight at a time. The Rust side serializes all
requests through an `mpsc` channel consumed by a single driver task that owns
the subprocess pipes; ttsd reads one line, processes it, writes the response,
then reads the next line. This matches the single-threaded design of the Silero
engine.

#### Scenario: concurrent callers are queued
- GIVEN two synthesis requests submitted concurrently
- WHEN the driver task is busy with the first
- THEN the second request waits in the channel and is written only after the first response has been read

### Requirement: Stderr Log Forwarding

All Python logs SHALL go to stderr only (never stdout, which is reserved for
protocol responses). The Rust side SHALL read stderr asynchronously and forward
each line to `tracing::info!` under the `ttsd` target.

#### Scenario: python log line reaches tracing
- GIVEN ttsd logs a line via its stderr logging handler
- WHEN the Rust driver task reads it
- THEN the line appears in the Rust log with the `ttsd` target prefix

### Requirement: Request Schema

Every request SHALL be a JSON object with a `cmd` discriminator field selecting
one of three commands: `warmup`, `synthesize`, `shutdown`. Unknown or malformed
requests SHALL be answered with a `bad_input` error response (the process keeps
running).

```typescript
type Request = WarmupRequest | SynthesizeRequest | ShutdownRequest;
```

#### Scenario: malformed request gets bad_input
- GIVEN a running ttsd subprocess
- WHEN a line that fails schema validation is written to stdin
- THEN ttsd responds with `{ "ok": false, "error": "bad_input", "message": "..." }` and continues serving further requests

### Requirement: Warmup Command

The `warmup` request (`{ "cmd": "warmup" }`, no additional fields) SHALL load
the Silero model into memory. On success ttsd SHALL respond
`{ "ok": true, "version": "<ttsd version>" }`. On failure ttsd SHALL respond
`{ "ok": false, "error": "model_not_loaded", "message": "<detail>" }` and MUST
keep running so a later `warmup` retry can succeed. Warmup is idempotent.

#### Scenario: successful warmup
- GIVEN a freshly spawned ttsd
- WHEN the Rust side sends `{"cmd":"warmup"}`
- THEN ttsd loads the model and responds with `ok: true` and a `version` string

#### Scenario: failed warmup keeps the process alive
- GIVEN a model load that raises an exception
- WHEN `warmup` fails
- THEN ttsd responds with `model_not_loaded` and accepts subsequent requests, including a retry `warmup`

### Requirement: Synthesize Command

The `synthesize` request SHALL carry the normalized text and synthesis
parameters, and ttsd SHALL write the resulting WAV file to the given path:

```json
{
  "cmd": "synthesize",
  "text": "<normalized text>",
  "speaker": "xenia",
  "sample_rate": 48000,
  "out_wav": "/absolute/path/<uuid>.wav",
  "char_mapping": [ { "norm_start": 0, "norm_end": 1, "orig_start": 0, "orig_end": 3 } ]
}
```

`text`, `speaker`, `sample_rate`, and `out_wav` are required; `char_mapping` is
optional and omitted from the JSON entirely when absent. The parent directory of
`out_wav` is guaranteed to exist.

On success ttsd SHALL respond:

```json
{
  "ok": true,
  "timestamps": [ { "word": "привет", "start": 0.0, "end": 0.5, "original_pos": [0, 6] } ],
  "duration_sec": 0.9
}
```

`timestamps` may be empty for very short text; `duration_sec` is the total WAV
duration in seconds. Before synthesizing, ttsd SHALL reject empty/whitespace
text with `bad_input` and SHALL reject any request while the model is not
loaded with `model_not_loaded`.

#### Scenario: successful synthesis
- GIVEN the model is loaded
- WHEN a valid `synthesize` request arrives
- THEN ttsd writes the WAV file at `out_wav` and responds with word timestamps and `duration_sec`

#### Scenario: synthesis before warmup
- GIVEN the model is not loaded
- WHEN a `synthesize` request arrives
- THEN ttsd responds `{ "ok": false, "error": "model_not_loaded", "message": "Silero model is not loaded; send warmup first" }`

#### Scenario: empty text rejected
- GIVEN the model is loaded
- WHEN a `synthesize` request with empty or whitespace-only `text` arrives
- THEN ttsd responds with `error: "bad_input"` and no file is written

### Requirement: Shutdown Command

The `shutdown` request (`{ "cmd": "shutdown" }`) SHALL make ttsd respond
`{ "ok": true }` and then exit with code 0. On the Rust side, after sending
`shutdown` the driver closes stdin, waits up to 5 seconds for the process to
exit, and force-kills it (SIGKILL via `start_kill`) if it does not.

#### Scenario: graceful shutdown
- GIVEN a running ttsd
- WHEN the Rust side sends `{"cmd":"shutdown"}`
- THEN ttsd writes `{"ok":true}` and terminates with exit code 0

#### Scenario: unresponsive shutdown is force-killed
- GIVEN a ttsd that does not exit after the shutdown response
- WHEN 5 seconds elapse
- THEN the Rust driver task force-kills the process and reaps it

### Requirement: Error Response Schema

Every response SHALL be a JSON object with a boolean `ok` field. Failure
responses SHALL carry `error` (machine-readable code) and `message`
(human-readable detail):

```typescript
interface ErrorResponse {
  ok: false;
  error: "model_not_loaded" | "synthesis_failed" | "bad_input" | "internal";
  message: string;
}
```

Code semantics: `model_not_loaded` — model not loaded, warmup required;
`synthesis_failed` — Silero raised during synthesis; `bad_input` — invalid
request (empty text, bad parameters, schema violation); `internal` — unexpected
Python exception.

#### Scenario: error responses are typed
- GIVEN any failing request
- WHEN ttsd writes the response
- THEN it contains `ok: false`, one of the four error codes, and a non-empty `message`

### Requirement: Char Mapping Schema

The optional `char_mapping` array SHALL consist of entries mapping normalized
text offsets back to original (pre-normalization) text offsets:

```typescript
interface CharMappingEntry {
  norm_start: number;
  norm_end: number;
  orig_start: number;
  orig_end: number;
}
```

When `char_mapping` is present, ttsd SHALL use it to compute `original_pos` of
each returned `WordTimestamp` in original-text coordinates; when absent,
`original_pos` values are offsets within the normalized text.

#### Scenario: timestamps mapped to original text
- GIVEN a synthesize request with `char_mapping` for transliterated English
- WHEN synthesis completes
- THEN each timestamp's `original_pos` addresses character offsets in the original text for UI highlighting

### Requirement: Word Timestamp Schema

Each word timestamp SHALL have the shape:

```typescript
interface WordTimestamp {
  word: string;                   // normalized word as spoken
  start: number;                  // seconds, relative to audio start
  end: number;
  original_pos: [number, number]; // [start, end] char offsets
}
```

#### Scenario: timestamp fields round-trip
- GIVEN a successful synthesize response
- WHEN the Rust side deserializes `timestamps`
- THEN each element provides `word`, `start`, `end`, and a two-element `original_pos` tuple

### Requirement: Request Timeouts

The Rust side SHALL enforce a 5-minute timeout on `synthesize` requests
(`TtsError::Timeout` on expiry) and a 5-second timeout waiting for the
`shutdown` response.

#### Scenario: hung synthesis times out
- GIVEN a ttsd that never responds to a synthesize request
- WHEN 5 minutes elapse
- THEN the call fails with a timeout error and the entry is marked as failed

### Requirement: Startup Model Lifecycle

At application startup the Rust side SHALL spawn ttsd, run `warmup` in a
background task, and emit the frontend lifecycle events `model_loading` →
`model_loaded` on success or `model_loading` → `model_error { message }` on
failure. Synthesis requests submitted before a successful load fail with
`model_not_loaded` until the model becomes ready.

#### Scenario: startup warmup events
- GIVEN the application just launched
- WHEN the initial warmup completes
- THEN the frontend has observed `model_loading` followed by `model_loaded` (or `model_error` with the failure message)

### Requirement: Auto-Restart on Subprocess Death

The `TtsSupervisor` SHALL detect a dead ttsd when a request fails with
`TtsError::Died` (only `Died` triggers respawn — protocol errors and timeouts
propagate as-is). On detection it SHALL:

1. Log a warning and emit `ttsd_restarting` (`{}`).
2. Make the respawn single-flight (concurrent callers share one respawn).
3. Try to spawn a fresh ttsd up to 3 times with backoff delays of 1 s, 3 s, 5 s.
4. After a successful respawn, run `warmup` in the background re-emitting
   `model_loading` → `model_loaded` / `model_error`, and retry the failed
   request against the new handle.
5. After all attempts fail, emit `tts_fatal { message }` and surface the spawn
   error to the caller; the next request SHALL trigger a fresh respawn attempt
   so the system can still recover later.

An in-flight request sent to the crashed process fails; pending entries go to
the `error` state via the normal command-error path.

#### Scenario: transparent respawn and retry
- GIVEN a ttsd that crashed mid-session
- WHEN the next request hits `TtsError::Died`
- THEN `ttsd_restarting` is emitted, a new process is spawned within the backoff schedule, warmup replays the model lifecycle events, and the request is retried against the new process

#### Scenario: respawn exhausted
- GIVEN a supervisor whose spawn attempts all fail
- WHEN the third attempt fails
- THEN `tts_fatal` is emitted with the error message and the caller receives the spawn error

#### Scenario: protocol errors do not trigger respawn
- GIVEN a live ttsd that responds with an error (e.g. `bad_input`)
- WHEN the request completes with that error
- THEN no respawn is attempted and no `ttsd_restarting` event is emitted
