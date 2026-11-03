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

type DisplayMode = "rendered" | "plain"; // lowercase

interface TextEntry {
  id: EntryId;
  original_text: string;
  normalized_text: string | null;
  status: EntryStatus;
  format: TextFormat | null;         // immutable source format; null = not recorded (derived at render)
  display_mode: DisplayMode | null;  // viewer display choice; null = default view of the source format (verbatim for a plain source)
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
  reading_width: string;           // "narrow" (default) | "wide"
  reading_scale: number;           // percent, default 100, clamped 80–200
  theme: string;                   // "light" | "dark" | "auto"
  window_geometry: [number, number, number, number] | null;
  preview_dialog_enabled: boolean;
  hide_to_tray_on_close: boolean;  // default false: closing the window quits
  engine: string;                  // "piper" | "silero" | "silero_native" | "teratts" (default)
  piper_voice: string;             // default "ruslan"
  teratts_voice: string;           // default "ru_m5"
  language: string;                // "ru" (default) | "en"
}
```

#### Scenario: TextEntry round-trips through get_entries
- GIVEN an entry persisted in storage with `format`, `display_mode` and `html_source` set
- WHEN the frontend calls `invoke("get_entries")`
- THEN each entry serializes with the field names above, `status`, `format` and `display_mode` as lowercase strings or null, and `html_source` as a string or null

#### Scenario: UIConfig defaults include engine fields
- GIVEN a fresh installation with no `config.json`
- WHEN the frontend calls `invoke("get_config")`
- THEN the response contains `engine: "teratts"`, `piper_voice: "ruslan"` and `teratts_voice: "ru_m5"` alongside the legacy fields

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

#### Scenario: UIConfig defaults include reading typography

- GIVEN a fresh installation with no `config.json`, or an existing
  `config.json` written before the reading controls existed
- WHEN the frontend calls `invoke("get_config")`
- THEN the response contains `reading_width: "narrow"` and `reading_scale: 100`,
  with out-of-range persisted values already canonicalized

#### Scenario: Legacy player_hotkeys key is ignored

- GIVEN a `config.json` written before `player_hotkeys` was removed
- WHEN the frontend calls `invoke("get_config")`
- THEN the response has no `player_hotkeys` key and the rest of the config is
  returned normally
- AND the next config save drops the key from the file
