# Storage Specification

## Purpose

Covers the on-disk persistence layer of RuVox (`src-tauri/src/storage/`): the per-user cache directory layout, the `history.json` entry store, the `config.json` application configuration, synthesized audio files (`{uuid}.opus`), word-level timestamp files (`{uuid}.timestamps.json`), the legacy WAV-to-Opus migration, and cache hygiene (orphan sweep and size-based eviction).

## Requirements

### Requirement: Cache Directory Layout

The system SHALL store persistent data under two per-user roots: a **data root**
holding `history.json` and `audio/`, and a **config root** holding `config.json`:

```
<data_root>/
├── history.json                         # Versioned list of TextEntry records
└── audio/
    ├── {uuid}.opus                      # Ogg-Opus audio (32 kbps VOIP, mono)
    └── {uuid}.timestamps.json           # Word-level timestamps for the entry

<config_root>/
└── config.json                          # Application configuration (UIConfig)
```

The roots are platform-dependent:

- **Windows:** both roots coincide with `dirs::data_local_dir()/<bundle identifier>` (`%LOCALAPPDATA%\com.ruvox.app`). They MUST NOT coincide with the NSIS install dir (`%LOCALAPPDATA%\<productName>`) and MUST match the directory the NSIS uninstaller removes via its "Delete the application data" checkbox.
- **Other platforms:** the data root is `dirs::data_local_dir()/ruvox` (e.g. `~/.local/share/ruvox/`) and the config root is `dirs::config_dir()/ruvox` (e.g. `~/.config/ruvox/`).

The storage service SHALL create each root and the `audio/` subdirectory on initialization if they do not exist.

#### Scenario: First launch creates the directory tree
- GIVEN neither the data root nor the config root exists
- WHEN the storage service is initialized
- THEN the data root with its `audio/` subdirectory and the config root exist on disk

#### Scenario: Default cache root location
- GIVEN no custom directories are configured
- WHEN the storage service is constructed with defaults
- THEN the data root is `%LOCALAPPDATA%\com.ruvox.app` on Windows and `~/.local/share/ruvox/` on Linux, and the config root is the same directory as the data root on Windows and `~/.config/ruvox/` on Linux

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

type TextFormat =
  | "plain"
  | "markdown"
  | "html";

type EntrySource =
  | "clipboard"
  | "file"
  | "url";

interface TextEntry {
  id: EntryId;
  original_text: string;
  normalized_text: string | null;       // Output of the Rust TTS pipeline
  status: EntryStatus;
  format: TextFormat | null;            // Display format chosen for this entry; null = never chosen
  html_source: string | null;           // Sanitized HTML for rendering; set only for HTML-ingested entries
  source: EntrySource | null;           // Where the text came from; null for legacy entries
  created_at: string;                   // Naive UTC timestamp, e.g. "2026-02-15T11:46:51.504055" (no TZ suffix)
  audio_generated_at: string | null;    // Naive UTC timestamp when audio file was written
  audio_path: string | null;            // Filename relative to audio/, e.g. "{uuid}.opus"
  timestamps_path: string | null;       // Filename relative to audio/, e.g. "{uuid}.timestamps.json"
  duration_sec: number | null;          // Audio duration in seconds
  was_regenerated: boolean;             // True if audio was re-synthesized at least once
  generation_count: number;             // Times audio was successfully baked (default 0); survives audio deletion
  generation: GenerationParams | null;  // Snapshot of the parameters that produced the current audio
  error_message: string | null;         // Human-readable error if status == "error"
}

interface GenerationParams {
  engine: string;                        // Engine that produced the audio: "silero_native" | "piper" | "silero"
  voice: string;                         // Engine-specific voice id actually used ("xenia", "ruslan", ...)
  sample_rate: number | null;            // Actual output sample rate of the rendered audio; null when unknown
  model: ModelParams | null;             // Model identity; null when the engine cannot report it cheaply
  app_version: string;                   // Application version at generation time
  code_block_mode: string | null;        // Normalization input: "skip" | "read"
  read_operators: boolean | null;        // Normalization input: read spoken operators
  normalized_text_sha256: string | null; // sha256 of the normalized text used for this audio
  audio_codec: string | null;            // Codec of the stored file ("Ogg Opus" | "WAV")
  audio_bytes: number | null;            // Size of the stored audio file in bytes
}

interface ModelParams {
  name: string;           // e.g. silero-native bundle "model_id" or Piper voice model file name
  sha256: string | null;  // Checksum where cheaply available (silero-native bundle manifest)
}
```

Status and format values SHALL serialize as lowercase strings. `created_at` and `audio_generated_at` SHALL be stored as naive timestamps without a timezone suffix; both SHALL be generated from the UTC clock (`Utc::now().naive_utc()`), and readers treat the values as UTC. `audio_path` and `timestamps_path` SHALL store the filename only; the full path resolves as `<cache_root>/audio/{filename}`. All optional `TextEntry` fields SHALL default when absent from the JSON, so entries written by older builds keep parsing; a missing `format` SHALL default to `null`, meaning the viewer falls back to the `text_format` config default, a missing `html_source` SHALL default to `null`, a missing `generation` SHALL default to `null`, a missing `generation_count` SHALL default to `0`, and a missing `source` SHALL default to `null`. For HTML-ingested entries, `original_text` SHALL hold the extracted plain text (the TTS pipeline input) and `html_source` SHALL hold the sanitized markup.

#### Scenario: New entry is persisted
- GIVEN an empty history
- WHEN a new entry is added
- THEN `history.json` contains a `version: 1` wrapper and the entry with status `"pending"`, a UUID v4 `id`, null audio fields, `format: null`, `generation: null`, and `generation_count: 0`

#### Scenario: Entry written with UTC timestamp
- GIVEN a newly ingested entry
- WHEN the entry is persisted to `history.json`
- THEN its `created_at` is the current UTC time in naive form (no timezone suffix), independent of the machine's local timezone

#### Scenario: History round-trips through disk
- GIVEN an entry with `normalized_text` set, `format` set to `"html"`, `html_source` set, and status `"ready"`
- WHEN the storage service is re-initialized against the same cache directory
- THEN the loaded entry has the same `normalized_text`, `format`, `html_source`, and field values as before the restart

#### Scenario: Older entries without optional fields parse
- GIVEN a `history.json` entry that lacks `format`, `html_source`, `normalized_text`, `audio_path`, `was_regenerated`, `generation`, `generation_count`, `source`, and other optional fields
- WHEN the history is loaded
- THEN the entry parses successfully with the missing fields defaulted (null / false / 0)

#### Scenario: Generation snapshot round-trips
- GIVEN a persisted entry whose `generation` snapshot is set
- WHEN the storage service is re-initialized against the same cache directory
- THEN the loaded entry has the same snapshot values, including engine, voice, sample rate, model identity, and audio facts

#### Scenario: Ingestion source annotation round-trips
- GIVEN an entry ingested from a file, persisted with `source: "file"`
- WHEN the storage service is re-initialized against the same cache directory
- THEN the loaded entry still has `source: "file"`

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
- An entry whose `audio_path` file is missing SHALL have its audio metadata (`audio_path`, `timestamps_path`, `duration_sec`, `audio_generated_at`, `generation`) cleared; if its status was `"ready"`, it SHALL be reset to `"pending"`.
- An entry whose `audio_path` file exists but whose status is not `"ready"` SHALL be set to `"ready"`.

#### Scenario: Interrupted synthesis is reset
- GIVEN a persisted entry with status `"processing"` and null `audio_path`
- WHEN the history is loaded
- THEN the entry status becomes `"pending"` and the corrected history is saved

#### Scenario: Ready entry with missing audio file
- GIVEN a persisted entry with status `"ready"` whose `audio_path` file does not exist in `audio/`
- WHEN the history is loaded
- THEN the entry status becomes `"pending"` and its audio metadata fields, including `generation`, are null

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
| Sample rate | One of 8 / 12 / 16 / 24 / 48 kHz — the rates libopus accepts natively (RFC 6716 §2). The TTS engine SHOULD write one of these; if it writes any other rate (e.g. a Piper voice at 22050 Hz, or 44100 Hz), the Rust side SHALL resample it to the nearest native rate before encoding. `OpusHead` SHALL record the native (resampled) rate the encoder actually used, not the original off-list rate |
| Bitrate | 32 000 bps (VOIP application) |
| Frame size | 20 ms |
| Pre-skip | Queried from `libopus`'s lookahead, scaled to 48 kHz output ticks |

The encoding pipeline is: the TTS engine (ttsd subprocess, Piper, or Silero Native) writes a mono WAV as either 32-bit-float PCM (ttsd, Piper) or 16-bit int PCM (Silero Native, matching upstream `save_wav`); the Rust side transcodes it to Opus and removes the source WAV. The transcode SHALL accept both formats, converting 16-bit int samples to float internally; any other sample format or bit width SHALL be rejected. If the WAV's sample rate is not one of the Opus-native rates, the Rust side SHALL resample it to the nearest native rate first. On encode failure the source `.wav` SHALL be left in place as a playback fallback. `save_audio` SHALL return the relative filename for `TextEntry.audio_path`.

#### Scenario: Saving audio returns the relative filename
- GIVEN an entry with id `550e8400-e29b-41d4-a716-446655440000`
- WHEN audio bytes are saved for the entry
- THEN the file `audio/550e8400-e29b-41d4-a716-446655440000.opus` exists and the returned filename is `550e8400-e29b-41d4-a716-446655440000.opus`

#### Scenario: Transcode failure keeps the WAV fallback
- GIVEN a synthesized `.wav` that fails Opus encoding
- WHEN the transcode step runs
- THEN the source `.wav` remains on disk so playback can still use it

#### Scenario: Piper clip is written as float WAV and transcodes to Opus
- GIVEN a synthesis produced by the Piper engine
- WHEN the clip is written to disk
- THEN the WAV is mono 32-bit-float PCM, so the transcode step accepts it, stores `.opus`, and removes the source `.wav`

#### Scenario: Silero Native 16-bit int WAV transcodes to Opus
- GIVEN a synthesis produced by the Silero Native engine (mono 16-bit int PCM at 24000 Hz)
- WHEN the Rust side transcodes it to Opus
- THEN the entry is stored as `.opus` (the source `.wav` is removed), `OpusHead` records 24000 Hz, and no "unsupported wav format" error is logged

#### Scenario: Off-list sample rate is resampled to the nearest native rate
- GIVEN a synthesized mono WAV at an off-list rate (e.g. 22050 Hz from a Piper voice)
- WHEN the Rust side transcodes it to Opus
- THEN the entry is stored as `.opus` (the source `.wav` is removed), `OpusHead` records 24000 Hz, and no "unsupported wav format" error is logged

#### Scenario: Native sample rate passes through without resampling
- GIVEN a synthesized mono WAV at a native rate (e.g. 48000 Hz)
- WHEN the Rust side transcodes it to Opus
- THEN the entry is stored as `.opus` and `OpusHead` records 48000 Hz, with no resampling step

#### Scenario: Other integer widths are still rejected
- GIVEN a synthesized mono WAV in a format outside the accepted set (e.g. 32-bit int PCM)
- WHEN the Rust side transcodes it to Opus
- THEN the transcode fails with an "unsupported wav format" error and the source `.wav` remains on disk

### Requirement: Legacy WAV to Opus Migration

On every app launch the system SHALL run a one-shot migration sweep over the loaded entries: any entry whose `audio_path` ends in `.wav` SHALL be transcoded to `.opus`, the entry's `audio_path` updated to the new filename, and the source `.wav` removed. The sweep SHALL be idempotent (already-`.opus` entries are not considered) and SHALL NOT abort on per-entry failures — encode errors and missing source files are logged and counted while the app keeps starting normally. Legacy `.wav` references in `history.json` SHALL continue to parse indefinitely. Off-list WAV rates (e.g. 22050 Hz Piper clips) SHALL be resampled to the nearest native rate during the transcode, not rejected.

#### Scenario: Legacy entry is migrated
- GIVEN an entry whose `audio_path` points at an existing `{uuid}.wav`
- WHEN the migration sweep runs
- THEN the entry's `audio_path` ends in `.opus`, the `.opus` file exists, and the source `.wav` is removed

#### Scenario: Legacy off-list-rate WAV is migrated
- GIVEN an entry whose `audio_path` points at an existing `{uuid}.wav` at an off-list rate (e.g. 22050 Hz)
- WHEN the migration sweep runs
- THEN the entry is migrated to `.opus` (resampled to the nearest native rate) and the source `.wav` is removed

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

The `UIConfig` field table SHALL gain:

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `language` | string | `"ru"` | UI language: `"ru"` / `"en"` |

Every existing defaulting/unknown-key/partial-update rule SHALL apply to the
new field unchanged.

#### Scenario: Older config without language key
- **GIVEN** a `config.json` written by a pre-localization build (no `language` key)
- **WHEN** the configuration is loaded
- **THEN** it parses successfully with `language` defaulted to `"ru"`

#### Scenario: Language round-trips
- **GIVEN** a configuration with `language` set to `"en"`
- **WHEN** the configuration is saved and loaded again
- **THEN** the loaded value is `"en"`

#### Scenario: Missing config returns defaults
- **GIVEN** no `config.json` in the cache directory
- **WHEN** the configuration is loaded
- **THEN** the default configuration is returned (`speaker` `"aidar"`, `sample_rate` `24000`, `engine` `"silero_native"`, `piper_voice` `"ruslan"`)

#### Scenario: Older config without engine keys
- **GIVEN** a `config.json` that contains only `speaker`, `sample_rate`, and `speech_rate`
- **WHEN** the configuration is loaded
- **THEN** it parses successfully with `engine` defaulted to `"silero_native"` and `piper_voice` defaulted to `"ruslan"`

#### Scenario: Config round-trips
- **GIVEN** a configuration with `speaker` `"xenia"` and `sample_rate` `48000`
- **WHEN** the configuration is saved and loaded again
- **THEN** the loaded values match the saved ones

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

The system SHALL support deleting only an entry's audio and timestamps files while keeping the text entry: the entry's `audio_path`, `timestamps_path`, `audio_generated_at`, `duration_sec`, and `generation` SHALL be cleared and its status reset to `"pending"`. `generation_count` and `was_regenerated` SHALL be preserved.

#### Scenario: Regeneration frees old audio
- GIVEN a ready entry with audio and timestamps files on disk
- WHEN audio-only deletion runs for that entry
- THEN the entry remains in history with status `"pending"` and null audio fields, `generation` is null, both files are removed

#### Scenario: Generation count survives audio deletion
- GIVEN a ready entry with `generation_count` set to 2
- WHEN audio-only deletion runs for that entry
- THEN the entry still has `generation_count` equal to 2 and `was_regenerated` unchanged

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

### Requirement: Atomic Conditional Update

The storage service SHALL provide a compare-and-set update operation over a
single entry: given an entry id, a predicate, and a mutation, it SHALL acquire
the entry map's write lock, evaluate the predicate against the current entry,
and — only when the predicate returns `true` — apply the mutation and persist,
all under that single lock hold. The predicate check and the mutation SHALL NOT
be separated by a release of the write lock.

The operation SHALL return `true` when the entry existed and the predicate
matched (the mutation was applied). It SHALL return `false` and change nothing
when the entry is absent or the predicate rejected it.

This operation SHALL be the mechanism used by status transitions that decide on
the basis of the entry's current status (entry cancellation, and the
stale-completion guards for synthesis ready/error), so a concurrent
read-decide-write cannot persist a stale entry clone over a transition that
already applied.

#### Scenario: predicate accepts applies the mutation
- GIVEN an entry whose status the predicate accepts
- WHEN the conditional update is invoked with a mutation
- THEN the mutation is applied, the history is persisted, and the operation returns `true`

#### Scenario: predicate rejects changes nothing
- GIVEN an entry whose status the predicate rejects
- WHEN the conditional update is invoked with a mutation
- THEN the entry is unchanged, the history is not modified for this update, and the operation returns `false`

#### Scenario: absent id is a no-op
- GIVEN no entry with the given id
- WHEN the conditional update is invoked
- THEN nothing is written and the operation returns `false`

#### Scenario: concurrent status transition cannot regress a completed entry
- GIVEN an entry mid-transition (e.g. `processing`) and two callers racing: one applies a completion that flips it to `ready`, another cancels it back to `pending`
- WHEN both run the conditional update under the predicate `status in {processing, pending}`
- THEN only one transition is applied, the entry ends in exactly one status, and no stale clone overwrites the applied transition

### Requirement: Legacy Cache Layout Migration

On startup the system SHALL migrate the legacy single-root layout from
`dirs::cache_dir()/ruvox` (Linux) into the two-root layout when the legacy directory
exists. Migration SHALL be per-item over `audio/`, `config.json`, and
`history.json`: an item moves only when its destination does not already exist,
making the migration idempotent and tolerant of partially completed earlier runs.
Items SHALL move in the order audio, then config, then history, so entry validation
on load never observes a moved `history.json` against not-yet-moved audio. Each move
SHALL prefer `rename` and fall back to copy-then-delete-source across filesystems.
After migration the system SHALL remove the legacy directory when it is empty.
Per-item failures SHALL be logged and SHALL NOT prevent startup.

#### Scenario: Legacy layout migrates on first launch
- GIVEN `~/.cache/ruvox/` containing `history.json`, `config.json`, and `audio/`, and no new-layout files
- WHEN the storage service is initialized with default roots
- THEN all three items exist under the new roots, are gone from the legacy directory, and the legacy directory is removed

#### Scenario: Migration is idempotent
- GIVEN the new layout is fully populated and the legacy directory is absent
- WHEN the storage service initializes repeatedly
- THEN no files move and no errors are logged

#### Scenario: Partial migration completes on next launch
- GIVEN a previous run moved `history.json` but left `audio/` and `config.json` in the legacy directory
- WHEN the storage service initializes
- THEN the remaining items move to their new roots and previously migrated items stay untouched

#### Scenario: Migration failure does not prevent startup
- GIVEN a legacy item that cannot be moved (e.g. permission denied)
- WHEN the storage service initializes
- THEN the failure is logged, the unmoved item stays in the legacy directory, and the app starts normally with whatever reached the new roots

### Requirement: Corrupted Config Recovery

If `config.json` cannot be parsed as JSON, the storage service SHALL rename it to
`config.json.bak` and return the default configuration. If `config.json` cannot be
read at all, the service SHALL log a warning and return the default configuration.

#### Scenario: Corrupted config falls back to defaults with backup
- GIVEN a `config.json` containing invalid JSON
- WHEN the configuration is loaded
- THEN the default configuration is returned, the original file is preserved as `config.json.bak`, and no `config.json` remains at its original path until the next save

#### Scenario: Unreadable config returns defaults
- GIVEN a `config.json` that fails to read at the IO level
- WHEN the configuration is loaded
- THEN the default configuration is returned and a warning is logged

### Requirement: Graceful Startup Failure

When the storage service cannot be opened at startup (per-user directories
unresolvable, permissions denied, I/O error), the application SHALL NOT panic or
abort. It SHALL log a structured error entry with the underlying cause to the
application log, show the user a native error dialog with an actionable
Russian-language message (including the log directory location), and exit the
process cleanly with a non-zero exit code.

#### Scenario: Storage open failure is graceful
- GIVEN the storage service cannot be opened during startup (e.g. the per-user data directory cannot be created)
- WHEN the application starts
- THEN no panic message is produced, an error entry with the cause is written to the application log, a native error dialog in Russian names the problem and points to the log directory, and after it is dismissed the process exits with a non-zero exit code
