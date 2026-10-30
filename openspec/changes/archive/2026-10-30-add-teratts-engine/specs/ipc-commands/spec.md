# Delta: ipc-commands

## ADDED Requirements

### Requirement: TeraTTS bundle download command

The system SHALL provide `download_teratts_bundle()` which downloads the
TeraTTSv2 model bundle from the project's `teratts-models-v1` GitHub Release
into the app data dir, following the bundle `manifest.json`: for every entry it
downloads the `asset` URL into the bundle-relative `path`, verifies the size
and sha256 while streaming to a `.partial` file, and renames the file into
place only after verification. Files already present with a matching size and
sha256 SHALL be skipped (idempotent re-runs), and an interrupted `.partial`
file SHALL be resumed rather than restarted. A checksum mismatch or any I/O
failure MUST abort the download with a typed error, remove the invalid
`.partial` file, and leave the engine unavailable; a partial file MUST NOT be
treated as installed. The command SHALL emit the `bundle_download_*` events
with `engine: "teratts"`.

#### Scenario: Fresh download succeeds

- GIVEN no bundle in the data dir and network access to GitHub Releases
- WHEN `download_teratts_bundle` is invoked
- THEN every manifest file lands at its `path`, the checksums verify, and a
  subsequent `get_available_engines` reports `teratts.available: true`

#### Scenario: Re-run skips valid files

- GIVEN a fully downloaded bundle
- WHEN `download_teratts_bundle` is invoked again
- THEN every file is reported as skipped and no bytes are downloaded

#### Scenario: Interrupted download resumes

- GIVEN a download interrupted after a `.partial` file was written
- WHEN the command runs again
- THEN it continues from the partial file's length and verifies the final
  checksum

#### Scenario: Checksum failure aborts

- GIVEN a download where one file's sha256 does not match the manifest
- WHEN verification runs
- THEN the command fails with a typed error, the invalid file is removed, and
  `teratts.available` stays `false`

## MODIFIED Requirements

### Requirement: Shared IPC Types

Commands and events SHALL exchange `TextEntry`, `WordTimestamp`, and `UIConfig`
using the exact JSON field names of the storage schema
(`src-tauri/src/storage/schema.rs`), serialized as follows:

```typescript
type EntryId = string; // UUID, lowercase hyphenated

type EntryStatus =
  | "pending" | "processing" | "ready" | "playing" | "error"; // lowercase

type TextFormat = "plain" | "markdown" | "html"; // lowercase

interface TextEntry {
  id: EntryId;
  original_text: string;
  normalized_text: string | null;
  status: EntryStatus;
  format: TextFormat | null;         // null = never chosen; viewer uses the config default
  html_source: string | null;        // sanitized HTML for rendering; HTML-ingested entries only
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
  speaker: string;                 // Silero speaker, default "aidar"
  sample_rate: number;             // default 24000 (the native engine's own default)
  speech_rate: number;             // playback speed multiplier, default 1.0
  notify_on_ready: boolean;
  notify_on_error: boolean;
  text_format: string;             // "plain" | "markdown" | "html"
  max_cache_size_mb: number;       // default 500
  code_block_mode: string;         // "brief" (default) | "read"
  theme: string;                   // "light" | "dark" | "auto"
  player_hotkeys: Record<string, string>;
  window_geometry: [number, number, number, number] | null;
  preview_dialog_enabled: boolean;
  hide_to_tray_on_close: boolean;  // default false: closing the window quits
  engine: string;                  // "piper" | "silero" | "silero_native" (default) | "teratts"
  piper_voice: string;             // default "ruslan"
  teratts_voice: string;           // default "ru_m5"
}
```

#### Scenario: TextEntry round-trips through get_entries
- GIVEN an entry persisted in storage with `format` and `html_source` set
- WHEN the frontend calls `invoke("get_entries")`
- THEN each entry serializes with the field names above, `status` and `format` as lowercase strings or null, and `html_source` as a string or null

#### Scenario: UIConfig defaults include engine fields
- GIVEN a fresh installation with no `config.json`
- WHEN the frontend calls `invoke("get_config")`
- THEN the response contains `engine: "silero_native"`, `piper_voice: "ruslan"` and `teratts_voice: "ru_m5"` alongside the legacy fields

#### Scenario: UIConfig defaults use brief code block mode
- GIVEN a fresh installation with no `config.json`
- WHEN the frontend calls `invoke("get_config")`
- THEN the response contains `code_block_mode: "brief"` and no `read_operators` key

#### Scenario: UIConfig defaults keep tray-on-close off

- GIVEN a fresh installation with no `config.json`, or an existing
  `config.json` written before `hide_to_tray_on_close` existed
- WHEN the frontend calls `invoke("get_config")`
- THEN the response contains `hide_to_tray_on_close: false`

### Requirement: Engine Availability Command

The system SHALL provide `get_available_engines()` returning per-engine
availability (`AvailableEngines`):

```typescript
interface LocalizedText {
  code: string;        // machine-readable reason id, e.g. "silero.uv_missing"
  params?: string[];   // positional interpolation values
  message?: string;    // optional raw diagnostic detail
}
interface EngineAvailability {
  available: boolean;
  reason: LocalizedText | null; // Some only when available == false
}
interface AvailableEngines {
  piper: EngineAvailability;
  silero: EngineAvailability;
  silero_native: EngineAvailability;
  teratts: EngineAvailability;
}
```

Piper (in-process) SHALL always report `available: true`. Silero SHALL report
availability based on a cheap probe: presence of `pyproject.toml` in the ttsd
directory and a successful `uv --version` exec; a `uv` binary that cannot be
spawned at all (not installed — the normal case on Windows, where ttsd is
not shipped) SHALL be treated as an unsuccessful probe, not an error.
Silero Native SHALL report availability based on presence and manifest
validity of the downloaded model bundle in the app data dir. TeraTTS SHALL
report availability based on a stat-only probe of its bundle: the manifest
parses, its `format_version` is supported, and every listed file exists with
the recorded size (no hashing). When unavailable, `reason` SHALL carry a
machine-readable code (translated by the frontend like command errors), not
user-facing prose.

#### Scenario: probe on a system without ttsd
- **GIVEN** no `pyproject.toml` in the resolved ttsd directory
- **WHEN** `get_available_engines` is invoked
- **THEN** `silero.available` is `false` with `reason.code` `"silero.ttsd_missing"`, and `piper.available` is `true`

#### Scenario: probe when uv cannot be spawned
- **GIVEN** a `pyproject.toml` exists in the resolved ttsd directory but the
  `uv` binary is not installed (spawn fails)
- **WHEN** `get_available_engines` is invoked
- **THEN** `silero.available` is `false` with `reason.code` `"silero.uv_missing"`, the command
  succeeds, and `piper.available` is `true`

#### Scenario: native engine unavailable before bundle download
- **GIVEN** no model bundle in the app data dir
- **WHEN** `get_available_engines` is invoked
- **THEN** `silero_native.available` is `false` with `reason.code`
  `"native.bundle_missing"` indicating that the model bundle must be downloaded

#### Scenario: TeraTTS unavailable before bundle download
- **GIVEN** no TeraTTS bundle in the app data dir
- **WHEN** `get_available_engines` is invoked
- **THEN** `teratts.available` is `false` with `reason.code`
  `"teratts.bundle_missing"` indicating that the model bundle must be downloaded

#### Scenario: TeraTTS unavailable with an incompatible bundle
- **GIVEN** a TeraTTS bundle whose manifest declares an unsupported
  `format_version`
- **WHEN** `get_available_engines` is invoked
- **THEN** `teratts.available` is `false` with `reason.code`
  `"teratts.bundle_incompatible"`

#### Scenario: TeraTTS available with a complete bundle
- **GIVEN** a TeraTTS bundle whose manifest parses and whose files match the
  recorded sizes
- **WHEN** `get_available_engines` is invoked
- **THEN** `teratts.available` is `true` and no file is hashed

### Requirement: Bundle Download Events

During `download_silero_native_bundle` or `download_teratts_bundle` the backend
SHALL emit:

- `bundle_download_started` — `{ engine: "silero_native" | "teratts" }`
- `bundle_download_progress` — `{ engine, file, file_idx, total_files, downloaded_bytes, total_bytes }`, throttled to roughly one event per 256 KB, plus `skipped: true` for files already present and valid
- `bundle_download_finished` — `{ engine, ok: true }` on success or `{ engine, ok: false, message }` on failure

#### Scenario: download progress reporting
- GIVEN a bundle that is not installed
- WHEN `download_silero_native_bundle` runs
- THEN `bundle_download_started` fires first, progress events carry cumulative
  byte counts per file, and a terminal `bundle_download_finished` completes
  the sequence

#### Scenario: TeraTTS events carry the engine id
- GIVEN a TeraTTS bundle that is not installed
- WHEN `download_teratts_bundle` runs
- THEN every `bundle_download_*` event carries `engine: "teratts"`

### Requirement: Generation Parameters Snapshot

On every successful synthesis the system SHALL record a `generation` snapshot on the entry describing the parameters that produced the current audio, and SHALL increment the entry's `generation_count` by one. The snapshot SHALL contain:

- the engine that actually served the request (`silero_native` | `piper` | `silero` | `teratts`) — the engine resolved at synthesis time, not the persisted config preference;
- the resolved engine-specific voice id actually passed to synthesis (Piper uses `piper_voice`, both Silero engines use `speaker`, TeraTTS uses `teratts_voice`);
- the actual output sample rate of the rendered audio (for Piper this is the rate fixed by the voice model; for TeraTTS it is 44100 Hz);
- the model identity where the engine can report it cheaply — silero-native: the bundle manifest `model_id` with the `tts_main.onnx` sha256; TeraTTS: the manifest `model_id` with the `bundle_version` and the manifest sha256; Piper: the voice model file name from the catalog; ttsd: no model identity yet, the field stays null;
- the application version at generation time;
- the code block narration mode actually applied to this synthesis (`code_block_mode`: `"brief"` | `"read"`);
- the sha256 of the normalized text used;
- the stored audio file's codec (from the final file: Ogg Opus, or WAV fallback) and size in bytes.

Regenerating an entry SHALL overwrite the snapshot and increment `generation_count`. Clearing an entry's audio (audio-only deletion, or load-time validation when the audio file is missing) SHALL clear the snapshot together with the other audio metadata. Entries synthesized by older builds have no snapshot; payloads carry `generation: null` and the commands MUST NOT fail. Snapshots written by earlier builds may carry a `read_operators` field; readers MUST ignore it.

The snapshot SHALL flow to the frontend as part of entry payloads (`get_entries`, `get_entry`, `entry_updated` events).

#### Scenario: Snapshot recorded on first synthesis
- GIVEN an entry is synthesized successfully on the silero-native engine with speaker `xenia`
- WHEN the synthesis completes
- THEN the entry has `generation_count` equal to 1 and a snapshot with engine `silero_native`, voice `xenia`, a non-null sample rate, the app version, and non-null audio codec and size

#### Scenario: Regeneration refreshes the snapshot
- GIVEN a ready entry with `generation_count` equal to 1 and a snapshot
- WHEN the entry is regenerated successfully with a different voice selected
- THEN the snapshot's voice equals the new voice and `generation_count` equals 2

#### Scenario: Snapshot records the code block mode in effect
- GIVEN the config has `code_block_mode: "brief"` and an entry with a fenced code block is synthesized
- WHEN the synthesis completes
- THEN the snapshot's `code_block_mode` is `"brief"`

#### Scenario: Snapshot survives while text stays, cleared with audio
- GIVEN a ready entry with a snapshot
- WHEN audio-only deletion runs for the entry
- THEN the entry's `generation` is null while `generation_count` keeps its value

#### Scenario: Legacy entries carry no snapshot
- GIVEN a `history.json` written by a build older than this feature, containing a ready entry with audio
- WHEN the frontend invokes `get_entries`
- THEN the entry is returned with `generation: null` and `generation_count: 0`, and no error is raised

#### Scenario: Legacy snapshot with read_operators parses
- GIVEN a `history.json` whose ready entry carries a snapshot with a `read_operators` field
- WHEN the frontend invokes `get_entries`
- THEN the entry is returned with its snapshot intact minus `read_operators`, and no error is raised

#### Scenario: TeraTTS snapshot records its model identity
- GIVEN an entry synthesized on the TeraTTS engine with `teratts_voice: "ru_m5"`
- WHEN the synthesis completes
- THEN the snapshot has engine `"teratts"`, voice `"ru_m5"`, sample rate `44100`, and a model identity naming the bundle `model_id` and `bundle_version`
