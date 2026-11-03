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
  display_mode: DisplayMode | null;  // viewer display choice; null = rendered default
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
- GIVEN an entry persisted in storage with `format`, `display_mode` and `html_source` set
- WHEN the frontend calls `invoke("get_entries")`
- THEN each entry serializes with the field names above, `status`, `format` and `display_mode` as lowercase strings or null, and `html_source` as a string or null

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

### Requirement: Text Ingestion Commands

The system SHALL provide `add_text_entry` and `add_clipboard_entry` to create a
queue entry and start background synthesis immediately.

`add_text_entry(text, play_when_ready)` is the preferred frontend path (the
frontend reads the clipboard itself via `tauri-plugin-clipboard-manager`).
It SHALL additionally accept optional `format` and `html_source` parameters.
`format` is the immutable source format recorded at ingestion: callers that
already know it — the preview dialog's effective format, the import route,
and the content detection on the dialog-less Add paths — SHALL pass it. When
`html_source` is present, `format` SHALL be `"html"`, `text` is the extracted
plain text and `html_source` carries the sanitized markup for rendering.
When omitted, the entry is persisted with `format: null` and the viewer
derives the source from the content at render time; this is the shape of the
tray path and of entries created by older builds.
`add_clipboard_entry(play_when_ready)` reads the system clipboard in Rust via
`arboard` on a blocking thread and exists for the system tray menu, where no
webview clipboard API is available; it SHALL NOT set `format` and SHALL leave
the entry unclassified (`format: null`). Both share one implementation
(`ingest_text`):
blank/whitespace-only text is rejected with `internal`; no length-based
rejection applies — input of any length is accepted regardless of the active
TTS engine (each engine bounds its own inference by chunking); the entry is
persisted with status `pending`; `entry_updated` is emitted; synthesis runs in a
background task; the command returns the new `EntryId` without waiting for
synthesis.

#### Scenario: add_text_entry creates entry and starts synthesis
- GIVEN the model is loaded
- WHEN the frontend invokes `add_text_entry` with non-blank text
- THEN the promise resolves with the new `EntryId`, an `entry_updated` event with `status: "pending"` is emitted, and background synthesis advances the entry through `processing` to `ready` (each step emitting `entry_updated`)

#### Scenario: add_text_entry with HTML parameters
- GIVEN the model is loaded
- WHEN the frontend invokes `add_text_entry` with extracted text, `format: "html"`, and a sanitized `html_source`
- THEN the entry is persisted with `format: "html"` and the given `html_source`, and synthesis normalizes the extracted text

#### Scenario: add_text_entry with a detected source format
- GIVEN the preview dialog is disabled and the frontend classified the clipboard text as `markdown`
- WHEN the frontend invokes `add_text_entry` with that text and `format: "markdown"`
- THEN the entry is persisted with `format: "markdown"` and a null `html_source`, and the viewer derives its views from that source format

#### Scenario: Format omitted stays unclassified
- GIVEN the system tray triggered `add_clipboard_entry`
- WHEN the entry is persisted
- THEN it carries `format: null`, `html_source: null`, and `display_mode: null`, and the viewer derives the source from the text at render time

#### Scenario: blank text is rejected
- GIVEN any engine state
- WHEN `add_text_entry` is invoked with whitespace-only text
- THEN the command fails with `type: "internal"` and no entry is persisted

#### Scenario: oversized text is rejected before normalization when Piper is active
- GIVEN the active TTS engine is Piper
- WHEN `add_text_entry` or `add_clipboard_entry` is invoked with text longer than 100 000 codepoints
- THEN no length-based rejection happens: the entry is created and synthesized exactly as with shorter text (Piper synthesizes in bounded chunks; the former rejection is removed by this change)

#### Scenario: oversized text is accepted when Silero is active
- GIVEN the active TTS engine is Silero
- WHEN `add_text_entry` or `add_clipboard_entry` is invoked with text longer than 100 000 codepoints
- THEN the entry is created and synthesized exactly as with shorter text

#### Scenario: add_clipboard_entry reads the system clipboard
- GIVEN the tray menu triggered a read-now action
- WHEN `add_clipboard_entry` is invoked and the clipboard contains text
- THEN the entry is created from that text exactly as with `add_text_entry`, with `format: null`; if the clipboard is empty or unavailable the command fails with `type: "internal"`

#### Scenario: auto-play after synthesis
- GIVEN `add_text_entry` was invoked with `play_when_ready: true`
- WHEN background synthesis completes successfully
- THEN the backend loads the audio into the player and starts playback, emitting `playback_started`

## REMOVED Requirements

### Requirement: Entry Format Command

**Reason**: `set_entry_format` persisted the viewer's display choice into
`TextEntry.format`, which is now the immutable source format written once at
ingestion. The command allowed the viewer to rewrite the source format and
was replaced by `set_entry_display_mode`, which persists the separate
`display_mode` field.

**Migration**: The frontend calls `set_entry_display_mode(id, display_mode)`
instead of `set_entry_format(id, format)`. No history migration is needed:
entries written by older builds keep their `format` value (or `null`) and load
with `display_mode: null`, and the viewer derives and clamps the source
format at render time.

## ADDED Requirements

### Requirement: Entry Display Mode Command

`set_entry_display_mode(id, display_mode)` SHALL persist the viewer display
choice of an entry and notify the UI. The command SHALL accept a
`DisplayMode` value (`"rendered"` or `"plain"`), store it in the entry's
`display_mode`, and emit `entry_updated` with the updated entry. The command
SHALL NOT touch the entry's source `format`, `normalized_text`, audio, or
timestamps — switching the view is display-only and never triggers
re-synthesis or changes the source format. An unknown entry id SHALL be
rejected with a typed `not_found`-style `CommandError` and emit no event.

#### Scenario: display mode is persisted and broadcast
- GIVEN a stored markdown entry with `format: "markdown"` and `display_mode: null`
- WHEN the frontend calls `invoke("set_entry_display_mode", { id, displayMode: "plain" })`
- THEN the entry in storage has `display_mode: "plain"` and `format: "markdown"`, and an `entry_updated` event with the updated entry is emitted

#### Scenario: switch preserves the source format and audio artifacts
- GIVEN a ready entry with synthesized audio and timestamps
- WHEN `set_entry_display_mode` changes its display mode
- THEN `format`, `normalized_text`, `audio_path`, and `timestamps_path` are unchanged and no synthesis is started

#### Scenario: unknown entry is rejected
- GIVEN no entry with the given id
- WHEN `set_entry_display_mode` is called
- THEN the command rejects with a typed error and emits no event
