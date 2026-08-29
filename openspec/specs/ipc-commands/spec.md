# IPC Commands and Events Specification

## Purpose

Defines the IPC contract between the React frontend and the Rust backend:
the Tauri commands invoked via `invoke("command_name", args)` (registered in
`src-tauri/src/lib.rs`, implemented in `src-tauri/src/commands/mod.rs`) and the
Tauri events the backend emits to the frontend via `listen("event_name", handler)`.
This covers command signatures, typed error format, shared data types, and event
payloads as currently implemented. The backend-to-Python protocol is specified
separately in `ttsd-protocol`.

## Requirements

### Requirement: Command Error Format

All fallible Tauri commands SHALL return errors as a typed JSON object
(`CommandError` in `src-tauri/src/commands/mod.rs`, serialized with
`#[serde(tag = "type", rename_all = "snake_case")]`), causing the frontend
`invoke()` promise to reject with that object.

```typescript
interface CommandError {
  type: "not_found" | "storage_error" | "synthesis_error"
      | "playback_error" | "config_error" | "internal";
  message: string; // human-readable detail (Russian for user-visible cases)
}
```

#### Scenario: Command failure rejects with typed error
- GIVEN a command such as `get_entry` with a malformed `id`
- WHEN the command handler returns an error
- THEN the `invoke()` promise rejects with `{ "type": "not_found", "message": "..." }`

#### Scenario: Storage error mapping
- GIVEN a storage failure where the entry does not exist
- WHEN any command surfaces that `StorageError::NotFound`
- THEN the error is serialized with `type: "not_found"` and the entry id in the message; all other storage failures serialize as `type: "storage_error"`

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
  code_block_mode: string;         // "skip" | "read"
  read_operators: boolean;
  theme: string;                   // "light" | "dark" | "auto"
  player_hotkeys: Record<string, string>;
  window_geometry: [number, number, number, number] | null;
  preview_dialog_enabled: boolean;
  engine: string;                  // "piper" | "silero" | "silero_native" (default)
  piper_voice: string;             // default "ruslan"
}
```

#### Scenario: TextEntry round-trips through get_entries
- GIVEN an entry persisted in storage with `format` and `html_source` set
- WHEN the frontend calls `invoke("get_entries")`
- THEN each entry serializes with the field names above, `status` and `format` as lowercase strings or null, and `html_source` as a string or null

#### Scenario: UIConfig defaults include engine fields
- GIVEN a fresh installation with no `config.json`
- WHEN the frontend calls `invoke("get_config")`
- THEN the response contains `engine: "silero_native"` and `piper_voice: "ruslan"` alongside the legacy fields

### Requirement: Frontend Parameter Casing

The frontend SHALL pass invoke arguments in camelCase (Tauri 2 converts them to
the Rust snake_case parameter names automatically), e.g.
`invoke("seek_to", { positionSec: 2.0 })`.

#### Scenario: camelCase argument reaches snake_case handler
- GIVEN the `seek_to` command declared with parameter `position_sec`
- WHEN the frontend invokes it with `{ positionSec: 2.0 }`
- THEN the Rust handler receives `position_sec = 2.0`

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

### Requirement: Entry Query Commands

The system SHALL provide `get_entries` returning all entries sorted by
`created_at` descending (newest first, empty array when none), and
`get_entry(id)` returning a single `TextEntry` or `null` when not found.

#### Scenario: get_entries ordering
- GIVEN three entries created at different times
- WHEN the frontend invokes `get_entries`
- THEN entries are returned newest-first

#### Scenario: get_entry miss returns null
- GIVEN no entry with the requested id
- WHEN the frontend invokes `get_entry`
- THEN the promise resolves with `null` (not an error)

#### Scenario: malformed id is an error
- GIVEN an id that is not a valid UUID
- WHEN `get_entry` is invoked
- THEN the command fails with `type: "not_found"`

### Requirement: Entry Deletion Commands

The system SHALL provide `delete_entry(id)` — removing the entry together with
its audio and timestamps files — and `delete_audio(id)` — removing only the
audio and timestamps files while keeping the entry, whose status is reset to
`pending`. If the deleted entry is currently playing, playback SHALL be stopped
first (emitting `playback_stopped`). After `delete_audio` the backend SHALL
emit `entry_updated` with the reset entry. A missing entry fails with
`not_found`.

#### Scenario: delete_entry stops playback of the playing entry
- GIVEN entry X is currently playing
- WHEN `delete_entry` is invoked for X
- THEN playback stops (`playback_stopped` emitted) and the entry plus its files are removed

#### Scenario: delete_audio resets the entry
- GIVEN an entry with status `ready`
- WHEN `delete_audio` is invoked
- THEN audio and timestamps files are deleted, status becomes `pending`, and `entry_updated` is emitted

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

### Requirement: Synthesis Cancellation Command

The system SHALL provide `cancel_synthesis(id)` which actually stops the
entry's synthesis work and sets the entry status back to `pending`, emitting
`entry_updated`. Cancellation SHALL abort the entry's spawned synthesis task
via a per-entry abort registry. If the cancelled entry had already entered
the TTS stage, the system SHALL additionally terminate the current ttsd
subprocess; recovery then follows the ttsd-protocol auto-restart procedure,
and requests belonging to other entries are retried transparently. If the
active engine was switched while the cancelled entry's synthesis was in
flight, cancellation SHALL also terminate the previous engine's ttsd
subprocess — the engine that is actually running the entry's request — so
the orphaned process does not keep consuming CPU. A late
completion or failure belonging to a cancelled entry SHALL be discarded: the
entry MUST NOT flip to `ready` or `error`, any audio/timestamp files written
by the late completion SHALL be removed, no further `entry_updated` for that
completion is emitted, and no autoplay starts. A missing entry fails with
`not_found`. Cancelling a `pending` entry SHALL succeed idempotently (the
entry is queued or idle; a just-added entry may already have its synthesis
task registered while still in `pending`, and cancellation must abort it).
An entry whose status is `ready`, `playing`, or `error` SHALL be rejected
with `synthesis_error` without changing its status or touching the
synthesis registries.

#### Scenario: cancel a queued synthesis

- GIVEN an entry with status `processing` whose request has not yet reached
  ttsd
- WHEN `cancel_synthesis` is invoked
- THEN the synthesis task is aborted, the entry status becomes `pending`,
  `entry_updated` is emitted, and the ttsd subprocess keeps running (no
  restart)

#### Scenario: cancel an in-flight synthesis

- GIVEN an entry with status `processing` whose request is being synthesized
  by ttsd
- WHEN `cancel_synthesis` is invoked
- THEN the synthesis task is aborted, the ttsd subprocess is terminated, the
  supervisor restarts it per the auto-restart procedure, and the entry
  status becomes `pending`

#### Scenario: cancel after an engine switch

- GIVEN an entry with status `processing` whose request is being synthesized
  by ttsd on the Silero engine
- WHEN the active engine is switched to Piper and `cancel_synthesis` is then
  invoked for that entry
- THEN the synthesis task is aborted, the entry status becomes `pending`,
  and the swapped-out Silero engine's ttsd subprocess is terminated

#### Scenario: late completion is discarded

- GIVEN an entry that was cancelled back to `pending` while its request was
  in flight
- WHEN the orphaned request completes
- THEN the entry remains `pending`, the generated audio/timestamp files are
  removed, no `entry_updated` with `ready` is emitted, and no autoplay
  starts

#### Scenario: cancel a missing entry

- GIVEN no entry with the given id
- WHEN `cancel_synthesis` is invoked
- THEN the command fails with `not_found`

#### Scenario: cancel an idle entry

- GIVEN an entry with status `pending` and no synthesis in flight
- WHEN `cancel_synthesis` is invoked
- THEN the command succeeds, the entry remains `pending`, and
  `entry_updated` is emitted

#### Scenario: cancel a terminal entry

- GIVEN an entry with status `ready`, `playing`, or `error`
- WHEN `cancel_synthesis` is invoked
- THEN the command fails with `synthesis_error`, the entry status is left
  unchanged, and no `entry_updated` is emitted

### Requirement: Playback Control Commands

The system SHALL provide `play_entry(id)`, `pause_playback()`, `resume_playback()`,
`stop_playback()`, and `seek_to(position_sec)` driving the mpv-backed player.

`play_entry` SHALL fail with `not_found` for a missing entry, with
`playback_error` when the entry status is not `ready`, and with `playback_error`
when the audio file is missing. `seek_to` forwards an absolute seek (seconds)
to mpv and immediately emits a `playback_position` event with the target.
Player failures surface as `playback_error`.

#### Scenario: play a ready entry
- GIVEN an entry with status `ready` and an existing audio file
- WHEN `play_entry` is invoked
- THEN playback starts and `playback_started` is emitted with the entry id and cached duration

#### Scenario: play a non-ready entry is rejected
- GIVEN an entry with status `pending`
- WHEN `play_entry` is invoked
- THEN the command fails with `type: "playback_error"`

#### Scenario: seek emits immediate position sync
- GIVEN an entry is playing
- WHEN `seek_to` is invoked with `position_sec: 2.0`
- THEN mpv receives an absolute seek and a `playback_position` event with `position_sec: 2.0` is emitted immediately

### Requirement: Playback Parameter Commands

The system SHALL provide `set_speed(speed)` and `set_volume(volume)` with
inclusive range validation: `speed` in `[0.5, 3.0]`, `volume` in `[0.0, 1.0]`.
Out-of-range values SHALL be rejected with `config_error` (not clamped).
`set_speed` SHALL persist the value to `UIConfig.speech_rate`; `set_volume`
SHALL NOT persist anything. Pitch-correct speed scaling uses mpv's
`scaletempo2` audio filter.

#### Scenario: valid speed is applied and persisted

- **GIVEN** playback is active
- **WHEN** `set_speed` is invoked with `2.7`
- **THEN** mpv speed is set to 2.7 and `speech_rate: 2.7` is written to the config

#### Scenario: out-of-range values are rejected

- **GIVEN** any playback state
- **WHEN** `set_speed` is invoked with `3.5` or `set_volume` with `1.2`
- **THEN** the command fails with `type: "config_error"` naming the allowed range

### Requirement: Configuration Commands

The system SHALL provide `get_config()` returning the current `UIConfig`, and
`update_config(patch)` merging a partial `UIConfigPatch` (any subset of the
`UIConfig` fields; omitted fields keep their values) into the current config.
Before persisting, `update_config` SHALL apply the requested TTS engine through
the engine switcher; if the engine cannot be activated (e.g. Silero stack not
spawnable) the command SHALL fail with `config_error` and the previous config
MUST remain on disk.

#### Scenario: partial patch updates only named fields
- GIVEN a stored config with `theme: "auto"`
- WHEN `update_config` is invoked with `{ theme: "dark" }`
- THEN only `theme` changes, all other fields keep their values, and the config is persisted

#### Scenario: engine switch failure preserves the old config
- GIVEN the Silero engine is unavailable on the system
- WHEN `update_config` is invoked with `{ engine: "silero" }`
- THEN the command fails with `type: "config_error"` and the on-disk config still has the previous engine

### Requirement: Engine Availability Command

The system SHALL provide `get_available_engines()` returning per-engine
availability (`AvailableEngines`):

```typescript
interface LocalizedText {
  code: string;        // machine-readable reason id, e.g. "silero.uv_missing"
  params?: string[];   // positional interpolation values
  message?: string;    // optional raw diagnostic detail
}
interface EngineAvailability {
  available: boolean;
  reason: LocalizedText | null; // Some only when available == false
}
interface AvailableEngines {
  piper: EngineAvailability;
  silero: EngineAvailability;
  silero_native: EngineAvailability;
}
```

Piper (in-process) SHALL always report `available: true`. Silero SHALL report
availability based on a cheap probe: presence of `pyproject.toml` in the ttsd
directory and a successful `uv --version` exec; a `uv` binary that cannot be
spawned at all (not installed — the normal case on Windows, where ttsd is
not shipped) SHALL be treated as an unsuccessful probe, not an error.
Silero Native SHALL report availability based on presence and manifest
validity of the downloaded model bundle in the app data dir. When
unavailable, `reason` SHALL carry a machine-readable code (translated by the
frontend like command errors), not user-facing prose.

#### Scenario: probe on a system without ttsd
- **GIVEN** no `pyproject.toml` in the resolved ttsd directory
- **WHEN** `get_available_engines` is invoked
- **THEN** `silero.available` is `false` with `reason.code` `"silero.ttsd_missing"`, and `piper.available` is `true`

#### Scenario: probe when uv cannot be spawned
- **GIVEN** a `pyproject.toml` exists in the resolved ttsd directory but the
  `uv` binary is not installed (spawn fails)
- **WHEN** `get_available_engines` is invoked
- **THEN** `silero.available` is `false` with `reason.code` `"silero.uv_missing"`, the command
  succeeds, and `piper.available` is `true`

#### Scenario: native engine unavailable before bundle download
- **GIVEN** no model bundle in the app data dir
- **WHEN** `get_available_engines` is invoked
- **THEN** `silero_native.available` is `false` with `reason.code`
  `"native.bundle_missing"` indicating that the model bundle must be downloaded

### Requirement: Piper Voice Download Command

The system SHALL provide `download_piper_voice(voice_id)` which downloads the
voice files on demand, skipping files already present on disk (idempotent).
Progress is reported via the `voice_download_*` events; the command result
reports only the final outcome. An unknown voice id fails with
`synthesis_error` (`voice_unknown`).

#### Scenario: download an installed voice is a no-op
- GIVEN the voice files already exist on disk
- WHEN `download_piper_voice` is invoked for that voice
- THEN the command succeeds and progress events report the files as skipped

### Requirement: Silero Native Bundle Download Command

The system SHALL provide `download_silero_native_bundle()` which downloads the
model bundle files from the project's GitHub Releases into the app data dir,
verifying each file's sha256 against the bundle manifest, skipping files
already present and valid (idempotent). A failed checksum MUST abort the
download with a typed error and leave the engine unavailable; partial files
MUST NOT be treated as installed on the next run.

#### Scenario: fresh download succeeds

- GIVEN no bundle in the data dir and network access to GitHub Releases
- WHEN `download_silero_native_bundle` is invoked
- THEN all bundle files land in the data dir, checksums verify, and a
  subsequent `get_available_engines` reports `silero_native.available: true`

#### Scenario: checksum failure aborts

- GIVEN a download where one file's sha256 does not match the manifest
- WHEN the verification step runs
- THEN the command fails with a typed error, the invalid file is removed or
  quarantined, and `silero_native.available` stays `false`

### Requirement: Timestamp Query Command

The system SHALL provide `get_timestamps(id)` returning the `WordTimestamp`
array for an entry (empty array when the entry has no timestamps file). A
missing entry fails with `not_found`; an unreadable timestamps file fails with
`storage_error`.

#### Scenario: entry without timestamps
- GIVEN a `pending` entry that was never synthesized
- WHEN `get_timestamps` is invoked
- THEN the promise resolves with an empty array

### Requirement: Cache Management Commands

The system SHALL provide `clear_cache(args)`, `get_cache_stats()`, and
`get_cache_dir()`.

`clear_cache` takes `{ mode, delete_texts }` where `mode` is
`{ mode: "size_limit", target_mb }` or `{ mode: "all" }` and `delete_texts`
defaults to `false`. It SHALL always sweep orphan files in the audio directory,
then evict entries per the mode. With `delete_texts: false` evicted entries keep
their history records with `audio_path: null` and status reset to `pending`
(emitting `entry_updated` per entry); with `delete_texts: true` they are removed
from history (emitting `entry_removed` with `{ id }` per entry). Entries with
status `processing` SHALL be skipped. The command returns
`{ deleted_files, deleted_entries, freed_bytes }`.

`get_cache_stats` SHALL return `{ total_bytes, audio_file_count }`.
`get_cache_dir` SHALL return the absolute path of the per-user **data directory**
resolved at startup — the root holding `history.json` and `audio/`.

#### Scenario: size-limit eviction keeps texts
- GIVEN a cache exceeding `target_mb` and `delete_texts: false`
- WHEN `clear_cache` is invoked
- THEN oldest entries are evicted until the cache fits, each evicted entry emits `entry_updated` with status `pending`, and the result reports the counts and freed bytes

#### Scenario: full eviction removes texts
- GIVEN `mode: "all"` and `delete_texts: true`
- WHEN `clear_cache` is invoked
- THEN all audio is dropped, entries are removed from history, and `entry_removed` is emitted per removed entry

### Requirement: Entry Lifecycle Events

The backend SHALL emit `entry_updated` with payload `{ entry: TextEntry }`
whenever an entry is created or any of its fields change: on ingestion
(`pending`), when synthesis starts (`processing`, `normalized_text` set), when
synthesis completes (`ready`, audio/timestamps paths and `duration_sec` set),
when synthesis fails (`error`, `error_message` set), after `delete_audio`,
`regenerate_entry`, `cancel_synthesis`, and after `clear_cache` for each reset
entry. A discarded late completion (after cancellation) SHALL NOT emit
`entry_updated` with `ready` or `error`. The backend SHALL emit
`entry_removed` with payload `{ id }` when an entry is removed from history
by a bulk operation; the frontend MUST drop the entry from local state
without expecting any `entry_updated` follow-up.

#### Scenario: synthesis progress is reflected via entry_updated

- GIVEN a newly ingested entry
- WHEN background synthesis runs to completion
- THEN the frontend receives `entry_updated` with `pending`, then
  `processing`, then `ready` carrying the audio path and duration

#### Scenario: no ready event after cancellation

- GIVEN an entry cancelled back to `pending`
- WHEN its orphaned synthesis completes
- THEN no `entry_updated` carrying `ready` is emitted for that completion

#### Scenario: bulk removal notification

- GIVEN `clear_cache` removed an entry from history
- WHEN the `entry_removed` event arrives
- THEN the payload is `{ id: "<uuid>" }` and no `entry_updated` follows for
  that entry

### Requirement: Synthesis Failure Event

When background synthesis fails at the TTS stage, the backend SHALL first emit
`entry_updated` with status `error` and then emit `tts_error` with payload
`{ entry_id, message }` for the frontend toast.

#### Scenario: TTS failure emits both events
- GIVEN an entry whose synthesis fails inside the TTS engine
- WHEN the error is handled
- THEN `entry_updated` (status `error`) arrives before `tts_error` with the entry id and message

### Requirement: Playback Events

The backend SHALL emit playback events with the following payloads:

- `playback_started` — `{ entry_id, duration_sec }` (`duration_sec` may be null until mpv reports it) on play and resume
- `playback_position` — `{ position_sec, entry_id, duration_sec }` every 100 ms while playing, plus an immediate emit after each `seek_to`
- `playback_paused` — `{ entry_id, position_sec }`
- `playback_stopped` — `{}` on manual stop, on natural end, on deletion of the playing entry, and when the mpv instance is re-initialized
- `playback_finished` — `{ entry_id }` when the track reaches its natural end (position within 0.2 s of duration or mpv unloads the file), immediately followed by `playback_stopped`

Position ticks within 300 ms after a seek SHALL be suppressed so stale mpv
`time-pos` values do not snap the UI back to the pre-seek position (EOF
detection still runs during the suppression window).

#### Scenario: periodic position updates
- GIVEN an entry is playing
- WHEN 500 ms elapse
- THEN approximately five `playback_position` events arrive with monotonically increasing `position_sec`

#### Scenario: natural end of track
- GIVEN an entry playing near its end
- WHEN the position reaches the duration
- THEN `playback_finished` with the entry id is emitted, followed by `playback_stopped`

#### Scenario: seek suppression window
- GIVEN a seek to 2.0 s just happened
- WHEN the next 100 ms tick fires within 300 ms of the seek
- THEN no stale `playback_position` with the pre-seek position is emitted

### Requirement: Model Lifecycle Events

The backend SHALL emit `model_loading` (`{}`) when the active TTS engine starts
loading its model, `model_loaded` (`{}`) when the model is ready, and
`model_error` (`{ message }`) when loading fails. The same lifecycle SHALL be
re-emitted after every successful ttsd respawn (Silero engine). When the ttsd
supervisor detects a dead subprocess it SHALL emit `ttsd_restarting` (`{}`)
before respawn attempts, and `tts_fatal` (`{ message }`) after all respawn
attempts are exhausted.

#### Scenario: startup warmup lifecycle
- GIVEN the application just started
- WHEN the engine warms up in the background
- THEN the frontend receives `model_loading` followed by `model_loaded` (or `model_error` on failure)

#### Scenario: ttsd crash lifecycle
- GIVEN the ttsd subprocess died unexpectedly
- WHEN the supervisor begins respawning
- THEN `ttsd_restarting` is emitted, and after a successful respawn `model_loading` → `model_loaded` replays; after three failed attempts `tts_fatal` is emitted with the spawn error message

### Requirement: Voice Download Events

During `download_piper_voice` the backend SHALL emit:

- `voice_download_started` — `{ engine, voice }`
- `voice_download_progress` — `{ engine, voice, file_kind, file_idx, total_files, downloaded_bytes, total_bytes }`, throttled to roughly one event per 256 KB, plus `skipped: true` for files already present
- `voice_download_finished` — `{ engine, voice, ok: true }` on success or `{ engine, voice, ok: false, message }` on failure

#### Scenario: download progress reporting
- GIVEN a voice that is not installed
- WHEN `download_piper_voice` runs
- THEN `voice_download_started` fires first, `voice_download_progress` events carry cumulative byte counts per file, and a terminal `voice_download_finished` with `ok: true` completes the sequence

### Requirement: Bundle Download Events

During `download_silero_native_bundle` the backend SHALL emit:

- `bundle_download_started` — `{ engine: "silero_native" }`
- `bundle_download_progress` — `{ engine, file, file_idx, total_files, downloaded_bytes, total_bytes }`, throttled to roughly one event per 256 KB, plus `skipped: true` for files already present and valid
- `bundle_download_finished` — `{ engine, ok: true }` on success or `{ engine, ok: false, message }` on failure

#### Scenario: download progress reporting
- GIVEN a bundle that is not installed
- WHEN `download_silero_native_bundle` runs
- THEN `bundle_download_started` fires first, progress events carry cumulative
  byte counts per file, and a terminal `bundle_download_finished` completes
  the sequence

### Requirement: Synthesis voice follows the active engine

The voice passed to the TTS engine during synthesis SHALL be selected by the
engine **active at synthesis time** (`EngineSwitcher.kind()`), not by the
persisted `UIConfig.engine`: Piper → `UIConfig.piper_voice`, Silero and
Silero Native → `UIConfig.speaker`. This matters whenever the startup
fallback serves a different engine than the config names (e.g.
`engine = "silero_native"` with no model bundle on disk runs Piper for that
session): the fallback engine SHALL receive its own voice id.

#### Scenario: fallback engine receives its own voice

- GIVEN the persisted config has `engine = "silero_native"` and
  `piper_voice = "ruslan"`, and the Silero Native bundle is not downloaded,
  so the active engine is Piper
- WHEN a synthesis runs
- THEN the Piper engine is invoked with voice `ruslan`, not the Silero
  speaker id

#### Scenario: no reverse coercion

- GIVEN the persisted config has `engine = "piper"` and
  `speaker = "aidar"`, and the active engine is Silero Native
- WHEN a synthesis runs
- THEN the engine is invoked with voice `aidar`, not the Piper voice id

### Requirement: Piper voice auto-download on synthesis

When a synthesis on the **active** Piper engine fails with
`voice_not_installed`, the system SHALL download the voice via the Piper
voice catalog and retry the synthesis once. The auto-download SHALL emit the
`voice_download_*` events so the user sees a progress notification instead
of a silent stall; only a failed download (or a failed retry) surfaces an
error to the entry. The gate SHALL key on the active engine kind, so a Piper
fallback session (persisted config naming a Silero engine) is covered too.
Auto-download does not apply to the Silero engines — their voices ship with
the engine.

#### Scenario: missing Piper voice is fetched transparently

- GIVEN the active engine is Piper and the configured Piper voice is not on
  disk
- WHEN a synthesis runs
- THEN a `voice_download_started` event fires, the voice files are
  downloaded, and the synthesis is retried once with the same parameters

#### Scenario: failed download surfaces the error

- GIVEN the active engine is Piper and the configured voice is not in the
  catalog (or the download fails)
- WHEN a synthesis runs
- THEN `voice_download_finished` carries `ok: false` with the message, and
  the entry transitions to `error`

#### Scenario: fallback session covered

- GIVEN the persisted config has `engine = "silero_native"`, the bundle is
  missing, and the active engine is Piper
- WHEN a synthesis hits `voice_not_installed`
- THEN the auto-download and retry run exactly as if Piper were the
  persisted engine

### Requirement: Log directory command

The system SHALL provide `get_log_dir()`, which returns the absolute
per-user log directory path (the same directory `tauri-plugin-log` writes
its rotated files into). The command SHALL create the directory if it does
not yet exist, so the frontend can reveal a real path even before any log
line is flushed. The frontend reveals this path in the OS file manager via
a "Открыть папку" button in Settings.

#### Scenario: log dir is created and returned

- GIVEN the app running on any supported OS
- WHEN `get_log_dir` is invoked
- THEN it returns the absolute per-app log directory path and that directory
  exists on disk

### Requirement: Entry Source Annotation

The `add_text_entry` command SHALL accept an optional `source` parameter (`"clipboard"` | `"file"` | `"url"`) recording where the entry's text came from, and SHALL persist it in `TextEntry.source`. The `add_clipboard_entry` command (tray path) SHALL annotate its entries as `"clipboard"`. Older entries without the annotation carry `source: null`; the commands MUST NOT fail when it is absent.

#### Scenario: Imported entry carries its source
- GIVEN the frontend ingests text fetched from a URL
- WHEN it invokes `add_text_entry` with `source: "url"`
- THEN the persisted entry has `source: "url"` and the annotation survives restarts

#### Scenario: Tray clipboard entries are annotated
- GIVEN the user adds text from the tray menu
- WHEN `add_clipboard_entry` runs
- THEN the persisted entry has `source: "clipboard"`

### Requirement: Generation Parameters Snapshot

On every successful synthesis the system SHALL record a `generation` snapshot on the entry describing the parameters that produced the current audio, and SHALL increment the entry's `generation_count` by one. The snapshot SHALL contain:

- the engine that actually served the request (`silero_native` | `piper` | `silero`) — the engine resolved at synthesis time, not the persisted config preference;
- the resolved engine-specific voice id actually passed to synthesis (Piper uses `piper_voice`, both Silero engines use `speaker`);
- the actual output sample rate of the rendered audio (for Piper this is the rate fixed by the voice model);
- the model identity where the engine can report it cheaply — silero-native: the bundle manifest `model_id` with the `tts_main.onnx` sha256; Piper: the voice model file name from the catalog; ttsd: no model identity yet, the field stays null;
- the application version at generation time;
- the normalization settings in effect (`code_block_mode`, `read_operators`);
- the sha256 of the normalized text used;
- the stored audio file's codec (from the final file: Ogg Opus, or WAV fallback) and size in bytes.

Regenerating an entry SHALL overwrite the snapshot and increment `generation_count`. Clearing an entry's audio (audio-only deletion, or load-time validation when the audio file is missing) SHALL clear the snapshot together with the other audio metadata. Entries synthesized by older builds have no snapshot; payloads carry `generation: null` and the commands MUST NOT fail.

The snapshot SHALL flow to the frontend as part of entry payloads (`get_entries`, `get_entry`, `entry_updated` events).

#### Scenario: Snapshot recorded on first synthesis
- GIVEN an entry is synthesized successfully on the silero-native engine with speaker `xenia`
- WHEN the synthesis completes
- THEN the entry has `generation_count` equal to 1 and a snapshot with engine `silero_native`, voice `xenia`, a non-null sample rate, the app version, and non-null audio codec and size

#### Scenario: Regeneration refreshes the snapshot
- GIVEN a ready entry with `generation_count` equal to 1 and a snapshot
- WHEN the entry is regenerated successfully with a different voice selected
- THEN the snapshot's voice equals the new voice and `generation_count` equals 2

#### Scenario: Snapshot survives while text stays, cleared with audio
- GIVEN a ready entry with a snapshot
- WHEN audio-only deletion runs for the entry
- THEN the entry's `generation` is null while `generation_count` keeps its value

#### Scenario: Legacy entries carry no snapshot
- GIVEN a `history.json` written by a build older than this feature, containing a ready entry with audio
- WHEN the frontend invokes `get_entries`
- THEN the entry is returned with `generation: null` and `generation_count: 0`, and no error is raised



### Requirement: Audio Export Commands

The system SHALL expose two Tauri commands for per-entry audio export
(issue #225), following the #224 rfd-backend pattern (no dialog/fs plugin,
no capability changes):

`pick_export_audio_path(entry_id)` SHALL open the xdg-desktop-portal save
dialog (Linux) pre-filled with the extensionless name `ruvox-<entry_id>` (the portal
does not sync the name with the combo, and a stale pre-filled extension
would trip the overwrite confirmation) and a
«Формат» choice combo — `WAV` selected by default, `Ogg Opus` as the
alternative — and SHALL NOT gate it on file-type filters (the combo, not a
filter switch, decides the format). The portal response SHALL report the
combo's selected value, and the returned path SHALL be normalized to that
format: a matching extension (case-insensitive) SHALL be kept as typed, a
mismatched or foreign extension SHALL be replaced, and a missing one
appended, so the file name always matches the exported bytes. If the
response carries no usable choice, the stored format's extension SHALL be
used as the fallback. Cancelling the dialog SHALL return `None`. A missing entry SHALL fail with
`entry.not_found`; an entry without a stored `audio_path` SHALL fail with
`export.no_audio`.

`export_audio(entry_id, path)` SHALL resolve the entry's stored audio file
under the storage lock (`audio/<audio_path>` inside the data dir) and, on
the blocking thread, produce the file at `path` (issue #252): a `.wav`
target for an `.opus`-stored file SHALL be produced by decoding the Opus
stream to a mono 16-bit PCM WAV at 48 kHz, honoring the stream's pre-skip
and end trim; every other combination SHALL be a byte-for-byte copy. The
cached original MUST NOT be modified in either case. A missing entry SHALL
fail with `entry.not_found`; a missing source file SHALL fail with
`export.no_audio`; a failed conversion SHALL fail with
`export.convert_failed` carrying the underlying error as a message param;
an I/O failure of the copy SHALL fail with `export.copy_failed` carrying the
underlying error as a message param. A panicked blocking task SHALL fail
with `export.dialog_panicked` (pick) or `export.task_panicked` (export).
The commands MUST NOT create history or queue side effects — no
`entry_updated` emission, no status change.

The frontend wrappers SHALL be `commands.pickExportAudioPath(entryId)` and
`commands.exportAudio(entryId, path)`.



#### Scenario: Returned path is normalized to the stored format

- GIVEN an `.opus`-stored entry and a dialog result of `/tmp/audio.mp3` (or
  an extensionless `/tmp/audio`)
- WHEN the command normalizes the returned path
- THEN the path is `/tmp/audio.opus` — a foreign extension is replaced and a
  missing one appended with the stored format's extension, while a
  recognized `opus`/`wav` extension (any case) is kept as typed

#### Scenario: Export dialog carries a format choice with WAV default

- GIVEN an entry with stored audio
- WHEN `pick_export_audio_path` is invoked
- THEN the dialog opens pre-filled with the extensionless name `ruvox-<id>`
  and a «Формат» combo reporting `WAV` by default with `Ogg Opus` as the
  alternative

#### Scenario: The chosen format decides the export

- GIVEN a dialog result of `/tmp/audio.wav` while the «Формат» combo
  reports `Ogg Opus`
- WHEN the command normalizes the returned path
- THEN the path is `/tmp/audio.opus` — the chosen format's extension is
  enforced (matching extensions in any case are kept as typed), and the
  subsequent `export_audio` copy/convert decision follows it

#### Scenario: A response without a usable choice falls back to the stored format

- GIVEN a portal response that carries no «Формат» value
- WHEN `pick_export_audio_path` is invoked for an `.opus`-stored entry
- THEN the returned path is normalized to the stored format's extension

#### Scenario: Cancelled dialog resolves to null

- GIVEN the save dialog is open for an entry
- WHEN the user cancels the dialog
- THEN the command resolves to `null` and no file is written

#### Scenario: Export copies the stored file

- GIVEN an entry with a stored audio file and a chosen target path whose
  extension does not request a conversion (e.g. `.opus` for an `.opus`
  source)
- WHEN `export_audio` is invoked
- THEN the cached file is copied byte-for-byte to the target path, the cache
  file remains in place, and no `entry_updated` is emitted

#### Scenario: Export to a `.wav` target converts the audio

- GIVEN an entry whose stored audio file is `audio/<id>.opus` and a chosen
  target path ending in `.wav`
- WHEN `export_audio` is invoked
- THEN a mono 16-bit PCM WAV at 48 kHz is written to the target path
  (decodable, with pre-skip discarded and end trim applied), and the cached
  `.opus` file remains in place

#### Scenario: Conversion failure fails with `export.convert_failed`

- GIVEN an entry whose stored `.opus` file cannot be decoded (e.g. corrupt
  data) and a chosen target path ending in `.wav`
- WHEN `export_audio` is invoked
- THEN the command rejects with `export.convert_failed` and the localized
  error is shown by the frontend; the target file is not left behind

#### Scenario: Export without audio fails

- GIVEN an entry whose `audio_path` is `None` or whose cached audio file has
  been evicted
- WHEN `export_audio` is invoked
- THEN the command rejects with `export.no_audio` and no file is written

#### Scenario: Export to an unwritable target fails

- GIVEN a chosen target path whose copy fails at the OS level (e.g. a
  read-only directory)
- WHEN `export_audio` is invoked
- THEN the command rejects with `export.copy_failed` and the localized error
  is shown by the frontend
