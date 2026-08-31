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
  engine: string;                  // "piper" | "silero" | "silero_native" (default)
  piper_voice: string;             // default "ruslan"
}
```

#### Scenario: TextEntry round-trips through get_entries
- GIVEN an entry persisted in storage with `format` and `html_source` set
- WHEN the frontend calls `invoke("get_entries")`
- THEN each entry serializes with the field names above, `status` and `format` as lowercase strings or null, and `html_source` as a string or null

#### Scenario: UIConfig defaults include engine fields
- GIVEN a fresh installation with no `config.json`
- WHEN the frontend calls `invoke("get_config")`
- THEN the response contains `engine: "silero_native"` and `piper_voice: "ruslan"` alongside the legacy fields

#### Scenario: UIConfig defaults use brief code block mode
- GIVEN a fresh installation with no `config.json`
- WHEN the frontend calls `invoke("get_config")`
- THEN the response contains `code_block_mode: "brief"` and no `read_operators` key

### Requirement: Configuration Commands

The system SHALL provide `get_config()` returning the current `UIConfig`, and
`update_config(patch)` merging a partial `UIConfigPatch` (any subset of the
`UIConfig` fields; omitted fields keep their values) into the current config.
Before persisting, `update_config` SHALL apply the requested TTS engine through
the engine switcher; if the engine cannot be activated (e.g. Silero stack not
spawnable) the command SHALL fail with `config_error` and the previous config
MUST remain on disk.

When the patch changes `code_block_mode`, `update_config` SHALL apply the new
code block narration mode to the shared normalization pipeline immediately
after persisting: subsequent synthesis and preview normalization requests
SHALL use the new mode without an app restart. A synthesis already in flight
when the setting changes MAY finish on the previous mode.

#### Scenario: partial patch updates only named fields
- GIVEN a stored config with `theme: "auto"`
- WHEN `update_config` is invoked with `{ theme: "dark" }`
- THEN only `theme` changes, all other fields keep their values, and the config is persisted

#### Scenario: engine switch failure preserves the old config
- GIVEN the Silero engine is unavailable on the system
- WHEN `update_config` is invoked with `{ engine: "silero" }`
- THEN the command fails with `type: "config_error"` and the on-disk config still has the previous engine

#### Scenario: code block mode change applies without restart
- GIVEN the pipeline is narrating code blocks in brief mode
- WHEN `update_config` is invoked with `{ code_block_mode: "read" }`
- THEN a subsequent `preview_normalize` call reads fenced code blocks in full mode, without restarting the app

### Requirement: Generation Parameters Snapshot

On every successful synthesis the system SHALL record a `generation` snapshot on the entry describing the parameters that produced the current audio, and SHALL increment the entry's `generation_count` by one. The snapshot SHALL contain:

- the engine that actually served the request (`silero_native` | `piper` | `silero`) — the engine resolved at synthesis time, not the persisted config preference;
- the resolved engine-specific voice id actually passed to synthesis (Piper uses `piper_voice`, both Silero engines use `speaker`);
- the actual output sample rate of the rendered audio (for Piper this is the rate fixed by the voice model);
- the model identity where the engine can report it cheaply — silero-native: the bundle manifest `model_id` with the `tts_main.onnx` sha256; Piper: the voice model file name from the catalog; ttsd: no model identity yet, the field stays null;
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
