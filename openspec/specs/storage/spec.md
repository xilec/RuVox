# Storage Specification

## Purpose

Covers the on-disk persistence layer of RuVox (`src-tauri/src/storage/`): the per-user cache directory layout, the `history.json` entry store, the `config.json` application configuration, synthesized audio files (`{uuid}.opus`), word-level timestamp files (`{uuid}.timestamps.json`), the legacy WAV-to-Opus migration, and cache hygiene (orphan sweep and size-based eviction).

## Requirements

### Requirement: Cache Directory Layout

The system SHALL store all persistent data under a per-user cache root resolved as `dirs::cache_dir()/ruvox` (e.g. `~/.cache/ruvox/`), with the following layout:

```
~/.cache/ruvox/
├── history.json                         # Versioned list of TextEntry records
├── config.json                          # Application configuration (UIConfig)
└── audio/
    ├── {uuid}.opus                      # Ogg-Opus audio (32 kbps VOIP, mono)
    └── {uuid}.timestamps.json           # Word-level timestamps for the entry
```

The storage service SHALL create the cache root and the `audio/` subdirectory on initialization if they do not exist.

#### Scenario: First launch creates the directory tree
- GIVEN the cache directory does not exist
- WHEN the storage service is initialized
- THEN the cache root and its `audio/` subdirectory exist on disk

#### Scenario: Default cache root location
- GIVEN no custom cache directory is configured
- WHEN the storage service is constructed with defaults
- THEN the cache root is the platform per-user cache directory joined with `ruvox`

### Requirement: History File Schema

The system SHALL persist the text entry queue to `history.json` as a versioned JSON document with schema version `1`:

```typescript
interface HistoryFile {
  version: number;          // Schema version. Starts at 1.
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
  created_at: string;                   // Naive timestamp, e.g. "2026-02-15T11:46:51.504055" (no TZ suffix)
  audio_generated_at: string | null;    // Naive timestamp when audio file was written
  audio_path: string | null;            // Filename relative to audio/, e.g. "{uuid}.opus"
  timestamps_path: string | null;       // Filename relative to audio/, e.g. "{uuid}.timestamps.json"
  duration_sec: number | null;          // Audio duration in seconds
  was_regenerated: boolean;             // True if audio was re-synthesized at least once
  error_message: string | null;         // Human-readable error if status == "error"
}
```

Status values SHALL serialize as lowercase strings. `created_at` and `audio_generated_at` SHALL be stored as naive timestamps without a timezone suffix; `created_at` is generated from the local clock (`Local::now().naive_local()`) and readers treat the values as UTC. `audio_path` and `timestamps_path` SHALL store the filename only; the full path resolves as `<cache_root>/audio/{filename}`. All optional `TextEntry` fields SHALL default when absent from the JSON, so entries written by older builds keep parsing.

#### Scenario: New entry is persisted
- GIVEN an empty history
- WHEN a new entry is added
- THEN `history.json` contains a `version: 1` wrapper and the entry with status `"pending"`, a UUID v4 `id`, and null audio fields

#### Scenario: History round-trips through disk
- GIVEN an entry with `normalized_text` set and status `"ready"`
- WHEN the storage service is re-initialized against the same cache directory
- THEN the loaded entry has the same `normalized_text` and field values as before the restart

#### Scenario: Older entries without optional fields parse
- GIVEN a `history.json` entry that lacks `normalized_text`, `audio_path`, `was_regenerated`, and other optional fields
- WHEN the history is loaded
- THEN the entry parses successfully with the missing fields defaulted (null / false)

### Requirement: Playing Status Is Never Persisted

The status `"playing"` SHALL be a runtime-only state. Before writing `history.json`, the storage service SHALL normalize any entry in `"playing"` state to `"ready"`.

#### Scenario: Saving while an entry plays
- GIVEN an entry whose in-memory status is `"playing"`
- WHEN the history is persisted
- THEN the entry is written to `history.json` with status `"ready"`

### Requirement: Status Validation on Load

On load, the storage service SHALL reconcile each entry's status with the files actually present in `audio/` and SHALL persist the history again if any entry was modified:

- An entry with status `"processing"` and no `audio_path` SHALL be reset to `"pending"` (the process that was synthesizing it no longer exists).
- An entry with status `"ready"` and no `audio_path` SHALL be reset to `"pending"`.
- An entry whose `audio_path` file is missing SHALL have its audio metadata (`audio_path`, `timestamps_path`, `duration_sec`, `audio_generated_at`) cleared; if its status was `"ready"`, it SHALL be reset to `"pending"`.
- An entry whose `audio_path` file exists but whose status is not `"ready"` SHALL be set to `"ready"`.

#### Scenario: Interrupted synthesis is reset
- GIVEN a persisted entry with status `"processing"` and null `audio_path`
- WHEN the history is loaded
- THEN the entry status becomes `"pending"` and the corrected history is saved

#### Scenario: Ready entry with missing audio file
- GIVEN a persisted entry with status `"ready"` whose `audio_path` file does not exist in `audio/`
- WHEN the history is loaded
- THEN the entry status becomes `"pending"` and its audio metadata fields are null

#### Scenario: Audio file present but status not ready
- GIVEN a persisted entry with status `"pending"` whose `audio_path` file exists in `audio/`
- WHEN the history is loaded
- THEN the entry status becomes `"ready"`

### Requirement: Corrupted History Recovery

If `history.json` cannot be parsed as JSON, the storage service SHALL rename it to `history.json.bak` and start with an empty history. If `history.json` cannot be read at all, the service SHALL log a warning and start with an empty history. If the persisted `version` is newer than the supported version, the service SHALL log a warning and load the entries anyway.

#### Scenario: Corrupted history file
- GIVEN a `history.json` containing invalid JSON
- WHEN the storage service is initialized
- THEN the service starts with an empty history and a `history.json.bak` backup exists next to the original

#### Scenario: Newer schema version
- GIVEN a `history.json` whose `version` is greater than the supported `1`
- WHEN the storage service is initialized
- THEN the entries are loaded and a warning is logged

### Requirement: Atomic UTF-8 Writes

The storage service SHALL write `history.json`, `config.json`, and timestamp files atomically: write to a sibling `.tmp` file, then rename over the target. All JSON files SHALL be UTF-8 without BOM, with Cyrillic characters written unescaped.

#### Scenario: Atomic history write
- GIVEN any state change that persists history
- WHEN the file is written
- THEN the content first lands in a temporary file and is renamed over `history.json`, so a crash cannot leave a truncated target

#### Scenario: Cyrillic text is stored unescaped
- GIVEN an entry whose `original_text` contains Cyrillic characters
- WHEN the history is persisted
- THEN the JSON file contains the Cyrillic text verbatim (no `\uXXXX` escapes) in UTF-8 without BOM

### Requirement: Audio File Storage

The system SHALL store synthesized audio per entry as `audio/{uuid}.opus`, where `{uuid}` is the entry's `EntryId`. The file SHALL be an Ogg-Opus stream:

| Property | Value |
|----------|-------|
| Container | Ogg |
| Codec | Opus (RFC 6716, RFC 7845) |
| Channels | 1 (mono) |
| Sample rate | One of 8 / 12 / 16 / 24 / 48 kHz — matches `UIConfig.sample_rate` (default 48 kHz); recorded verbatim in `OpusHead` |
| Bitrate | 32 000 bps (VOIP application) |
| Frame size | 20 ms |
| Pre-skip | Queried from `libopus`'s lookahead, scaled to 48 kHz output ticks |

The encoding pipeline is: the TTS subprocess writes a mono 32-bit-float WAV at `UIConfig.sample_rate`; the Rust side transcodes it to Opus and removes the source WAV. On encode failure the source `.wav` SHALL be left in place as a playback fallback. `save_audio` SHALL return the relative filename for `TextEntry.audio_path`.

#### Scenario: Saving audio returns the relative filename
- GIVEN an entry with id `550e8400-e29b-41d4-a716-446655440000`
- WHEN audio bytes are saved for the entry
- THEN the file `audio/550e8400-e29b-41d4-a716-446655440000.opus` exists and the returned filename is `550e8400-e29b-41d4-a716-446655440000.opus`

#### Scenario: Transcode failure keeps the WAV fallback
- GIVEN a synthesized `.wav` that fails Opus encoding
- WHEN the transcode step runs
- THEN the source `.wav` remains on disk so playback can still use it

### Requirement: Legacy WAV to Opus Migration

On every app launch the system SHALL run a one-shot migration sweep over the loaded entries: any entry whose `audio_path` ends in `.wav` SHALL be transcoded to `.opus`, the entry's `audio_path` updated to the new filename, and the source `.wav` removed. The sweep SHALL be idempotent (already-`.opus` entries are not considered) and SHALL NOT abort on per-entry failures — encode errors and missing source files are logged and counted while the app keeps starting normally. Legacy `.wav` references in `history.json` SHALL continue to parse indefinitely.

#### Scenario: Legacy entry is migrated
- GIVEN an entry whose `audio_path` points at an existing `{uuid}.wav`
- WHEN the migration sweep runs
- THEN the entry's `audio_path` ends in `.opus`, the `.opus` file exists, and the source `.wav` is removed

#### Scenario: Migration is idempotent
- GIVEN all entries already reference `.opus` files
- WHEN the migration sweep runs
- THEN no entries are considered and no files are touched

#### Scenario: Missing source file does not abort the sweep
- GIVEN one entry referencing a missing `.wav` and another referencing an existing `.wav`
- WHEN the migration sweep runs
- THEN the missing one is skipped with a warning and the existing one is migrated

### Requirement: Word Timestamps File

The system SHALL store word-level timing information per entry as `audio/{uuid}.timestamps.json`, produced by the TTS subprocess and used by the frontend to highlight words during playback:

```typescript
interface Timestamps {
  words: WordTimestamp[];
}

interface WordTimestamp {
  word: string;                    // Normalized word as spoken by the TTS engine
  start: number;                   // Start time in seconds (relative to audio start)
  end: number;                     // End time in seconds
  original_pos: [number, number];  // [start, end] character offsets in original_text
}
```

`original_pos` SHALL map each spoken word to a character range in the pre-normalization `original_text` string, so the UI can highlight the source text while the normalized text is being spoken. Multiple normalized words MAY map to the same `original_pos` range (e.g. `getUserData` → `["гет", "юзер", "дата"]`). `timestamps_path` SHALL store the filename only. Loading timestamps for an entry with no `timestamps_path` or a missing file SHALL return no timestamps.

#### Scenario: Timestamps round-trip
- GIVEN an entry with synthesized audio
- WHEN word timestamps are saved and then loaded for that entry
- THEN each word preserves its `word`, `start`, `end`, and `original_pos` values, with `original_pos` serialized as a 2-element JSON array

#### Scenario: Loading without timestamps
- GIVEN an entry whose `timestamps_path` is null or points at a missing file
- WHEN timestamps are loaded for that entry
- THEN no timestamps are returned and no error is raised

### Requirement: Config File Schema

The system SHALL persist application configuration to `config.json` as a `UIConfig` JSON object:

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `speaker` | string | `"xenia"` | Silero speaker name |
| `sample_rate` | number | `48000` | TTS output rate; any of 8000 / 12000 / 16000 / 24000 / 48000 round-trips through the Opus encoder without resampling |
| `speech_rate` | number | `1.0` | Playback speed multiplier (0.5–2.0) |
| `notify_on_ready` | boolean | `true` | Show notification when synthesis completes |
| `notify_on_error` | boolean | `true` | Show notification on synthesis error |
| `text_format` | string | `"plain"` | Default viewer format: `"plain"` / `"markdown"` / `"html"` |
| `max_cache_size_mb` | number | `500` | Soft limit on audio cache size in MB; drives startup eviction (0 = disabled) |
| `code_block_mode` | string | `"read"` | How to handle Markdown code blocks: `"skip"` / `"read"` |
| `read_operators` | boolean | `true` | Whether to speak mathematical/code operators |
| `theme` | string | `"auto"` | Color scheme: `"light"` / `"dark"` / `"auto"` |
| `player_hotkeys` | object | 10-key map (`play_pause` → `"Space"`, `forward_5` → `"Right"`, `backward_5` → `"Left"`, `forward_30` → `"Shift+Right"`, `backward_30` → `"Shift+Left"`, `speed_up` → `"]"`, `speed_down` → `"["`, `next_entry` → `"n"`, `prev_entry` → `"p"`, `repeat_sentence` → `"r"`) | Local player hotkeys |
| `window_geometry` | `[x, y, width, height]` or null | `null` | Saved window geometry |
| `preview_dialog_enabled` | boolean | `true` | Show normalization preview dialog before synthesis |
| `engine` | string | `"piper"` | Active TTS engine: `"piper"` / `"silero"` |
| `piper_voice` | string | `"ruslan"` | Active Piper voice id |

Every field SHALL default when absent from the JSON, so configs written by older builds parse cleanly and silently adopt current defaults (e.g. pre-engine configs switch to `"piper"`). Unknown JSON keys SHALL be ignored on read. When `config.json` does not exist, the service SHALL return the default configuration. Partial updates SHALL be expressed as a patch object in which omitted fields keep their current value.

#### Scenario: Missing config returns defaults
- GIVEN no `config.json` in the cache directory
- WHEN the configuration is loaded
- THEN the default configuration is returned (`speaker` `"xenia"`, `sample_rate` `48000`, `engine` `"piper"`, `piper_voice` `"ruslan"`)

#### Scenario: Older config without engine keys
- GIVEN a `config.json` that contains only `speaker`, `sample_rate`, and `speech_rate`
- WHEN the configuration is loaded
- THEN it parses successfully with `engine` defaulted to `"piper"` and `piper_voice` defaulted to `"ruslan"`

#### Scenario: Config round-trips
- GIVEN a configuration with `speaker` `"aidar"` and `sample_rate` `24000`
- WHEN the configuration is saved and loaded again
- THEN the loaded values match the saved ones

### Requirement: Entry CRUD

The storage service SHALL provide create, read, update, and delete operations over entries:

- Adding an entry SHALL strip a leading UTF-8 BOM from the text, assign a fresh UUID v4, set status `"pending"`, and persist the history.
- Updating an entry SHALL replace the stored record and persist the history.
- Listing entries SHALL return them sorted by `created_at`, newest first.
- Deleting an entry SHALL remove its record, its audio file, and its timestamps file, then persist the history.

#### Scenario: BOM is stripped on add
- GIVEN text that starts with the UTF-8 BOM character `﻿`
- WHEN the entry is added
- THEN `original_text` is stored without the BOM

#### Scenario: Entries are listed newest first
- GIVEN two entries created one second apart
- WHEN all entries are listed
- THEN the more recently created entry comes first

#### Scenario: Deleting an entry removes its files
- GIVEN a ready entry with audio and timestamps files on disk
- WHEN the entry is deleted
- THEN the entry is gone from history and both files are removed from `audio/`

### Requirement: Audio-Only Deletion

The system SHALL support deleting only an entry's audio and timestamps files while keeping the text entry: the entry's `audio_path`, `timestamps_path`, `audio_generated_at`, and `duration_sec` SHALL be cleared and its status reset to `"pending"`.

#### Scenario: Regeneration frees old audio
- GIVEN a ready entry with audio and timestamps files on disk
- WHEN audio-only deletion runs for that entry
- THEN the entry remains in history with status `"pending"` and null audio fields, and both files are removed

### Requirement: Orphan Sweep

The system SHALL remove files in `audio/` that are referenced by no entry (neither as `audio_path` nor as `timestamps_path`). Files modified within a 60-second grace window SHALL be preserved to avoid racing with in-flight synthesis whose output is on disk but not yet recorded in `history.json`.

#### Scenario: Orphan file is removed
- GIVEN an aged file in `audio/` referenced by no entry
- WHEN the orphan sweep runs
- THEN the file is deleted and the reclaimed bytes are reported

#### Scenario: Recent file survives the sweep
- GIVEN a file in `audio/` modified seconds ago and referenced by no entry
- WHEN the orphan sweep runs
- THEN the file is preserved

### Requirement: Size-Based Eviction

The system SHALL evict entries oldest-first until the cumulative on-disk size of all entries' audio and timestamps files fits a target byte limit. A target of `0` SHALL disable eviction (no-op). Entries currently `"processing"` SHALL be skipped. With `delete_texts = false` only the audio and timestamps files are removed and the entry is reset to `"pending"`; with `delete_texts = true` the entry SHALL be removed from `history.json` entirely.

#### Scenario: Oldest entries are evicted first
- GIVEN three ready entries whose combined file size exceeds the target
- WHEN size-based eviction runs with `delete_texts = false`
- THEN the oldest entries lose their audio files and are reset to `"pending"` until the total fits, while the newest keeps its audio

#### Scenario: Zero target disables eviction
- GIVEN a ready entry with audio on disk
- WHEN eviction runs with target `0`
- THEN nothing is removed

#### Scenario: Processing entries are protected
- GIVEN an entry in `"processing"` status that is the oldest candidate
- WHEN size-based eviction runs
- THEN that entry's audio is not touched

### Requirement: Startup Cache Cleanup

On app startup, after the WAV-to-Opus migration completes, the system SHALL run a cache cleanup consisting of the orphan sweep followed by size-based eviction toward `max_cache_size_mb` (converted to bytes), always with `delete_texts = false` — automatic deletion of texts SHALL NOT happen without an explicit user gesture. Migration SHALL finish before the sweep so freshly-renamed `.opus` files are already linked to their entries.

#### Scenario: Startup cleanup keeps texts
- GIVEN an aged orphan file and entries whose total size exceeds `max_cache_size_mb`
- WHEN startup cleanup runs
- THEN the orphan is removed, the oldest entries' audio is evicted, and every text entry remains in `history.json`

### Requirement: Backwards Compatibility

The on-disk format originated in the earlier PyQt implementation of RuVox, and existing user files SHALL keep working without migration: fields added later use serde defaults so older JSON parses cleanly, and fields that no longer exist in the current schema SHALL be silently ignored on read.

#### Scenario: PyQt-era history file loads
- GIVEN a `history.json` written by the pre-Tauri build (including a legacy `.wav` `audio_path` and no `was_regenerated` field)
- WHEN the storage service is initialized
- THEN the entries load successfully with missing fields defaulted

#### Scenario: Unknown config keys are ignored
- GIVEN a `config.json` containing keys not present in the current `UIConfig` schema
- WHEN the configuration is loaded
- THEN it parses successfully and the unknown keys are dropped
