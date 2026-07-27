# Delta: ipc-commands

## MODIFIED Requirements

### Requirement: Text Ingestion Commands

The system SHALL provide `add_text_entry` and `add_clipboard_entry` to create a
queue entry and start background synthesis immediately.

`add_text_entry(text, play_when_ready)` is the preferred frontend path (the
frontend reads the clipboard itself via `tauri-plugin-clipboard-manager`).
It SHALL additionally accept optional `format` and `html_source` parameters;
when `format` is `"html"`, `text` is the extracted plain text and
`html_source` carries the sanitized markup for rendering. When omitted, the
entry behaves exactly as a plain entry (`format: null`, `html_source: null`).
`add_clipboard_entry(play_when_ready)` reads the system clipboard in Rust via
`arboard` on a blocking thread and exists for the system tray menu, where no
webview clipboard API is available. Both share one implementation (`ingest_text`):
blank/whitespace-only text is rejected with `internal`; text longer than
100 000 codepoints SHALL be rejected with `internal` and a Russian message
naming the limit before any normalization or persistence happens; the entry
is persisted
with status `pending`; `entry_updated` is emitted; synthesis runs in a
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

#### Scenario: blank text is rejected
- GIVEN any engine state
- WHEN `add_text_entry` is invoked with whitespace-only text
- THEN the command fails with `type: "internal"` and no entry is persisted

#### Scenario: oversized text is rejected before normalization
- GIVEN any engine state
- WHEN `add_text_entry` or `add_clipboard_entry` is invoked with text longer than 100 000 codepoints
- THEN the command fails with `type: "internal"` and a Russian message naming the limit, no entry is persisted, and no synthesis is started

#### Scenario: add_clipboard_entry reads the system clipboard
- GIVEN the tray menu triggered a read-now action
- WHEN `add_clipboard_entry` is invoked and the clipboard contains text
- THEN the entry is created from that text exactly as with `add_text_entry`; if the clipboard is empty or unavailable the command fails with `type: "internal"`

#### Scenario: auto-play after synthesis
- GIVEN `add_text_entry` was invoked with `play_when_ready: true`
- WHEN background synthesis completes successfully
- THEN the backend loads the audio into the player and starts playback, emitting `playback_started`

### Requirement: Normalization Preview Command

The system SHALL provide `preview_normalize(text)` returning
`{ normalized: string }` — the output of the Rust normalization pipeline
(char-mapping discarded) without persisting any entry or touching storage.
A pipeline task panic is reported as `type: "internal"`. Text longer than
100 000 codepoints SHALL be rejected with `internal` and a Russian message
naming the limit before normalization starts.

#### Scenario: preview returns normalized text without side effects
- GIVEN raw text containing English identifiers
- WHEN the frontend invokes `preview_normalize`
- THEN the response is `{ normalized: "<pipeline output>" }` and no new entry appears in `get_entries`

#### Scenario: oversized preview input is rejected
- GIVEN any engine state
- WHEN the frontend invokes `preview_normalize` with text longer than 100 000 codepoints
- THEN the command fails with `type: "internal"` and a Russian message naming the limit, and normalization does not run
