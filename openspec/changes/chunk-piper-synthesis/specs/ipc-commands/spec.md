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

#### Scenario: blank text is rejected
- GIVEN any engine state
- WHEN `add_text_entry` is invoked with whitespace-only text
- THEN the command fails with `type: "internal"` and no entry is persisted

#### Scenario: long text is accepted with Piper active
- GIVEN the active TTS engine is Piper
- WHEN `add_text_entry` or `add_clipboard_entry` is invoked with text longer than 100 000 codepoints
- THEN the entry is created and synthesized exactly as with shorter text (Piper synthesizes in bounded chunks)

#### Scenario: long text is accepted with Silero active
- GIVEN the active TTS engine is Silero
- WHEN `add_text_entry` or `add_clipboard_entry` is invoked with text longer than 100 000 codepoints
- THEN the entry is created and synthesized exactly as with shorter text

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
A pipeline task panic is reported as `type: "internal"`. No length-based
rejection applies: input of any length is normalized regardless of the active
TTS engine.

#### Scenario: preview returns normalized text without side effects
- GIVEN raw text containing English identifiers
- WHEN the frontend invokes `preview_normalize`
- THEN the response is `{ normalized: "<pipeline output>" }` and no new entry appears in `get_entries`

#### Scenario: oversized preview input is normalized with Piper active
- GIVEN the active TTS engine is Piper
- WHEN the frontend invokes `preview_normalize` with text longer than 100 000 codepoints
- THEN the response is `{ normalized: "<pipeline output>" }` for the whole input

#### Scenario: oversized preview input is normalized when Silero is active
- GIVEN the active TTS engine is Silero
- WHEN the frontend invokes `preview_normalize` with text longer than 100 000 codepoints
- THEN the response is `{ normalized: "<pipeline output>" }` for the whole input

## REMOVED Requirements

### Requirement: Synthesis-time input length guard
**Reason**: The guard rejected >100 000-codepoint entries at Piper synthesis time because Piper
ran one-shot unchunked inference; chunked synthesis (this change) bounds inference for any input
length, so the guard protects nothing and only degrades behavior for entries accepted under one
engine and synthesized under the other.
**Migration**: Entries longer than 100 000 codepoints are now synthesized normally under Piper
(bounded chunks). Remove the synthesis-time re-check from the background synthesis task.
