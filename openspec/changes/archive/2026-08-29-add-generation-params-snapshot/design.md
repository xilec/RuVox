## Context

Synthesis flows through `spawn_synthesis` → `run_normalization` → `synthesize_audio` (resolves voice from the config keyed on the *active* engine, calls `tts.synthesize`) → `finalize_audio_files` (saves timestamps, transcodes WAV → Ogg Opus) → `apply_ready_if_current` (atomic `update_entry_if` that flips the entry to `ready` and sets the audio metadata). The stale-completion guard makes `apply_ready_if_current` the only safe place to mutate the entry after synthesis.

`TtsEngine` is engine-agnostic (`kind()`, `synthesize(...)`, …); `SynthesizeOutput` carries only timestamps and duration. The silero-native bundle dir holds `manifest.json` (`model_id`, `opset`, `export_date_utc`, per-file `path`/`size`/`sha256`); Piper voices are identified by the catalog (`tts/piper/catalog.rs`, voice id = model file basename). `hound` is already a `src-tauri` dependency and is used to read the intermediate WAV during Opus transcode; `sha2` is already used by both downloaders.

## Goals / Non-Goals

**Goals:**

- Snapshot captured from the values actually used at synthesis time (active engine, resolved voice, rendered sample rate), not from the current config.
- Zero migration: all new `TextEntry` fields are `#[serde(default)]`; old `history.json` files parse unchanged, new fields serialize as `null`/`0` for legacy entries.
- Engines that cannot report model identity cheaply stay `null` — no hashing of 60 MB voice files per synthesis, no protocol changes.

**Non-Goals:**

- No staleness hint UI (the stored `normalized_text_sha256` enables it later).
- No ttsd model tag (requires a `ttsd-protocol` change; out of scope here).
- No editing/regeneration controls in the dialog — strictly read-only.
- No pipeline/dictionary version stamping (they are unversioned today).

## Decisions

**Snapshot type lives in `storage/schema.rs` as `GenerationParams` (+ `ModelParams`).** It is persisted state, so it belongs next to `TextEntry`; commands and TTS layers construct it but do not own it. All fields required inside the struct (the app writes complete snapshots); unknown-ness is expressed with `Option` fields per the spec.

**`generation_count` is a separate `TextEntry` field, not a snapshot field.** The snapshot is cleared wherever audio metadata is cleared (`delete_audio`, load validation), but the count must survive like `was_regenerated`, otherwise it could never exceed 1 and would not pair with regeneration.

**Model identity via `TtsEngine::model_info() -> Option<ModelParams>` with a default `None` impl.** Alternatives considered: querying files from the synthesis site (duplicates engine-internal paths/knowledge outside the engine) or computing file sha256 at synthesis time (re-hashes tens of MB per run). silero-native reads the already-verified `manifest.json` from its `bundle_dir` (small sync read, sha256 of `tts_main.onnx` comes from the manifest — no hashing); Piper resolves the voice model file name from the static catalog; the ttsd supervisor keeps the default `None`. Test stub engines inherit `None` automatically.

**Actual sample rate read from the intermediate WAV header (`hound::WavReader`), not stored by engines.** The engine writes a complete WAV before `finalize_audio_files` transcodes it, so a header read gives the produced rate for all three engines with one code path, and captures Piper's voice-fixed rate for free. Read failures degrade to `null`.

**Snapshot built in one place, stored in `apply_ready_if_current`.** A `build_generation_snapshot` helper runs in `spawn_synthesis` after `finalize_audio_files` (final audio filename known → codec from extension, size from `stat`) and before `apply_ready_if_current`, which sets `e.generation = Some(snapshot)` and `e.generation_count += 1` inside the existing atomic `update_entry_if` closure — the stale guard applies to the snapshot exactly as to the rest of the metadata. The voice-resolution match moves from `synthesize_audio` into a shared `voice_for_engine(kind, &config)` helper so the snapshot builder reuses it instead of duplicating the engine→voice mapping.

**No new IPC command.** Entries (and `entry_updated` payloads) already carry the full `TextEntry`; the frontend dialog reads `entry.generation`. The TS `TextEntry` mirror in `src/lib/tauri.ts` gains the two fields.

**Dialog as a new `src/dialogs/GenerationParamsDialog.tsx`.** Follows the existing dialog patterns (Mantine components, i18n `tt()`, tests with `transitionProps={{ duration: 0 }}` where transitions would block jsdom). Menu item placed between "Сохранить аудио как…" and "Перегенерировать аудио"; enabled when `generation !== null || audio_generated_at !== null`. Localized engine names reuse dedicated short keys (`settings.engine.*` carries the settings-specific «рекомендуемый» suffix); Piper voice labels reuse `PIPER_VOICES` keys from `src/lib/piperVoices.ts`.

## Risks / Trade-offs

- [Snapshot slightly bloats `history.json` (~300 bytes/entry)] → negligible against existing texts; fields stay flat and optional.
- [`model_info()` reads `manifest.json` per synthesis] → one small file read on an already I/O-heavy path; a read/parse failure degrades to `null` and never fails synthesis.
- [Engine switched by startup fallback between config load and synthesis] → the snapshot keys on `tts.kind()` at synthesis time (same rule `synthesize_audio` uses for the voice), so it records what actually produced the audio.
- [A snapshot written by a newer build may contain fields an older build drops] → serde ignores unknown keys (existing behavior, covered by config tests); older builds still parse the entry.
- [Cyrillic config values (`code_block_mode`) are ASCII enums] → serialized as `"skip"`/`"read"`; the dialog localizes them for display only.

## Migration Plan

None required: additive optional fields only. Rollback is safe — older builds ignore the unknown `generation`/`generation_count` keys.

## Open Questions

None.
