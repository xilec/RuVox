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

interface TextEntry {
  id: EntryId;
  original_text: string;
  normalized_text: string | null;       // Output of the Rust TTS pipeline
  status: EntryStatus;
  format: TextFormat | null;            // Display format chosen for this entry; null = never chosen
  created_at: string;                   // Naive timestamp, e.g. "2026-02-15T11:46:51.504055" (no TZ suffix)
  audio_generated_at: string | null;    // Naive timestamp when audio file was written
  audio_path: string | null;            // Filename relative to audio/, e.g. "{uuid}.opus"
  timestamps_path: string | null;       // Filename relative to audio/, e.g. "{uuid}.timestamps.json"
  duration_sec: number | null;          // Audio duration in seconds
  was_regenerated: boolean;             // True if audio was re-synthesized at least once
  error_message: string | null;         // Human-readable error if status == "error"
}
```

Status and format values SHALL serialize as lowercase strings. `created_at` and `audio_generated_at` SHALL be stored as naive timestamps without a timezone suffix; `created_at` is generated from the local clock (`Local::now().naive_local()`) and readers treat the values as UTC. `audio_path` and `timestamps_path` SHALL store the filename only; the full path resolves as `<cache_root>/audio/{filename}`. All optional `TextEntry` fields SHALL default when absent from the JSON, so entries written by older builds keep parsing; a missing `format` SHALL default to `null`, meaning the viewer falls back to the `text_format` config default.

#### Scenario: New entry is persisted
- GIVEN an empty history
- WHEN a new entry is added
- THEN `history.json` contains a `version: 1` wrapper and the entry with status `"pending"`, a UUID v4 `id`, null audio fields, and `format: null`

#### Scenario: History round-trips through disk
- GIVEN an entry with `normalized_text` set, `format` set to `"html"`, and status `"ready"`
- WHEN the storage service is re-initialized against the same cache directory
- THEN the loaded entry has the same `normalized_text`, `format: "html"`, and field values as before the restart

#### Scenario: Older entries without optional fields parse
- GIVEN a `history.json` entry that lacks `format`, `normalized_text`, `audio_path`, `was_regenerated`, and other optional fields
- WHEN the history is loaded
- THEN the entry parses successfully with the missing fields defaulted (null / false)
