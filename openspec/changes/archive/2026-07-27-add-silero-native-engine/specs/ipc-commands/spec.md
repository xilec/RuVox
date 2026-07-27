# Delta spec: ipc-commands

## MODIFIED Requirements

### Requirement: Shared IPC Types

Commands and events SHALL exchange `TextEntry`, `WordTimestamp`, and `UIConfig`
using the exact JSON field names of the storage schema
(`src-tauri/src/storage/schema.rs`), serialized as follows:

```typescript
type EntryId = string; // UUID, lowercase hyphenated

type EntryStatus =
  | "pending" | "processing" | "ready" | "playing" | "error"; // lowercase

interface TextEntry {
  id: EntryId;
  original_text: string;
  normalized_text: string | null;
  status: EntryStatus;
  created_at: string;              // naive datetime, e.g. "2026-02-15T11:46:51.504055"
  audio_path: string | null;       // filename relative to the cache audio dir
  timestamps_path: string | null;
  duration_sec: number | null;
  audio_generated_at: string | null;
  was_regenerated: boolean;
  error_message: string | null;
}

interface WordTimestamp {
  word: string;
  start: number;                   // seconds
  end: number;
  original_pos: [number, number];  // [start, end] char offsets in original_text
}

interface UIConfig {
  speaker: string;                 // Silero speaker, default "xenia"
  sample_rate: number;             // default 48000; native engine defaults to 24000
  speech_rate: number;             // playback speed multiplier, default 1.0
  notify_on_ready: boolean;
  notify_on_error: boolean;
  text_format: string;             // "plain" | "markdown" | "html"
  max_cache_size_mb: number;       // default 500
  code_block_mode: string;         // "skip" | "read"
  read_operators: boolean;
  theme: string;                   // "light" | "dark" | "auto"
  player_hotkeys: Record<string, string>;
  window_geometry: [number, number, number, number] | null;
  preview_dialog_enabled: boolean;
  engine: string;                  // "piper" (default) | "silero" | "silero_native"
  piper_voice: string;             // default "ruslan"
}
```

#### Scenario: TextEntry round-trips through get_entries
- GIVEN an entry persisted in storage
- WHEN the frontend calls `invoke("get_entries")`
- THEN each entry serializes with the field names above and `status` as a lowercase string

#### Scenario: UIConfig defaults include engine fields
- GIVEN a fresh installation with no `config.json`
- WHEN the frontend calls `invoke("get_config")`
- THEN the response contains `engine: "piper"` and `piper_voice: "ruslan"` alongside the legacy fields

### Requirement: Engine Availability Command

The system SHALL provide `get_available_engines()` returning per-engine
availability (`AvailableEngines`):

```typescript
interface EngineAvailability { available: boolean; reason: string | null }
interface AvailableEngines {
  piper: EngineAvailability;
  silero: EngineAvailability;
  silero_native: EngineAvailability;
}
```

Piper (in-process) SHALL always report `available: true`. Silero SHALL report
availability based on a cheap probe: presence of `pyproject.toml` in the ttsd
directory and a successful `uv --version` exec. Silero Native SHALL report
availability based on presence and manifest validity of the downloaded model
bundle in the app data dir. When unavailable, `reason` SHALL be a
Russian-language user-facing string.

#### Scenario: probe on a system without ttsd
- GIVEN no `pyproject.toml` in the resolved ttsd directory
- WHEN `get_available_engines` is invoked
- THEN `silero.available` is `false` with a Russian `reason`, and `piper.available` is `true`

#### Scenario: native engine unavailable before bundle download
- GIVEN no model bundle in the app data dir
- WHEN `get_available_engines` is invoked
- THEN `silero_native.available` is `false` with a Russian `reason` explaining
  that the model bundle must be downloaded

## ADDED Requirements

### Requirement: Silero Native Bundle Download Command

The system SHALL provide `download_silero_native_bundle()` which downloads the
model bundle files from the project's GitHub Releases into the app data dir,
verifying each file's sha256 against the bundle manifest, skipping files
already present and valid (idempotent). A failed checksum MUST abort the
download with a typed error and leave the engine unavailable; partial files
MUST NOT be treated as installed on the next run.

#### Scenario: fresh download succeeds

- GIVEN no bundle in the data dir and network access to GitHub Releases
- WHEN `download_silero_native_bundle` is invoked
- THEN all bundle files land in the data dir, checksums verify, and a
  subsequent `get_available_engines` reports `silero_native.available: true`

#### Scenario: checksum failure aborts

- GIVEN a download where one file's sha256 does not match the manifest
- WHEN the verification step runs
- THEN the command fails with a typed error, the invalid file is removed or
  quarantined, and `silero_native.available` stays `false`

### Requirement: Bundle Download Events

During `download_silero_native_bundle` the backend SHALL emit:

- `bundle_download_started` — `{ engine: "silero_native" }`
- `bundle_download_progress` — `{ engine, file, file_idx, total_files, downloaded_bytes, total_bytes }`, throttled to roughly one event per 256 KB, plus `skipped: true` for files already present and valid
- `bundle_download_finished` — `{ engine, ok: true }` on success or `{ engine, ok: false, message }` on failure

#### Scenario: download progress reporting

- GIVEN a bundle that is not installed
- WHEN `download_silero_native_bundle` runs
- THEN `bundle_download_started` fires first, progress events carry cumulative
  byte counts per file, and a terminal `bundle_download_finished` completes
  the sequence
