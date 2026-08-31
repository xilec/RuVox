## MODIFIED Requirements

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
  code_block_mode: string | null;        // Code block narration mode actually applied: "brief" | "read"
  normalized_text_sha256: string | null; // sha256 of the normalized text used for this audio
  audio_codec: string | null;            // Codec of the stored file ("Ogg Opus" | "WAV")
  audio_bytes: number | null;            // Size of the stored audio file in bytes
}

interface ModelParams {
  name: string;           // e.g. silero-native bundle "model_id" or Piper voice model file name
  sha256: string | null;  // Checksum where cheaply available (silero-native bundle manifest)
}
```

Status and format values SHALL serialize as lowercase strings. `created_at` and `audio_generated_at` SHALL be stored as naive timestamps without a timezone suffix; both SHALL be generated from the UTC clock (`Utc::now().naive_utc()`), and readers treat the values as UTC. `audio_path` and `timestamps_path` SHALL store the filename only; the full path resolves as `<cache_root>/audio/{filename}`. All optional `TextEntry` fields SHALL default when absent from the JSON, so entries written by older builds keep parsing; a missing `format` SHALL default to `null`, meaning the viewer falls back to the `text_format` config default, a missing `html_source` SHALL default to `null`, a missing `generation` SHALL default to `null`, a missing `generation_count` SHALL default to `0`, and a missing `source` SHALL default to `null`. Snapshots written by earlier builds may carry a `read_operators` field; readers MUST ignore it, and re-saving the entry drops it. For HTML-ingested entries, `original_text` SHALL hold the extracted plain text (the TTS pipeline input) and `html_source` SHALL hold the sanitized markup.

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

#### Scenario: Legacy snapshot with read_operators parses
- GIVEN a `history.json` written by an earlier build whose `generation` snapshot contains `read_operators: true`
- WHEN the history is loaded
- THEN the entry parses successfully, the snapshot keeps its `code_block_mode` and other fields, and `read_operators` is not surfaced; re-saving the entry persists the snapshot without `read_operators`

#### Scenario: Ingestion source annotation round-trips
- GIVEN an entry ingested from a file, persisted with `source: "file"`
- WHEN the storage service is re-initialized against the same cache directory
- THEN the loaded entry still has `source: "file"`

### Requirement: Config File Schema

The `UIConfig` field table SHALL gain:

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `language` | string | `"ru"` | UI language: `"ru"` / `"en"` |

Every existing defaulting/unknown-key/partial-update rule SHALL apply to the
new field unchanged.

The `code_block_mode` field SHALL accept the values `"brief"` and `"read"`,
with `"brief"` as the default for fresh and partial configs. The legacy value
`"skip"` SHALL be accepted on load as an alias for `"brief"`; any other value
SHALL fall back to `"brief"`. The `read_operators` field SHALL NOT be part of
`UIConfig`; config files written by earlier builds that contain it MUST still
parse (the unknown key is ignored and dropped on the next save).

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
- **THEN** the default configuration is returned (`speaker` `"aidar"`, `sample_rate` `24000`, `engine` `"silero_native"`, `piper_voice` `"ruslan"`, `code_block_mode` `"brief"`)

#### Scenario: Older config without engine keys
- **GIVEN** a `config.json` that contains only `speaker`, `sample_rate`, and `speech_rate`
- **WHEN** the configuration is loaded
- **THEN** it parses successfully with `engine` defaulted to `"silero_native"` and `piper_voice` defaulted to `"ruslan"`

#### Scenario: Config round-trips
- **GIVEN** a configuration with `speaker` `"xenia"` and `sample_rate` `48000`
- **WHEN** the configuration is saved and loaded again
- **THEN** the loaded values match the saved ones

#### Scenario: Older config without code_block_mode key
- **GIVEN** a `config.json` written by an earlier build with no `code_block_mode` key
- **WHEN** the configuration is loaded
- **THEN** it parses successfully with `code_block_mode` defaulted to `"brief"`

#### Scenario: Persisted read value stays read
- **GIVEN** a `config.json` with `code_block_mode: "read"`
- **WHEN** the configuration is loaded and saved again
- **THEN** the value stays `"read"` (no migration or coercion)

#### Scenario: Legacy skip value is aliased to brief
- **GIVEN** a `config.json` with `code_block_mode: "skip"`
- **WHEN** the configuration is loaded
- **THEN** `code_block_mode` resolves to `"brief"`

#### Scenario: Unknown code_block_mode value falls back to brief
- **GIVEN** a `config.json` with `code_block_mode: "loud"`
- **WHEN** the configuration is loaded
- **THEN** `code_block_mode` resolves to `"brief"`

#### Scenario: Config with read_operators parses
- **GIVEN** a `config.json` written by an earlier build containing `read_operators: false`
- **WHEN** the configuration is loaded
- **THEN** it parses successfully, all other fields keep their values, and saving drops the `read_operators` key
