# Delta: ipc-commands

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
  sample_rate: number;             // default 48000
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
  engine: string;                  // "piper" (default) | "silero"
  piper_voice: string;             // default "ruslan"
}
```

#### Scenario: TextEntry round-trips through get_entries
- GIVEN an entry persisted in storage with `format` set
- WHEN the frontend calls `invoke("get_entries")`
- THEN each entry serializes with the field names above, `status` as a lowercase string, and `format` as a lowercase string or null

#### Scenario: UIConfig defaults include engine fields
- GIVEN a fresh installation with no `config.json`
- WHEN the frontend calls `invoke("get_config")`
- THEN the response contains `engine: "piper"` and `piper_voice: "ruslan"` alongside the legacy fields

## ADDED Requirements

### Requirement: Entry Format Command

`set_entry_format(id, format)` SHALL persist the display format of an entry
and notify the UI. The command SHALL accept any `TextFormat` value, store it
on the entry, and emit `entry_updated` with the updated entry. The command
SHALL NOT touch `normalized_text`, audio, or timestamps — switching the
format is display-only and never triggers re-synthesis. An unknown entry id
SHALL be rejected with a typed `not_found`-style `CommandError`.

#### Scenario: format is persisted and broadcast
- GIVEN a stored entry with `format: null`
- WHEN the frontend calls `invoke("set_entry_format", { id, format: "html" })`
- THEN the entry in storage has `format: "html"` and an `entry_updated` event with the updated entry is emitted

#### Scenario: switch preserves audio artifacts
- GIVEN a ready entry with synthesized audio and timestamps
- WHEN `set_entry_format` changes its format
- THEN `normalized_text`, `audio_path`, and `timestamps_path` are unchanged and no synthesis is started

#### Scenario: unknown entry is rejected
- GIVEN no entry with the given id
- WHEN `set_entry_format` is called
- THEN the command rejects with a typed error and emits no event
