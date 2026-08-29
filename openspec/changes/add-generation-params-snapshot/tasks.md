## 1. Storage schema

- [x] 1.1 Add `ModelParams` and `GenerationParams` structs and the `generation` / `generation_count` fields (both `#[serde(default)]`) to `TextEntry` in `src-tauri/src/storage/schema.rs`; add schema tests: old-file parse (fields default to `null`/`0`), snapshot round-trip. Verify: `cargo test --manifest-path src-tauri/Cargo.toml storage`
- [x] 1.2 Clear `generation` in `delete_audio` and in the load-time audio validation (missing-file branch) in `src-tauri/src/storage/service.rs`, keeping `generation_count`; extend the existing service tests. Verify: `cargo test --manifest-path src-tauri/Cargo.toml storage`

## 2. Model identity from engines

- [x] 2.1 Add `ModelParams`-returning `model_info()` to the `TtsEngine` trait with a default `None` impl (`src-tauri/src/tts/engine.rs`, struct in `tts/mod.rs`); implement it for `SileroNativeEngine` (read `manifest.json`: `model_id` + `tts_main.onnx` sha256, `None` on read/parse failure), `PiperEngine` (voice model file name from the catalog); ttsd supervisor keeps the default. Verify: `cargo test --manifest-path src-tauri/Cargo.toml tts` incl. a unit test for the silero-native impl against a temp manifest

## 3. Snapshot capture

- [x] 3.1 Extract the engine→voice match from `synthesize_audio` into a shared `voice_for_engine` helper (`src-tauri/src/commands/mod.rs`); existing voice-selection tests must stay green. Verify: `cargo test --manifest-path src-tauri/Cargo.toml commands::tests::synthesize_audio`
- [x] 3.2 Add `build_generation_snapshot` (engine kind, voice, actual sample rate from the intermediate WAV via `hound`, `model_info()`, app version, normalization settings from config, normalized-text sha256, final-file codec + size) and store it in `apply_ready_if_current` inside the atomic `update_entry_if` closure with `generation_count += 1`; wire it into `spawn_synthesis` after `finalize_audio_files`. Verify: `cargo test --manifest-path src-tauri/Cargo.toml commands` — new tests: snapshot recorded on success with expected fields, count increments, stale completion does not resurrect a snapshot
- [x] 3.3 Regeneration path picks up the refresh automatically via `spawn_synthesis`; add a test that a second synthesis of the same entry overwrites voice and bumps the count. Verify: `cargo test --manifest-path src-tauri/Cargo.toml commands`

## 4. Frontend

- [x] 4.1 Extend the `TextEntry` TS mirror with `generation` / `generation_count` in `src/lib/tauri.ts` (types only). Verify: `pnpm typecheck`
- [x] 4.2 Add `src/dialogs/GenerationParamsDialog.tsx` (read-only rows: engine, voice, sample rate, model, app version, normalization settings, normalized-text checksum, audio codec/size, duration, generated-at, generation number; absent → «—»; legacy-entry explanatory line) + RU/EN strings in `src/i18n/`; add component tests (snapshot render, dash for absent model, legacy line, disabled gate). Verify: `pnpm test:unit`
- [x] 4.3 Add the «Параметры озвучки…» context-menu item in `src/components/QueueList.tsx` (between «Сохранить аудио как…» and «Перегенерировать аудио», enabled when `generation !== null || audio_generated_at !== null`) opening the dialog; extend `QueueList.test.tsx`. Verify: `pnpm test:unit`

## 5. Gates and wrap-up

- [x] 5.1 Full gates: `nix develop -c just lint && nix develop -c just test`; `openspec validate --specs --strict` after syncing deltas. Verify: all clean
- [ ] 5.2 Manual pass checklist handed to the user (synthesize → inspect dialog; regenerate → refreshed voice/count; legacy entry → explanatory line; pending entry → disabled item; RU/EN). Verify: user acceptance
