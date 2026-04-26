# IPC Contract

Authoritative specification of all three communication layers in RuVox.

---

## Parameter casing

Tauri 2 принимает параметры invoke на JS-стороне в **camelCase**, даже если Rust-handler объявлен в snake_case. Документация ниже показывает Rust-имена параметров (snake_case) для соответствия сигнатурам команд; реальный JS-код вызывает `invoke('seek_to', { positionSec: ... })`, не `{ position_sec: ... }`. Типизированные обёртки в `src/lib/tauri.ts` делают преобразование автоматически.

---

## Table of Contents

1. [Shared Types](#shared-types)
2. [Layer 1: Tauri Commands (Frontend → Backend)](#layer-1-tauri-commands)
3. [Layer 2: Tauri Events (Backend → Frontend)](#layer-2-tauri-events)
4. [Layer 3: ttsd JSON Protocol (Backend → Python Subprocess)](#layer-3-ttsd-json-protocol)
5. [Example Exchanges](#example-exchanges)

---

## Shared Types

All types are described in TypeScript-like syntax. The same field names and enum values are used in JSON serialization across all layers.

```typescript
// Unique identifier for a text entry (UUID v4 string)
type EntryId = string;

// Status of a text entry through the TTS pipeline
type EntryStatus =
  | "pending"     // Waiting for TTS synthesis to start
  | "processing"  // TTS synthesis is in progress
  | "ready"       // Audio is synthesized and ready for playback
  | "playing"     // Currently being played back
  | "error";      // Synthesis failed

// A text entry in the TTS queue.
// Field names map 1:1 to JSON keys in history.json (legacy-compatible).
interface TextEntry {
  id: EntryId;                     // UUID v4
  original_text: string;           // Raw text from clipboard
  normalized_text: string | null;  // Text after pipeline normalization (what Silero reads)
  status: EntryStatus;
  created_at: string;              // ISO 8601 datetime, e.g. "2024-01-15T10:30:00.123456"
  audio_generated_at: string | null; // ISO 8601 datetime when WAV was created
  audio_path: string | null;       // Filename relative to ~/.cache/ruvox/audio/, e.g. "<uuid>.wav"
  timestamps_path: string | null;  // Filename relative to ~/.cache/ruvox/audio/, e.g. "<uuid>.timestamps.json"
  duration_sec: number | null;     // Total audio duration in seconds
  was_regenerated: boolean;        // True if audio was manually re-synthesized at least once
  error_message: string | null;    // Human-readable error if status === "error"
}

// Word-level timestamp, mapping a spoken word to a time range and
// a character range in the original (pre-normalization) text.
interface WordTimestamp {
  word: string;           // Normalized word as spoken by Silero
  start: number;          // Start time in seconds (relative to entry audio start)
  end: number;            // End time in seconds
  original_pos: [number, number]; // [start, end] char offsets in original_text (for highlighting)
}

// Full application configuration persisted to ~/.cache/ruvox/config.json.
interface UIConfig {
  speaker: string;               // Silero speaker name, e.g. "xenia", "aidar"
  sample_rate: number;           // Audio sample rate: 8000 | 24000 | 48000
  speech_rate: number;           // Playback speed multiplier (0.5–2.0), default 1.0
  notify_on_ready: boolean;      // Show system notification when synthesis completes
  notify_on_error: boolean;      // Show system notification on synthesis error
  text_format: string;           // Default viewer format: "plain" | "markdown" | "html"
  history_days: number;          // Days to keep entries in history (0 = forever)
  audio_max_files: number;       // Maximum number of WAV files to keep in cache
  audio_regenerated_hours: number; // Hours to keep manually regenerated audio
  max_cache_size_mb: number;     // Soft limit on total audio cache size in MB
  auto_cleanup_days: number;     // Auto-delete entries older than N days (0 = disabled)
  code_block_mode: string;       // How to handle Markdown code blocks: "skip" | "read"
  read_operators: boolean;       // Whether to speak mathematical/code operators
  theme: string;                 // Color scheme: "light" | "dark" | "auto"
  player_hotkeys: Record<string, string>; // Local player hotkeys map, e.g. {"play_pause": "Space"}
  window_geometry: [number, number, number, number] | null; // [x, y, width, height]
  preview_dialog_enabled: boolean; // FF 1.1: show preview dialog before synthesis
}

// Partial UIConfig — only the fields that need to be updated.
// Any omitted field keeps its current value.
type UIConfigPatch = Partial<UIConfig>;
```

---

## Layer 1: Tauri Commands

Tauri commands are invoked from the React frontend via `invoke("command_name", args)`.

On the Rust side each command is annotated `#[tauri::command]` and registered in `tauri::Builder`. Errors are returned as a typed JSON object with a `type` field (see [Error format](#error-format)).

### Error Format

All commands that can fail return a `TauriError` on the `Err` path:

```typescript
interface TauriError {
  type: "not_found" | "storage_error" | "synthesis_error" | "playback_error" | "config_error" | "internal";
  message: string; // Human-readable detail
}
```

On the TypeScript side `invoke()` rejects the promise with this object.

---

### `add_clipboard_entry`

Read text from the system clipboard and add a new entry to the queue. Triggers TTS synthesis immediately. Used **only by the system tray menu** — there is no webview context there, so the Rust side reads the clipboard via `arboard`.

```typescript
invoke("add_clipboard_entry", { play_when_ready: boolean }): Promise<EntryId>
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `play_when_ready` | `boolean` | If `true`, playback starts automatically when synthesis completes |

**Returns:** `EntryId` of the newly created entry.

**Errors:** `storage_error` if writing to history.json fails; `internal` if clipboard is unavailable. **Note:** `arboard` silently fails with `ContentNotAvailable` on KDE Plasma 6 with Wayland — the frontend should prefer `add_text_entry` (see below).

**Side effects:**
- Emits `entry_updated` with `status: "pending"` immediately.
- Emits `entry_updated` with `status: "processing"` when synthesis starts.
- Emits `entry_updated` with `status: "ready"` or `status: "error"` when synthesis finishes.
- If `play_when_ready` is true and synthesis succeeds, emits `playback_started`.

---

### `add_text_entry`

Add a new entry with the given text and trigger TTS synthesis. **Preferred command from the frontend** — frontend reads the clipboard via `tauri-plugin-clipboard-manager` (which works reliably on Wayland/KDE) and passes the resulting text here.

```typescript
invoke("add_text_entry", { text: string, play_when_ready: boolean }): Promise<EntryId>
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `text` | `string` | Raw text to ingest. The pipeline will normalize before synthesis. |
| `play_when_ready` | `boolean` | If `true`, playback starts automatically when synthesis completes |

**Returns:** `EntryId` of the newly created entry.

**Errors:** `storage_error` if writing to history.json fails.

**Side effects:** Same as `add_clipboard_entry` — emits `entry_updated` lifecycle events and `playback_started` when applicable.

---

### `preview_normalize`

Run the normalization pipeline on the given text and return the normalized result **without** persisting an entry. Used by the FF 1.1 preview dialog to show original ↔ normalized side-by-side before the user confirms synthesis.

```typescript
invoke("preview_normalize", { text: string }): Promise<{ normalized: string }>
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `text` | `string` | Raw text to normalize. |

**Returns:** `{ normalized: string }` — pipeline output (string only; char-mapping is discarded).

**Errors:** `internal` if the pipeline task panics.

**Side effects:** None — the entry list and storage are not touched.

See [preview-dialog.md](preview-dialog.md) for full preview flow.

---

### `get_entries`

Return all entries sorted by `created_at` descending (newest first).

```typescript
invoke("get_entries"): Promise<TextEntry[]>
```

**Returns:** Array of `TextEntry`. Empty array if no entries exist.

**Errors:** `storage_error` on read failure.

---

### `get_entry`

Return a single entry by ID.

```typescript
invoke("get_entry", { id: EntryId }): Promise<TextEntry | null>
```

**Returns:** `TextEntry` if found, `null` if not found (not an error).

**Errors:** `storage_error` on read failure.

---

### `delete_entry`

Delete an entry and its associated audio and timestamps files.

```typescript
invoke("delete_entry", { id: EntryId }): Promise<void>
```

**Errors:** `not_found` if no entry with this ID exists; `storage_error` on file deletion failure.

**Note:** If the entry is currently playing, playback is stopped first and `playback_stopped` is emitted.

---

### `delete_audio`

Delete only the audio (and timestamps) files for an entry without removing the entry itself. Entry status is reset to `"pending"`.

```typescript
invoke("delete_audio", { id: EntryId }): Promise<void>
```

**Errors:** `not_found` if no entry with this ID exists; `storage_error` on file deletion failure.

**Side effects:** Emits `entry_updated` with `status: "pending"`.

---

### `cancel_synthesis`

Cancel an in-progress or queued synthesis job. If the entry is currently being synthesized, the job is aborted. Entry status is set to `"pending"`.

```typescript
invoke("cancel_synthesis", { id: EntryId }): Promise<void>
```

**Errors:** `not_found` if no entry with this ID exists.

**Side effects:** Emits `entry_updated` with `status: "pending"`.

---

### `play_entry`

Start playback of a ready entry. If another entry is playing, it is stopped first.

```typescript
invoke("play_entry", { id: EntryId }): Promise<void>
```

**Errors:** `not_found` if entry does not exist; `playback_error` if entry status is not `"ready"`; `playback_error` if the WAV file is missing or unreadable.

**Side effects:** Emits `playback_started`; emits `playback_stopped` for the previously playing entry if any.

---

### `pause_playback`

Pause the currently playing entry. No-op if nothing is playing.

```typescript
invoke("pause_playback"): Promise<void>
```

**Errors:** `playback_error` on libmpv failure.

---

### `resume_playback`

Resume playback from the paused position. No-op if not paused.

```typescript
invoke("resume_playback"): Promise<void>
```

**Errors:** `playback_error` on libmpv failure.

---

### `stop_playback`

Stop playback entirely. Entry status returns to `"ready"`.

```typescript
invoke("stop_playback"): Promise<void>
```

**Errors:** `playback_error` on libmpv failure.

**Side effects:** Emits `playback_stopped`.

---

### `seek_to`

Seek to an absolute position in the current audio.

```typescript
invoke("seek_to", { position_sec: number }): Promise<void>
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `position_sec` | `number` | Target position in seconds. Clamped to `[0, duration_sec]`. |

**Errors:** `playback_error` if nothing is loaded; `playback_error` on libmpv failure.

---

### `set_speed`

Set playback speed (scaletempo2). Persisted to `UIConfig.speech_rate`.

```typescript
invoke("set_speed", { speed: number }): Promise<void>
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `speed` | `number` | Speed multiplier. Valid range: `[0.5, 2.0]`. |

**Errors:** `playback_error` on libmpv failure; `config_error` if value is out of range.

---

### `set_volume`

Set playback volume (0.0–1.0). Not persisted.

```typescript
invoke("set_volume", { volume: number }): Promise<void>
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `volume` | `number` | Volume level. Valid range: `[0.0, 1.0]`. |

**Errors:** `playback_error` on libmpv failure; `config_error` if value is out of range.

---

### `get_config`

Return the current application configuration.

```typescript
invoke("get_config"): Promise<UIConfig>
```

**Errors:** `config_error` if config.json cannot be read (returns defaults instead in most cases).

---

### `update_config`

Merge a partial config patch into the current configuration and persist to disk.

```typescript
invoke("update_config", { patch: UIConfigPatch }): Promise<void>
```

**Errors:** `config_error` if writing to disk fails.

---

### `get_timestamps`

Load and return word timestamps for an entry. Used by the frontend to initialize word highlighting before playback.

```typescript
invoke("get_timestamps", { id: EntryId }): Promise<WordTimestamp[]>
```

**Returns:** Array of `WordTimestamp`. Empty array if entry has no timestamps.

**Errors:** `not_found` if no entry with this ID; `storage_error` if the timestamps file cannot be read.

---

### `clear_cache`

Delete all audio files older than the configured `auto_cleanup_days`, or forcibly delete all audio files if `force` is true.

```typescript
invoke("clear_cache", { force: boolean }): Promise<{ deleted_files: number; freed_bytes: number }>
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `force` | `boolean` | If `true`, delete all audio regardless of age. Entries are kept but reset to `"pending"`. |

**Errors:** `storage_error` on file system failures.

**Side effects:** Emits `entry_updated` for each entry whose audio was deleted.

---

### `get_cache_stats`

Return current cache size information.

```typescript
invoke("get_cache_stats"): Promise<{ total_bytes: number; audio_file_count: number }>
```

---

## Layer 2: Tauri Events

Events are emitted by the Rust backend and received in the frontend via `listen("event_name", handler)`.

All events are emitted on the main Tauri window. Payload is a JSON object.

### `entry_updated`

Emitted whenever a `TextEntry` is created or its fields change (status, audio_path, normalized_text, error_message, etc.).

```typescript
// Payload
{ entry: TextEntry }
```

**When emitted:**
- Immediately after `add_clipboard_entry` (status: `"pending"`).
- When synthesis starts (status: `"processing"`).
- When synthesis completes (status: `"ready"`, audio_path set).
- When synthesis fails (status: `"error"`, error_message set).
- When playback starts/stops (status: `"playing"` / `"ready"`).
- After `delete_audio` (status reset to `"pending"`).
- After `cancel_synthesis`.

---

### `playback_position`

Emitted periodically during playback at approximately 100 ms intervals.

```typescript
// Payload
{ position_sec: number; entry_id: EntryId }
```

---

### `playback_started`

Emitted when playback of an entry begins (including auto-play after synthesis).

```typescript
// Payload
{ entry_id: EntryId }
```

---

### `playback_paused`

Emitted when playback is paused.

```typescript
// Payload
{ entry_id: EntryId; position_sec: number }
```

---

### `playback_stopped`

Emitted when playback ends (either completed naturally, was stopped, or the entry was deleted).

```typescript
// Payload
{}
```

---

### `playback_finished`

Emitted when a track plays to its natural end (position reaches duration).

```typescript
// Payload
{ entry_id: EntryId }
```

---

### `model_loading`

Emitted when ttsd starts loading the Silero model (at warmup or on first use if warmup failed).

```typescript
// Payload
{}
```

---

### `model_loaded`

Emitted when the Silero model is ready. Any synthesis requests queued during loading are now unblocked.

```typescript
// Payload
{}
```

---

### `model_error`

Emitted when the Silero model fails to load. Synthesis is blocked until the next successful load.

```typescript
// Payload
{ message: string }
```

---

### `tts_error`

Emitted when synthesis of a specific entry fails (after `entry_updated` with status `"error"` has already been emitted).

```typescript
// Payload
{ entry_id: EntryId; message: string }
```

---

### `synthesis_progress`

Emitted periodically during multi-chunk synthesis to allow progress indication.

```typescript
// Payload
{ entry_id: EntryId; progress: number } // progress in [0.0, 1.0]
```

---

## Layer 3: ttsd JSON Protocol

### Overview

`ttsd` is a long-lived Python subprocess managing the Silero TTS model. It is spawned by the Tauri Rust backend at startup.

```
Rust backend                    Python ttsd subprocess
    │                                    │
    │──── stdin (NDJSON request) ───────>│
    │<─── stdout (NDJSON response) ──────│
    │         stderr (logs) ─────────────> Rust proxies to tracing::info!("ttsd: ...")
```

**Protocol:**
- **Transport:** `stdin`/`stdout` of the subprocess. One JSON object per line (NDJSON / newline-delimited JSON).
- **Framing:** Each request is a single UTF-8 line terminated by `\n`. Each response is a single UTF-8 line terminated by `\n`.
- **Concurrency:** Exactly one request in flight at a time. The Rust side serializes all requests through an `mpsc` channel. `ttsd` reads one line, processes it, writes the response, then reads the next.
- **Stderr:** All Python logs go to stderr. Rust reads stderr asynchronously and forwards each line to `tracing::info!` with the `ttsd:` prefix.
- **Shutdown:** Rust sends a `shutdown` request and then waits for the subprocess to exit. On timeout (5 s), Rust sends SIGTERM.

---

### Request Schema

Every request is a JSON object with a `cmd` field identifying the command.

```typescript
type Request = WarmupRequest | SynthesizeRequest | ShutdownRequest;
```

#### `warmup`

Load the Silero model into memory. Called once at startup before the user can submit text.

```json
{ "cmd": "warmup" }
```

No additional fields.

**Response on success:**
```json
{ "ok": true }
```

**Response on failure:**
```json
{ "ok": false, "error": "model_not_loaded", "message": "torch.hub.load failed: ..." }
```

---

#### `synthesize`

Synthesize speech from text and write the result as a WAV file at the specified path.

```json
{
  "cmd": "synthesize",
  "text": "<normalized text to speak>",
  "speaker": "xenia",
  "sample_rate": 48000,
  "out_wav": "/home/user/.cache/ruvox/audio/550e8400-e29b-41d4-a716-446655440000.wav",
  "char_mapping": [...]
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `cmd` | `"synthesize"` | yes | Command identifier |
| `text` | `string` | yes | Normalized text (output of Rust pipeline). Must not be empty. |
| `speaker` | `string` | yes | Silero speaker name, e.g. `"xenia"`, `"aidar"` |
| `sample_rate` | `number` | yes | WAV sample rate: `8000`, `24000`, or `48000` |
| `out_wav` | `string` | yes | Absolute filesystem path where ttsd writes the WAV file. Parent directory is guaranteed to exist. |
| `char_mapping` | `array \| null` | no | Optional char mapping from Rust pipeline for precise `original_pos` calculation. When present, ttsd uses it to map normalized text positions back to original text positions. When absent, `original_pos` values are positions within normalized text. |

**`char_mapping` element schema (when provided):**
```typescript
interface CharMappingEntry {
  norm_start: number;   // Start offset in normalized text
  norm_end: number;     // End offset in normalized text
  orig_start: number;   // Corresponding start offset in original text
  orig_end: number;     // Corresponding end offset in original text
}
```

**Response on success:**
```json
{
  "ok": true,
  "timestamps": [
    { "word": "вызови", "start": 0.0,   "end": 0.42, "original_pos": [0, 6] },
    { "word": "функцию", "start": 0.45, "end": 1.10, "original_pos": [7, 14] }
  ],
  "duration_sec": 12.34
}
```

| Field | Type | Description |
|-------|------|-------------|
| `ok` | `true` | |
| `timestamps` | `WordTimestamp[]` | List of word-level timestamps. May be empty if text is very short. |
| `duration_sec` | `number` | Total audio duration in seconds (WAV length / sample_rate). |

**Response on failure:**
```json
{ "ok": false, "error": "<error_code>", "message": "<human-readable detail>" }
```

---

#### `shutdown`

Request graceful shutdown. ttsd flushes any buffered output and exits with code 0.

```json
{ "cmd": "shutdown" }
```

**Response:**
```json
{ "ok": true }
```

After sending the response, the ttsd process exits. The Rust side waits up to 5 seconds for process exit before sending SIGTERM.

---

### Response Schema

Every response is a JSON object. The `ok` field is always present and boolean.

**Success:**
```typescript
interface SuccessResponse {
  ok: true;
  // Command-specific fields follow
}
```

**Failure:**
```typescript
interface ErrorResponse {
  ok: false;
  error: ErrorCode;
  message: string; // Human-readable detail for logging/UI
}

type ErrorCode =
  | "model_not_loaded"  // warmup was not called or failed; synthesize cannot proceed
  | "synthesis_failed"  // Silero raised an exception during apply_tts
  | "bad_input"         // text is empty, sample_rate is unsupported, out_wav path is invalid
  | "internal";         // Unexpected Python exception
```

**Error code semantics:**

| Code | When used | Recovery |
|------|-----------|----------|
| `model_not_loaded` | `synthesize` called before successful `warmup`, or model was unloaded | Rust retries `warmup` and re-queues request |
| `synthesis_failed` | Silero's `apply_tts` raised an exception (bad character, memory) | Mark entry as `error`; user may retry |
| `bad_input` | Empty text, unknown speaker, unsupported sample rate | Mark entry as `error`; no retry |
| `internal` | Any other unhandled Python exception | Mark entry as `error`; log traceback |

---

### ttsd Startup Behavior

1. ttsd reads its first request from stdin.
2. If the first request is `warmup`, it loads the model and responds.
3. If `warmup` succeeds, it enters the main request loop.
4. If `warmup` fails, ttsd responds with an error but continues running — it can accept another `warmup` request.
5. Subsequent `synthesize` requests while the model is not loaded return `model_not_loaded`.

**Rust behavior on startup:**
- Rust spawns ttsd from `on_setup` via `tokio::process::Command`.
- Immediately sends `{"cmd": "warmup"}`.
- Emits `model_loading` event to frontend.
- On success response: emits `model_loaded`.
- On error response: emits `model_error` with the error message.
- Logs stderr from ttsd asynchronously to `tracing::info!("ttsd: {line}")`.

---

### ttsd Auto-restart

If ttsd exits unexpectedly (crash, SIGKILL from OOM killer), Rust detects the process exit and:
1. Logs a `tracing::warn!` message.
2. Spawns a new ttsd process.
3. Sends `warmup` again.
4. Re-emits `model_loading` to frontend.

Any in-flight synthesis request that was sent to the crashed process is marked as failed with `synthesis_failed`.

---

## Example Exchanges

### Example 1: "Read Now" — full synthesis and playback flow

**Scenario:** User presses the "Read Now" button. Text is in clipboard. Model is already loaded.

```
Frontend                    Rust Backend                  ttsd
    │                            │                          │
    │─ invoke("add_clipboard_entry",                        │
    │         {play_when_ready:true}) ──────────────────>  │
    │                            │                          │
    │                            │ Read clipboard: "Вызови  │
    │                            │ getUserData() через API" │
    │                            │                          │
    │                            │ Create TextEntry         │
    │                            │ id = "abc-123"           │
    │                            │ status = "pending"       │
    │                            │                          │
    │<─ entry_updated({entry: {id:"abc-123", status:"pending", ...}})
    │                            │                          │
    │<─ Promise resolves: "abc-123"                         │
    │                            │                          │
    │                            │ Run Rust pipeline:       │
    │                            │ "Вызови гет юзер дата   │
    │                            │  через эй пи ай"         │
    │                            │                          │
    │                            │ entry.status = "processing"
    │                            │ entry.normalized_text = "..."
    │<─ entry_updated({entry: {id:"abc-123", status:"processing", ...}})
    │                            │                          │
    │                            │──── stdin ──────────────>│
    │                            │  {"cmd":"synthesize",    │
    │                            │   "text":"Вызови гет юзер│
    │                            │    дата через эй пи ай",  │
    │                            │   "speaker":"xenia",     │
    │                            │   "sample_rate":48000,   │
    │                            │   "out_wav":"/...abc-123.wav"}
    │                            │                          │
    │                            │ (synthesis in progress)  │
    │<─ synthesis_progress({entry_id:"abc-123", progress:0.4})
    │<─ synthesis_progress({entry_id:"abc-123", progress:0.8})
    │                            │                          │
    │                            │<──── stdout ─────────────│
    │                            │  {"ok":true,             │
    │                            │   "timestamps":[...],    │
    │                            │   "duration_sec":3.7}    │
    │                            │                          │
    │                            │ Save timestamps file     │
    │                            │ entry.status = "ready"   │
    │                            │ entry.audio_path = "abc-123.wav"
    │                            │ entry.duration_sec = 3.7 │
    │<─ entry_updated({entry: {id:"abc-123", status:"ready", duration_sec:3.7, ...}})
    │                            │                          │
    │                            │ play_when_ready=true →   │
    │                            │ start playback           │
    │<─ playback_started({entry_id:"abc-123"})
    │                            │                          │
    │<─ playback_position({position_sec:0.1, entry_id:"abc-123"})
    │<─ playback_position({position_sec:0.2, entry_id:"abc-123"})
    │    ... (every ~100ms) ...  │                          │
    │<─ playback_finished({entry_id:"abc-123"})
    │<─ playback_stopped({})
```

---

### Example 2: Playback with seek

**Scenario:** User clicks "Play" on a ready entry, then drags the seek slider.

```
Frontend                    Rust Backend
    │                            │
    │─ invoke("play_entry", {id:"abc-123"}) ──────────────>│
    │<─ playback_started({entry_id:"abc-123"})              │
    │                            │                          │
    │<─ playback_position({position_sec:0.5, entry_id:"abc-123"})
    │<─ playback_position({position_sec:0.6, entry_id:"abc-123"})
    │                            │                          │
    │ (user drags slider to 2.0s)│                          │
    │─ invoke("seek_to", {position_sec:2.0}) ─────────────>│
    │<─ Promise resolves: void                              │
    │<─ playback_position({position_sec:2.0, entry_id:"abc-123"})
    │<─ playback_position({position_sec:2.1, entry_id:"abc-123"})
    │                            │                          │
    │ (user presses Space → pause)                          │
    │─ invoke("pause_playback") ──────────────────────────>│
    │<─ playback_paused({entry_id:"abc-123", position_sec:2.1})
    │                            │                          │
    │ (user presses Space → resume)                         │
    │─ invoke("resume_playback") ─────────────────────────>│
    │<─ playback_position({position_sec:2.1, entry_id:"abc-123"})
    │<─ playback_position({position_sec:2.2, entry_id:"abc-123"})
```

---

### Example 3: ttsd warmup at startup, successful synthesis, then error case

**Scenario:** Application starts, ttsd warms up, one synthesis succeeds, next fails.

```
Rust Backend                              ttsd (stdin/stdout)
    │                                          │
    │ spawn subprocess                         │ (process starts)
    │                                          │
    │──── {"cmd":"warmup"}\n ─────────────>   │
    │                                          │ torch.hub.load(...)
    │                                          │ model loaded successfully
    │<──── {"ok":true}\n ─────────────────   │
    │                                          │
    │ emit model_loaded event to frontend      │
    │                                          │
    │──── {"cmd":"synthesize",                 │
    │      "text":"привет мир",                │
    │      "speaker":"xenia",                  │
    │      "sample_rate":48000,                │
    │      "out_wav":"/...xyz.wav"}\n ──────> │
    │                                          │ synthesize chunks
    │                                          │ write xyz.wav
    │<──── {"ok":true,                         │
    │       "timestamps":[                     │
    │         {"word":"привет","start":0.0,    │
    │          "end":0.5,"original_pos":[0,6]},│
    │         {"word":"мир","start":0.55,      │
    │          "end":0.9,"original_pos":[7,10]}│
    │       ],                                 │
    │       "duration_sec":0.9}\n ──────────  │
    │                                          │
    │ (later, a bad entry is submitted)        │
    │──── {"cmd":"synthesize",                 │
    │      "text":"",                          │
    │      "speaker":"xenia",                  │
    │      "sample_rate":48000,                │
    │      "out_wav":"/...bad.wav"}\n ──────> │
    │                                          │ text is empty → validation fails
    │<──── {"ok":false,                        │
    │       "error":"bad_input",               │
    │       "message":"text must not be empty"}\n
    │                                          │
    │ mark entry as error                      │
    │ emit tts_error event                     │
    │                                          │
    │ (application exits)                      │
    │──── {"cmd":"shutdown"}\n ─────────────> │
    │<──── {"ok":true}\n ─────────────────   │
    │                                          │ process exits with code 0
```

---

## Implementation Notes

### Rust backend

- State is held in `tauri::State<AppState>` where `AppState` has fields: `storage: Arc<Mutex<StorageService>>`, `tts: Arc<TtsSubprocess>`, `player: Arc<Mutex<PlayerHandle>>`, `pipeline: Arc<TtsPipeline>`.
- The `TtsSubprocess` contains an `mpsc::Sender` for serializing all ttsd requests. Only one request is in flight at a time (matches ttsd's single-threaded design).
- Playback position polling is done in a `tokio::task::spawn` loop inside `PlayerHandle`, emitting `playback_position` events via `AppHandle::emit`.
- All Tauri commands returning errors use `Result<T, TauriError>` with `TauriError` implementing `serde::Serialize`.

### Frontend (`src/lib/tauri.ts`)

This file should export typed wrappers:

```typescript
// Example typed wrappers
export const addClipboardEntry = (playWhenReady: boolean): Promise<EntryId> =>
  invoke("add_clipboard_entry", { play_when_ready: playWhenReady });

export const listenEntryUpdated = (handler: (entry: TextEntry) => void) =>
  listen<{ entry: TextEntry }>("entry_updated", (e) => handler(e.payload.entry));
```

### ttsd (`ttsd/ttsd/protocol.py`)

Request and response types should be defined using pydantic v2 or `TypedDict`. Example:

```python
from typing import Literal
from pydantic import BaseModel

class SynthesizeRequest(BaseModel):
    cmd: Literal["synthesize"]
    text: str
    speaker: str
    sample_rate: int
    out_wav: str
    char_mapping: list[dict] | None = None

class SynthesizeSuccess(BaseModel):
    ok: Literal[True] = True
    timestamps: list[dict]
    duration_sec: float

class ErrorResponse(BaseModel):
    ok: Literal[False] = False
    error: str
    message: str
```

The main loop in `ttsd/ttsd/main.py` reads `sys.stdin`, line by line, parses with `json.loads`, dispatches to the appropriate handler, and writes `json.dumps(response) + "\n"` to `sys.stdout`. `sys.stdout.flush()` must be called after every write to prevent buffering.
