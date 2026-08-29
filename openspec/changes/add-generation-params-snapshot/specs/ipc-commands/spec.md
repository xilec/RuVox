## ADDED Requirements

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
