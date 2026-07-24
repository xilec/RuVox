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

interface TextEntry {
  id: EntryId;
  original_text: string;
  normalized_text: string | null;
  status: EntryStatus;
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
- GIVEN an entry persisted in storage
- WHEN the frontend calls `invoke("get_entries")`
- THEN each entry serializes with the field names above and `status` as a lowercase string

#### Scenario: UIConfig defaults include engine fields
- GIVEN a fresh installation with no `config.json`
- WHEN the frontend calls `invoke("get_config")`
- THEN the response contains `engine: "piper"` and `piper_voice: "ruslan"` alongside the legacy fields

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
`add_clipboard_entry(play_when_ready)` reads the system clipboard in Rust via
`arboard` on a blocking thread and exists for the system tray menu, where no
webview clipboard API is available. Both share one implementation (`ingest_text`):
blank/whitespace-only text is rejected with `internal`; the entry is persisted
with status `pending`; `entry_updated` is emitted; synthesis runs in a
background task; the command returns the new `EntryId` without waiting for
synthesis.

#### Scenario: add_text_entry creates entry and starts synthesis
- GIVEN the model is loaded
- WHEN the frontend invokes `add_text_entry` with non-blank text
- THEN the promise resolves with the new `EntryId`, an `entry_updated` event with `status: "pending"` is emitted, and background synthesis advances the entry through `processing` to `ready` (each step emitting `entry_updated`)

#### Scenario: blank text is rejected
- GIVEN any engine state
- WHEN `add_text_entry` is invoked with whitespace-only text
- THEN the command fails with `type: "internal"` and no entry is persisted

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
A pipeline task panic is reported as `type: "internal"`.

#### Scenario: preview returns normalized text without side effects
- GIVEN raw text containing English identifiers
- WHEN the frontend invokes `preview_normalize`
- THEN the response is `{ normalized: "<pipeline output>" }` and no new entry appears in `get_entries`

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

### Requirement: Entry Regeneration Command

The system SHALL provide `regenerate_entry(id)` which drops the current audio
and timestamps, sets `was_regenerated: true` and `error_message: null`, emits
`entry_updated`, and re-runs background synthesis with the current config
(speaker/voice, sample rate). If the entry is playing, playback SHALL be
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

### Requirement: Synthesis Cancellation Command

The system SHALL provide `cancel_synthesis(id)` which sets the entry status
back to `pending` and emits `entry_updated`. The current implementation does
NOT abort an in-flight TTS request — the TTS supervisor serializes all requests
through a single channel, so a running synthesis continues to completion; only
the entry status is reset. A missing entry fails with `not_found`.

#### Scenario: cancel a queued synthesis
- GIVEN an entry with status `processing`
- WHEN `cancel_synthesis` is invoked
- THEN the entry status becomes `pending` and `entry_updated` is emitted

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
inclusive range validation: `speed` in `[0.5, 2.0]`, `volume` in `[0.0, 1.0]`.
Out-of-range values SHALL be rejected with `config_error` (not clamped).
`set_speed` SHALL persist the value to `UIConfig.speech_rate`; `set_volume`
SHALL NOT persist anything. Pitch-correct speed scaling uses mpv's
`scaletempo2` audio filter.

#### Scenario: valid speed is applied and persisted
- GIVEN playback is active
- WHEN `set_speed` is invoked with `1.5`
- THEN mpv speed is set to 1.5 and `speech_rate: 1.5` is written to the config

#### Scenario: out-of-range values are rejected
- GIVEN any playback state
- WHEN `set_speed` is invoked with `2.5` or `set_volume` with `1.2`
- THEN the command fails with `type: "config_error"` naming the allowed range

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
interface EngineAvailability { available: boolean; reason: string | null }
interface AvailableEngines { piper: EngineAvailability; silero: EngineAvailability }
```

Piper (in-process) SHALL always report `available: true`. Silero SHALL report
availability based on a cheap probe: presence of `pyproject.toml` in the ttsd
directory and a successful `uv --version` exec. When unavailable, `reason`
SHALL be a Russian-language user-facing string.

#### Scenario: probe on a system without ttsd
- GIVEN no `pyproject.toml` in the resolved ttsd directory
- WHEN `get_available_engines` is invoked
- THEN `silero.available` is `false` with a Russian `reason`, and `piper.available` is `true`

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
`get_cache_dir` SHALL return the absolute cache directory path resolved at startup.

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
entry. The backend SHALL emit `entry_removed` with payload `{ id }` when an
entry is removed from history by a bulk operation; the frontend MUST drop the
entry from local state without expecting any `entry_updated` follow-up.

#### Scenario: synthesis progress is reflected via entry_updated
- GIVEN a newly ingested entry
- WHEN background synthesis runs to completion
- THEN the frontend receives `entry_updated` with `pending`, then `processing`, then `ready` carrying the audio path and duration

#### Scenario: bulk removal notification
- GIVEN `clear_cache` removed an entry from history
- WHEN the `entry_removed` event arrives
- THEN the payload is `{ id: "<uuid>" }` and no `entry_updated` follows for that entry

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
