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
  english_reading: string;         // "auto" (default) | "native" | "narrated"
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

#### Scenario: UIConfig defaults include auto English reading

- GIVEN a fresh installation with no `config.json`, or an existing
  `config.json` written before `english_reading` existed
- WHEN the frontend calls `invoke("get_config")`
- THEN the response contains `english_reading: "auto"`, and the value is
  canonicalized to `"auto"` when `config.json` holds anything else

#### Scenario: UIConfig defaults keep tray-on-close off

- GIVEN a fresh installation with no `config.json`, or an existing
  `config.json` written before `hide_to_tray_on_close` existed
- WHEN the frontend calls `invoke("get_config")`
- THEN the response contains `hide_to_tray_on_close: false`

### Requirement: Configuration Commands

The system SHALL provide `get_config()` returning the current `UIConfig`, and
`update_config(patch)` merging a partial `UIConfigPatch` (any subset of the
`UIConfig` fields; omitted fields keep their values) into the current config.
Before persisting, `update_config` SHALL apply the requested TTS engine through
the engine switcher; if the engine cannot be activated (e.g. Silero stack not
spawnable) the command SHALL fail with `config_error` and the previous config
MUST remain on disk.

When the patch changes `code_block_mode` or `english_reading`,
`update_config` SHALL apply the new text-shaping values to the shared
normalization pipeline immediately after persisting: subsequent synthesis and
preview normalization requests SHALL use the new values without an app
restart. A synthesis already in flight when the settings change MAY finish on
the previous values.

#### Scenario: partial patch updates only named fields
- GIVEN a stored config with `theme: "auto"`
- WHEN `update_config` is invoked with `{ theme: "dark" }`
- THEN only `theme` changes, all other fields keep their values, and the config is persisted

#### Scenario: engine switch failure preserves the old config
- GIVEN the Silero engine is unavailable on the system
- WHEN `update_config` is invoked with `{ engine: "silero" }`
- THEN the command fails with `type: "config_error"` and the on-disk config still has the previous engine

#### Scenario: English reading change applies without restart

- GIVEN the pipeline narrates Latin prose
- WHEN `update_config` is invoked with `{ english_reading: "native" }`
- THEN the shared pipeline holds the new preference for the next synthesis,
  and a subsequent `preview_normalize` call resolves Latin prose against it
  without restarting the app

#### Scenario: code block mode change applies without restart
- GIVEN the pipeline is narrating code blocks in brief mode
- WHEN `update_config` is invoked with `{ code_block_mode: "read" }`
- THEN a subsequent `preview_normalize` call reads fenced code blocks in full mode, without restarting the app

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
  reads_latin_natively: boolean; // the profile's TextProfile.reads_latin_natively
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

`reads_latin_natively` SHALL be stamped from the engine's `TextProfile` in the
same probe, not from a second per-engine table, so the UI reads one source of
truth for whether an engine can read English prose in English.

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

#### Scenario: Latin capability follows the engine profile

- GIVEN any system state
- WHEN `get_available_engines` is invoked
- THEN `teratts.reads_latin_natively` is `true` and `piper`, `silero`,
  `silero_native` report `false`, matching the profiles regardless of whether
  each engine is currently available

#### Scenario: TeraTTS available with a complete bundle
- **GIVEN** a TeraTTS bundle whose manifest parses and whose files match the
  recorded sizes
- **WHEN** `get_available_engines` is invoked
- **THEN** `teratts.available` is `true` and no file is hashed
