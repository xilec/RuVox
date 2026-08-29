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
