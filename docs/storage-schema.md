# Storage Schema

JSON schemas for all files written by RuVox to its cache directory.

---

## Directory Layout

```
~/.cache/ruvox/
├── history.json                         # Versioned list of TextEntry records
├── config.json                          # Application configuration (UIConfig)
└── audio/
    ├── {uuid}.opus                      # Ogg-Opus audio (32 kbps VOIP, mono)
    └── {uuid}.timestamps.json           # Word-level timestamps for the entry
```

> Audio used to be stored as raw 32-bit float WAV. RuVox 0.2.0+ writes Ogg-Opus
> (≈40× smaller for the same content) and migrates legacy `.wav` files on
> startup — see [Migration](#migration-wav--opus).

The cache root defaults to `~/.cache/ruvox/`. It is stored per-user.

---

## Backwards compatibility

The on-disk format originated in the earlier PyQt implementation of RuVox. Existing users moving to the Tauri build keep their `history.json`/`config.json` without migration — `serde` attributes (`rename_all`, `#[serde(default)]`) cover both directions:

- New fields added later (e.g., `preview_dialog_enabled` in `UIConfig`) use `#[serde(default)]`, so older JSON parses cleanly.
- Fields that no longer exist in the current schema (e.g., the experimental `edited_text` override) are silently ignored on read.

---

## `history.json`

**Path:** `~/.cache/ruvox/history.json`

### Schema

```typescript
interface HistoryFile {
  version: number;          // Schema version. Starts at 1. Increment on breaking changes.
  entries: TextEntry[];
}

type EntryId = string;      // UUID v4, e.g. "550e8400-e29b-41d4-a716-446655440000"

type EntryStatus =
  | "pending"     // Waiting for TTS synthesis
  | "processing"  // TTS synthesis is running
  | "ready"       // Audio is synthesized and playable
  | "playing"     // Runtime-only: entry is currently playing. NEVER persisted.
  | "error";      // Synthesis failed

interface TextEntry {
  id: EntryId;
  original_text: string;
  normalized_text: string | null;       // Output of the Rust TTS pipeline
  status: EntryStatus;
  created_at: string;                   // Naive UTC timestamp, e.g. "2026-02-15T11:46:51.504055" (no TZ suffix)
  audio_generated_at: string | null;    // Naive UTC timestamp when audio file was written
  audio_path: string | null;            // Filename relative to audio/, e.g. "{uuid}.opus"
                                        // (legacy entries may still reference "{uuid}.wav" until migrated)
  timestamps_path: string | null;       // Filename relative to audio/, e.g. "{uuid}.timestamps.json"
  duration_sec: number | null;          // Audio duration in seconds
  was_regenerated: boolean;             // True if audio was re-synthesized at least once
  error_message: string | null;         // Human-readable error if status == "error"
}
```

> **Timestamp format:** `created_at` and `audio_generated_at` are stored as naive UTC timestamps — no timezone suffix (e.g. `"2026-02-15T11:46:51.504055"`). All values are treated as UTC.

### Notes on `status`

`"playing"` is a runtime state only. The storage layer **never persists** an entry with `status: "playing"`. Before writing, any entry in `"playing"` state is saved as `"ready"`.

On load, entries whose `status` is `"processing"` AND have no `audio_path` are reset to `"pending"` (the process that was synthesizing them no longer exists).

Entries with `status: "ready"` whose `audio_path` file is missing are reset to `"pending"`.

### Example

```json
{
  "version": 1,
  "entries": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "original_text": "Вызови getUserData() через API",
      "normalized_text": "Вызови гет юзер дата через эй пи ай",
      "status": "ready",
      "created_at": "2025-01-15T10:30:00.123456",
      "audio_generated_at": "2025-01-15T10:30:05.654321",
      "audio_path": "550e8400-e29b-41d4-a716-446655440000.opus",
      "timestamps_path": "550e8400-e29b-41d4-a716-446655440000.timestamps.json",
      "duration_sec": 3.7,
      "was_regenerated": false,
      "error_message": null
    },
    {
      "id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
      "original_text": "https://example.com — сломан",
      "normalized_text": null,
      "status": "error",
      "created_at": "2025-01-15T11:00:00.000000",
      "audio_generated_at": null,
      "audio_path": null,
      "timestamps_path": null,
      "duration_sec": null,
      "was_regenerated": false,
      "error_message": "synthesis_failed: Silero raised an exception"
    }
  ]
}
```

### Versioning

`history.json` carries a `version` field. Current version is **1**.

When the schema changes in a breaking way:
1. Increment `version`.
2. Add migration logic in the Rust storage service.
3. Document the change here.

---

## `audio/{uuid}.opus`

**Path:** `~/.cache/ruvox/audio/{uuid}.opus`

An Ogg-Opus file:

| Property | Value |
|----------|-------|
| Container | Ogg |
| Codec | Opus (RFC 6716, RFC 7845) |
| Channels | 1 (mono) |
| Sample rate | One of 8 / 12 / 16 / 24 / 48 kHz — matches `UIConfig.sample_rate` (default 48 kHz) |
| Bitrate | 32 000 bps (VBR, VOIP application) |
| Frame size | 20 ms |
| Pre-skip | Queried from `libopus`'s `lookahead`, scaled to 48 kHz output ticks |

The filename is the entry's UUID, e.g. `550e8400-e29b-41d4-a716-446655440000.opus`.

`audio_path` in `TextEntry` stores the **filename only** (not the full path). The full path is resolved as `~/.cache/ruvox/audio/{audio_path}`.

Encoding pipeline: `ttsd` writes a mono 32-bit-float WAV at `UIConfig.sample_rate`; the Rust side immediately transcodes it to Opus via `crate::audio::replace_wav_with_opus` and removes the source WAV. The encoder uses the `opus = "0.3"` crate (FFI to system `libopus` 1.x, see [nix/devshell.nix](../nix/devshell.nix) / [flake.nix](../flake.nix)). The Opus container records the input sample rate verbatim; decoders (mpv, libopusfile, browsers) handle any-to-48 kHz output internally. On encode failure the source `.wav` is left in place as a fallback so playback keeps working.

### Migration: WAV → Opus

On every app launch RuVox runs a one-shot migration sweep over `history.json`: any entry whose `audio_path` still ends in `.wav` is transcoded to `.opus` (in place, idempotent) and the source `.wav` is removed. Per-entry failures are logged and do not abort the sweep; the app keeps starting normally regardless. See `StorageService::migrate_wav_audio_to_opus` in `src-tauri/src/storage/service.rs`.

The legacy `.wav` field value continues to parse from `history.json` indefinitely (see `schema::tests::deserialize_real_history_format`), so a downgrade to a pre-Opus build remains possible until the migration runs.

---

## `audio/{uuid}.timestamps.json`

**Path:** `~/.cache/ruvox/audio/{uuid}.timestamps.json`

Stores word-level timing information produced by `ttsd` (the Python TTS subprocess). Used by the frontend to highlight words during playback.

### Schema

```typescript
interface Timestamps {
  words: WordTimestamp[];
}

interface WordTimestamp {
  word: string;                    // Normalized word as spoken by Silero
  start: number;                   // Start time in seconds (relative to audio start)
  end: number;                     // End time in seconds
  original_pos: [number, number];  // [start, end] character offsets in original_text
}
```

### Example

```json
{
  "words": [
    { "word": "вызови",  "start": 0.00, "end": 0.42, "original_pos": [0, 6] },
    { "word": "гет",     "start": 0.45, "end": 0.65, "original_pos": [7, 14] },
    { "word": "юзер",    "start": 0.67, "end": 0.90, "original_pos": [7, 14] },
    { "word": "дата",    "start": 0.92, "end": 1.15, "original_pos": [7, 14] },
    { "word": "через",   "start": 1.20, "end": 1.55, "original_pos": [15, 20] },
    { "word": "эй",      "start": 1.60, "end": 1.75, "original_pos": [21, 24] },
    { "word": "пи",      "start": 1.77, "end": 1.90, "original_pos": [21, 24] },
    { "word": "ай",      "start": 1.92, "end": 2.05, "original_pos": [21, 24] }
  ]
}
```

`original_pos` maps each spoken word to a character range in the **pre-normalization** (`original_text`) string, so the UI can highlight the source text while the normalized text is being spoken.

Multiple normalized words may map to the same `original_pos` range (e.g., "getUserData" → ["гет", "юзер", "дата"]).

---

## `config.json`

**Path:** `~/.cache/ruvox/config.json`

Stores the application configuration. Written by the Rust storage service.

### Schema

```typescript
interface UIConfig {
  version?: number;             // Config schema version (read but not required)
  speaker: string;              // Silero speaker name: "xenia" | "aidar" | "baya" | "kseniya" | "eugene"
  sample_rate: number;          // Silero output rate: 8000 | 24000 | 48000. The Opus encoder
                                // accepts the same set natively (plus 12/16 kHz) so any of these
                                // round-trips through the pipeline without resampling.
  speech_rate: number;          // Playback speed multiplier: 0.5–2.0
  notify_on_ready: boolean;     // Show notification when synthesis completes
  notify_on_error: boolean;     // Show notification on synthesis error
  text_format: string;          // Default viewer format: "plain" | "markdown" | "html"
  max_cache_size_mb: number;    // Soft limit on audio cache size in MB; drives startup eviction (0 = disabled)
  code_block_mode: string;      // How to handle Markdown code blocks: "skip" | "read"
  read_operators: boolean;      // Whether to speak mathematical/code operators
  theme: string;                // Color scheme: "light" | "dark" | "auto"
  player_hotkeys: Record<string, string>; // Local player hotkeys map
  window_geometry: [number, number, number, number] | null; // [x, y, width, height]
  preview_dialog_enabled: boolean; // Show FF 1.1 preview dialog before synthesis
}
```

### Default Values

| Field | Default |
|-------|---------|
| `speaker` | `"xenia"` |
| `sample_rate` | `48000` |
| `speech_rate` | `1.0` |
| `notify_on_ready` | `true` |
| `notify_on_error` | `true` |
| `text_format` | `"plain"` |
| `max_cache_size_mb` | `500` |
| `code_block_mode` | `"read"` |
| `read_operators` | `true` |
| `theme` | `"auto"` |
| `preview_dialog_enabled` | `true` |

### Example

```json
{
  "version": 1,
  "speaker": "xenia",
  "sample_rate": 48000,
  "speech_rate": 1.0,
  "notify_on_ready": true,
  "notify_on_error": true,
  "text_format": "plain",
  "max_cache_size_mb": 500,
  "code_block_mode": "read",
  "read_operators": true,
  "theme": "auto",
  "player_hotkeys": {
    "play_pause": "Space",
    "forward_5": "Right",
    "backward_5": "Left",
    "forward_30": "Shift+Right",
    "backward_30": "Shift+Left",
    "speed_up": "]",
    "speed_down": "[",
    "next_entry": "n",
    "prev_entry": "p",
    "repeat_sentence": "r"
  },
  "window_geometry": null,
  "preview_dialog_enabled": true
}
```

---

## Field Reference

Cross-reference between `docs/ipc-contract.md` types and the JSON fields in each file.

### TextEntry fields

| Field | JSON key | Type | history.json | IPC (TextEntry) |
|-------|----------|------|:---:|:---:|
| Entry ID | `id` | UUID string | yes | yes |
| Original text | `original_text` | string | yes | yes |
| Normalized text | `normalized_text` | string or null | yes | yes |
| Status | `status` | enum string | yes | yes |
| Created at | `created_at` | ISO 8601 | yes | yes |
| Audio generated at | `audio_generated_at` | ISO 8601 or null | yes | yes |
| Audio path | `audio_path` | filename or null | yes | yes |
| Timestamps path | `timestamps_path` | filename or null | yes | yes |
| Duration | `duration_sec` | number or null | yes | yes |
| Was regenerated | `was_regenerated` | boolean | yes | yes |
| Error message | `error_message` | string or null | yes | yes |

### UIConfig fields

| Field | JSON key | history.json | config.json | IPC (UIConfig) |
|-------|----------|:---:|:---:|:---:|
| Speaker | `speaker` | — | yes | yes |
| Sample rate | `sample_rate` | — | yes | yes |
| Speech rate | `speech_rate` | — | yes | yes |
| Notify on ready | `notify_on_ready` | — | yes | yes |
| Notify on error | `notify_on_error` | — | yes | yes |
| Text format | `text_format` | — | yes | yes |
| Max cache MB | `max_cache_size_mb` | — | yes | yes |
| Code block mode | `code_block_mode` | — | yes | yes |
| Read operators | `read_operators` | — | yes | yes |
| Theme | `theme` | — | yes | yes |
| Player hotkeys | `player_hotkeys` | — | yes | yes |
| Window geometry | `window_geometry` | — | yes | yes |
| Preview dialog enabled | `preview_dialog_enabled` | — | yes | yes |

### WordTimestamp fields

| Field | JSON key | timestamps.json | IPC (WordTimestamp) |
|-------|----------|:---:|:---:|
| Word | `word` | yes | yes |
| Start time | `start` | yes | yes |
| End time | `end` | yes | yes |
| Original position | `original_pos` | yes (2-element array) | yes (tuple) |

---

## Implementation Notes

### Rust types

The canonical Rust types are in `src-tauri/src/storage/schema.rs`:

- `EntryId = Uuid`
- `EntryStatus` — `#[serde(rename_all = "lowercase")]`
- `TextEntry` — all optional fields use `#[serde(default)]`
- `HistoryFile` — top-level wrapper with `version: u32`
- `WordTimestamp` — `original_pos: (usize, usize)` serializes as a 2-element JSON array
- `Timestamps` — top-level wrapper with `words: Vec<WordTimestamp>`
- `UIConfig` — `Default` impl uses the values from the table above
- `UIConfigPatch` — all fields `Option<T>` for partial updates via `update_config` Tauri command

### Atomic writes

The storage service must write `history.json` atomically (write to a `.tmp` file, then `rename`) to avoid corruption on crash. Same applies to `config.json`.

### Encoding

All JSON files use UTF-8 without BOM. `ensure_ascii: false` — Cyrillic characters are written as-is, not escaped.
