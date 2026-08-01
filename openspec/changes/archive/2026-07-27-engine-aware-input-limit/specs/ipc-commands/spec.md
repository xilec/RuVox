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
blank/whitespace-only text is rejected with `internal`; when the active TTS
engine is Piper, text longer than 100 000 codepoints SHALL be rejected with
`internal` and a Russian message naming the limit, the Piper engine, and the
option to switch to Silero, before any normalization or persistence happens
(with Silero active, no length limit applies — it synthesizes in bounded
chunks); the entry is persisted
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

#### Scenario: oversized text is rejected before normalization when Piper is active
- GIVEN the active TTS engine is Piper
- WHEN `add_text_entry` or `add_clipboard_entry` is invoked with text longer than 100 000 codepoints
- THEN the command fails with `type: "internal"` and a Russian message naming the limit and the Piper engine, no entry is persisted, and no synthesis is started

#### Scenario: oversized text is accepted when Silero is active
- GIVEN the active TTS engine is Silero
- WHEN `add_text_entry` or `add_clipboard_entry` is invoked with text longer than 100 000 codepoints
- THEN the entry is created and synthesized exactly as with an under-limit text

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
A pipeline task panic is reported as `type: "internal"`. When the active TTS
engine is Piper, text longer than 100 000 codepoints SHALL be rejected with
`internal` and a Russian message naming the limit, the Piper engine, and the
option to switch to Silero, before normalization starts; with Silero active,
no length limit applies.

#### Scenario: preview returns normalized text without side effects
- GIVEN raw text containing English identifiers
- WHEN the frontend invokes `preview_normalize`
- THEN the response is `{ normalized: "<pipeline output>" }` and no new entry appears in `get_entries`

#### Scenario: oversized preview input is rejected when Piper is active
- GIVEN the active TTS engine is Piper
- WHEN the frontend invokes `preview_normalize` with text longer than 100 000 codepoints
- THEN the command fails with `type: "internal"` and a Russian message naming the limit and the Piper engine, and normalization does not run

#### Scenario: oversized preview input is normalized when Silero is active
- GIVEN the active TTS engine is Silero
- WHEN the frontend invokes `preview_normalize` with text longer than 100 000 codepoints
- THEN the response is `{ normalized: "<pipeline output>" }` for the whole input

## ADDED Requirements

### Requirement: Synthesis-time input length guard

Background synthesis SHALL re-check the input length guard at synthesis time:
when the engine active at synthesis start is Piper and the entry text is
longer than 100 000 codepoints, the synthesis task SHALL fail the entry with
the same Russian message as the ingestion-time rejection instead of running
unchunked Piper inference. This covers entries accepted while Silero was
active whose synthesis runs after the user switched to Piper (queued
synthesis or `regenerate_entry`). With Silero active at synthesis time, no
length limit applies.

#### Scenario: oversized entry accepted under Silero fails synthesis under Piper
- GIVEN an entry with text longer than 100 000 codepoints (accepted while Silero was active)
- WHEN background synthesis for it starts with Piper as the active engine
- THEN the entry transitions to `error` with the Russian message naming the limit and the Piper engine, and no Piper inference is started

#### Scenario: oversized entry synthesizes under Silero
- GIVEN an entry with text longer than 100 000 codepoints
- WHEN background synthesis for it starts with Silero as the active engine
- THEN synthesis proceeds in bounded chunks with no length-based rejection

## MODIFIED Requirements

### Requirement: Entry Regeneration Command

The system SHALL provide `regenerate_entry(id)` which drops the current audio
and timestamps, sets `was_regenerated: true` and `error_message: null`, emits
`entry_updated`, and re-runs background synthesis with the current config
(speaker/voice, sample rate) — including the synthesis-time input length
guard (see "Synthesis-time input length guard"). If the entry is playing,
playback SHALL be
stopped first. Regeneration of an entry with status `processing` SHALL be
rejected with `synthesis_error` to avoid racing the in-flight task.

#### Scenario: regenerate a ready entry
- GIVEN a `ready` entry and a changed `speaker` in the config
- WHEN `regenerate_entry` is invoked
- THEN the old audio is deleted, `entry_updated` is emitted with `was_regenerated: true`, and a fresh synthesis advances the entry back to `ready`

#### Scenario: regenerate during synthesis is rejected
- GIVEN an entry with status `processing`
- WHEN `regenerate_entry` is invoked
- THEN the command fails with `type: "synthesis_error"` and the in-flight synthesis continues

#### Scenario: regenerate an oversized entry under Piper fails with the limit message
- GIVEN an entry with text longer than 100 000 codepoints that was accepted while Silero was active
- WHEN `regenerate_entry` is invoked with Piper as the active engine
- THEN the re-run synthesis fails the entry with the Russian message naming the limit and the Piper engine
