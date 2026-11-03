# Delta: storage

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

type DisplayMode =
  | "rendered"    // Render the entry's source format in the viewer
  | "plain";      // Show the original text verbatim, markup included

type EntrySource =
  | "clipboard"
  | "file"
  | "url";

interface TextEntry {
  id: EntryId;
  original_text: string;
  normalized_text: string | null;       // Output of the Rust TTS pipeline
  status: EntryStatus;
  format: TextFormat | null;            // Immutable source format recorded at ingestion; null = not recorded
  display_mode: DisplayMode | null;     // Viewer display choice; null = never chosen (default view of the source format, verbatim for a plain source)
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
  engine: string;                        // Engine that produced the audio: "silero_native" | "piper" | "silero" | "teratts"
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

Status, format and display-mode values SHALL serialize as lowercase strings. `format` SHALL be written once at ingestion and SHALL NOT be rewritten afterwards; the viewer's display choice lives in `display_mode`. `created_at` and `audio_generated_at` SHALL be stored as naive timestamps without a timezone suffix; both SHALL be generated from the UTC clock (`Utc::now().naive_utc()`), and readers treat the values as UTC. `audio_path` and `timestamps_path` SHALL store the filename only; the full path resolves as `<cache_root>/audio/{filename}`. All optional `TextEntry` fields SHALL default when absent from the JSON, so entries written by older builds keep parsing; a missing `format` SHALL default to `null`, meaning the viewer derives the source format from the content, and a missing `display_mode` SHALL default to `null`, meaning the viewer shows the default view of the source format (verbatim for a plain source); a missing `html_source` SHALL default to `null`, a missing `generation` SHALL default to `null`, a missing `generation_count` SHALL default to `0`, and a missing `source` SHALL default to `null`. Snapshots written by earlier builds may carry a `read_operators` field; readers MUST ignore it, and re-saving the entry drops it. For HTML-ingested entries, `original_text` SHALL hold the extracted plain text (the TTS pipeline input) and `html_source` SHALL hold the sanitized markup.

#### Scenario: New entry is persisted
- GIVEN an empty history
- WHEN a new entry is added through the storage service without a source format
- THEN `history.json` contains a `version: 1` wrapper and the entry with status `"pending"`, a UUID v4 `id`, null audio fields, `format: null`, `display_mode: null`, `generation: null`, and `generation_count: 0`

#### Scenario: Entry written with UTC timestamp
- GIVEN a newly ingested entry
- WHEN the entry is persisted to `history.json`
- THEN its `created_at` is the current UTC time in naive form (no timezone suffix), independent of the machine's local timezone

#### Scenario: History round-trips through disk
- GIVEN an entry with `normalized_text` set, `format` set to `"html"`, `display_mode` set to `"plain"`, `html_source` set, and status `"ready"`
- WHEN the storage service is re-initialized against the same cache directory
- THEN the loaded entry has the same `normalized_text`, `format`, `display_mode`, `html_source`, and field values as before the restart

#### Scenario: Display mode is stored separately from the source format
- GIVEN an entry persisted with `format: "markdown"` and `display_mode: "plain"`
- WHEN the storage service is re-initialized against the same cache directory
- THEN the loaded entry keeps `format: "markdown"` and `display_mode: "plain"`

#### Scenario: Older entries without optional fields parse
- GIVEN a `history.json` entry that lacks `format`, `display_mode`, `html_source`, `normalized_text`, `audio_path`, `was_regenerated`, `generation`, `generation_count`, `source`, and other optional fields
- WHEN the history is loaded
- THEN the entry parses successfully with the missing fields defaulted (null / false / 0), including `display_mode: null`

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

### Requirement: Backwards Compatibility

The on-disk format originated in the earlier PyQt implementation of RuVox, and
existing user files SHALL keep working without migration: fields added later
use serde defaults so older JSON parses cleanly, and fields that no longer
exist in the current schema SHALL be silently ignored on read.

Entries written before `display_mode` existed SHALL load with
`display_mode: null`; no migration SHALL rewrite the stored entries. Their
allowed views and effective display mode follow the source-format
derivation and render-time clamping defined by the text-display spec,
"Display mode switching"; the storage layer only defaults the missing
field to `null`.

#### Scenario: PyQt-era history file loads
- GIVEN a `history.json` written by the pre-Tauri build (including a legacy `.wav` `audio_path` and no `was_regenerated` field)
- WHEN the storage service is initialized
- THEN the entries load successfully with missing fields defaulted

#### Scenario: Unknown config keys are ignored
- GIVEN a `config.json` containing keys not present in the current `UIConfig` schema
- WHEN the configuration is loaded
- THEN it parses successfully and the unknown keys are dropped
