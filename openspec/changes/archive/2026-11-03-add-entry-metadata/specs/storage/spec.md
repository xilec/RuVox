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
  title: string | null;                 // Display title seeded at ingestion; null = derive from original_text (legacy / cleared)
  note: string | null;                  // Free-form user note; never part of the TTS input
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

Status, format and display-mode values SHALL serialize as lowercase strings. `format` SHALL be written once at ingestion and SHALL NOT be rewritten afterwards; the viewer's display choice lives in `display_mode`. `created_at` and `audio_generated_at` SHALL be stored as naive timestamps without a timezone suffix; both SHALL be generated from the UTC clock (`Utc::now().naive_utc()`), and readers treat the values as UTC. `audio_path` and `timestamps_path` SHALL store the filename only; the full path resolves as `<cache_root>/audio/{filename}`. All optional `TextEntry` fields SHALL default when absent from the JSON, so entries written by older builds keep parsing; a missing `format` SHALL default to `null`, meaning the viewer derives the source format from the content, and a missing `display_mode` SHALL default to `null`, meaning the viewer shows the default view of the source format (verbatim for a plain source); a missing `html_source` SHALL default to `null`, a missing `generation` SHALL default to `null`, a missing `generation_count` SHALL default to `0`, a missing `source` SHALL default to `null`, and missing `title` / `note` SHALL default to `null` (the viewer then derives the title from `original_text`, so legacy entries need no migration). `title` is seeded on add (see "Entry CRUD") and is never required; `note` is user metadata only and SHALL NOT be concatenated into `original_text` or fed to the synthesis pipeline. Snapshots written by earlier builds may carry a `read_operators` field; readers MUST ignore it, and re-saving the entry drops it. For HTML-ingested entries, `original_text` SHALL hold the extracted plain text (the TTS pipeline input) and `html_source` SHALL hold the sanitized markup.

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
- GIVEN a `history.json` entry that lacks `format`, `display_mode`, `html_source`, `normalized_text`, `audio_path`, `was_regenerated`, `generation`, `generation_count`, `source`, `title`, `note`, and other optional fields
- WHEN the history is loaded
- THEN the entry parses successfully with the missing fields defaulted (null / false / 0), including `display_mode`, `title` and `note` as `null`

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

#### Scenario: Title and note round-trip
- GIVEN an entry with `title: "Установка"` and `note: "вернуться к бенчмаркам"`
- WHEN the entry is saved and the storage service is re-initialized
- THEN the loaded entry keeps both values and `note` is not folded into `original_text` or `normalized_text`

#### Scenario: Legacy entry without title and note
- GIVEN a `history.json` entry written before #353 that has neither `title` nor `note`
- WHEN the history is loaded
- THEN both fields default to `null` and the entry remains usable; the viewer derives its title from `original_text`

### Requirement: Entry CRUD

The storage service SHALL provide create, read, update, and delete operations over entries:

- Adding an entry SHALL strip a leading UTF-8 BOM from the text, assign a fresh UUID v4, set status `"pending"`, seed `title` from the text (see the title-seeding rule below), set `note` to `null`, and persist the history.
- Updating an entry SHALL replace the stored record and persist the history.
- Listing entries SHALL return them sorted by `created_at`, newest first.
- Deleting an entry SHALL remove its record, its audio file, and its timestamps file, then persist the history.

The title seeded on add SHALL be derived from `original_text` in the shared storage path used by every ingestion path (the UI commands and the tray alike): take the first line that yields something usable after stripping Markdown heading markers (`#`…`######`) and list bullets (`-`, `*`, `+`, `1.`, `1)`), collapse runs of whitespace to single spaces, trim, and cap at 200 Unicode scalar values; when nothing usable remains (empty or markup-only text), store `null`. The stored title SHALL NOT be rewritten later by the derivation — once set, it is the user's title until edited or cleared.

#### Scenario: BOM is stripped on add
- GIVEN text that starts with the UTF-8 BOM character `﻿`
- WHEN the entry is added
- THEN `original_text` is stored without the BOM

#### Scenario: Title is seeded from the first usable line
- GIVEN text that starts with `## Установка` followed by the body
- WHEN the entry is added through the storage service
- THEN `title` is `"Установка"` and `note` is `null`, and the title persists across a reload

#### Scenario: Nothing usable stores no title
- GIVEN text that is whitespace-only or consists only of Markdown marks (`#`, `-`, `1.`)
- WHEN the entry is added
- THEN `title` is `null`

#### Scenario: The title is capped at 200 characters
- GIVEN a first line longer than 200 characters
- WHEN the entry is added
- THEN the stored title has at most 200 characters and no trailing whitespace

#### Scenario: Every ingestion path seeds the title
- GIVEN the tray clipboard path (`add_entry`) and the UI path (`add_text_entry`)
- WHEN either adds an entry
- THEN both go through the shared storage add path and store the derived title

#### Scenario: Entries are listed newest first
- GIVEN two entries created one second apart
- WHEN all entries are listed
- THEN the more recently created entry comes first

#### Scenario: Deleting an entry removes its files
- GIVEN a ready entry with audio and timestamps files on disk
- WHEN the entry is deleted
- THEN the entry is gone from history and both files are removed from `audio/`

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

This operation SHALL also be the mechanism used for single-field metadata edits (title and note), so saving a title while a synthesis completion lands cannot persist a stale clone that reverts the completion.

#### Scenario: metadata edit does not revert a concurrent completion
- GIVEN an entry being synthesized
- WHEN a title edit applies through the conditional update while the completion transitions the entry to `ready`
- THEN the persisted entry carries both the new title and the completed status/audio fields

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
