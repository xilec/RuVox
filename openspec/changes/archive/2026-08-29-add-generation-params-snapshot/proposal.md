## Why

When the user returns to a history record after some time, there is no way to tell how its voiceover was produced. `TextEntry` stores the text and an opaque audio file; the engine, voice and sample rate that actually produced the audio live only in the global config at synthesis time and change freely afterwards. Regeneration is documented as re-baking current settings into the audio, but which settings were baked in is invisible (GitHub issue #243).

## What Changes

- Add an optional per-entry snapshot of synthesis parameters (`TextEntry.generation`), written when audio is produced and refreshed on every regeneration:
  - engine actually used (`silero_native` / `piper` / `silero`),
  - resolved voice id,
  - actual output sample rate (read from the rendered WAV, so Piper's voice-fixed rate is stored as produced),
  - model identity where cheaply available: silero-native `model_id` + `tts_main.onnx` sha256 from the bundle manifest, Piper voice model file name; ttsd leaves it absent until the protocol exposes the torch.hub tag,
  - app version, normalization settings (`code_block_mode`, `read_operators`), sha256 of the normalized text used,
  - audio facts (codec from the final file extension, file size).
- Add a monotonic `TextEntry.generation_count` (default 0) incremented on every successful bake; it survives deletion/regeneration like `was_regenerated`.
- Clear the snapshot together with the existing audio metadata wherever that metadata is cleared (`delete_audio`, load-time validation when the audio file is missing).
- Add a read-only «Параметры записи…» context-menu item in the queue list opening a details dialog for the snapshot; values that are absent (legacy entries, unknown model identity) render as absent, never guessed. RU/EN localization.
- Record an ingestion-source annotation on each entry (`TextEntry.source`: clipboard / file / URL, set by the frontend at ingestion; tray clipboard adds are annotated as clipboard) and show it as the dialog's first row.
- Out of scope: pipeline/dictionary versions (not versioned today), a normalized-text staleness hint (the stored sha256 enables it later), prosody knobs (none exist; `speech_rate` is playback-only), ttsd model tag.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `storage`: the `TextEntry` schema gains optional `generation` (synthesis-parameter snapshot), `generation_count`, and `source` (ingestion annotation); the snapshot is cleared together with the other audio metadata when audio is deleted or missing.
- `ipc-commands`: synthesis SHALL record the generation snapshot (parameters above) on every successful synthesis and refresh it on regeneration; snapshots flow to the frontend inside entry payloads; `add_text_entry` accepts an optional `source` annotation.
- `ui`: the queue context menu gains a «Параметры записи…» item opening a read-only recording-parameters dialog whose first row shows the ingestion source.

## Impact

- `src-tauri/src/storage/schema.rs` — `GenerationParams` / `ModelParams` structs, two new `TextEntry` fields (`#[serde(default)]`, existing `history.json` files stay valid).
- `src-tauri/src/storage/service.rs` — clear `generation` in `delete_audio` and in load-time audio validation.
- `src-tauri/src/tts/engine.rs` + implementations — `model_info()` on the `TtsEngine` trait (default `None`); silero-native reads the bundle manifest, Piper resolves the voice-model file name from the catalog.
- `src-tauri/src/commands/mod.rs` — snapshot construction between `finalize_audio_files` and `apply_ready_if_current`; voice resolution extracted into a helper shared with `synthesize_audio`; `apply_ready_if_current` stores the snapshot and increments `generation_count`.
- `src/lib/tauri.ts` — `TextEntry` TS mirror gains `generation` / `generation_count`.
- `src/components/QueueList.tsx`, new `src/dialogs/GenerationParamsDialog.tsx`, `src/i18n/{ru,en}.ts` — menu item, dialog, RU/EN strings.
- No dependency changes; no capability/config changes; existing history files parse unchanged (all new fields optional).
