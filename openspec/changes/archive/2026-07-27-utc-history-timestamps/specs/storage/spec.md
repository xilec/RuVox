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

interface TextEntry {
  id: EntryId;
  original_text: string;
  normalized_text: string | null;       // Output of the Rust TTS pipeline
  status: EntryStatus;
  created_at: string;                   // Naive UTC timestamp, e.g. "2026-02-15T11:46:51.504055" (no TZ suffix)
  audio_generated_at: string | null;    // Naive UTC timestamp when audio file was written
  audio_path: string | null;            // Filename relative to audio/, e.g. "{uuid}.opus"
  timestamps_path: string | null;       // Filename relative to audio/, e.g. "{uuid}.timestamps.json"
  duration_sec: number | null;          // Audio duration in seconds
  was_regenerated: boolean;             // True if audio was re-synthesized at least once
  error_message: string | null;         // Human-readable error if status == "error"
}
```

Status values SHALL serialize as lowercase strings. `created_at` and `audio_generated_at` SHALL be stored as naive timestamps without a timezone suffix; both SHALL be generated from the UTC clock (`Utc::now().naive_utc()`), and readers treat the values as UTC. `audio_path` and `timestamps_path` SHALL store the filename only; the full path resolves as `<cache_root>/audio/{filename}`. All optional `TextEntry` fields SHALL default when absent from the JSON, so entries written by older builds keep parsing.

#### Scenario: New entry is persisted
- GIVEN an empty history
- WHEN a new entry is added
- THEN `history.json` contains a `version: 1` wrapper and the entry with status `"pending"`, a UUID v4 `id`, and null audio fields

#### Scenario: Entry written with UTC timestamp
- GIVEN a newly ingested entry
- WHEN the entry is persisted to `history.json`
- THEN its `created_at` is the current UTC time in naive form (no timezone suffix), independent of the machine's local timezone

#### Scenario: History round-trips through disk
- GIVEN an entry with `normalized_text` set and status `"ready"`
- WHEN the storage service is re-initialized against the same cache directory
- THEN the loaded entry has the same `normalized_text` and field values as before the restart

#### Scenario: Older entries without optional fields parse
- GIVEN a `history.json` entry that lacks `normalized_text`, `audio_path`, `was_regenerated`, and other optional fields
- WHEN the history is loaded
- THEN the entry parses successfully with the missing fields defaulted (null / false)
