## 1. Backend catalogue

- [x] 1.1 Add `tts::teratts::voices(bundle_dir) -> Vec<String>` that reads the manifest with `teratts::bundle::Manifest::load` and returns `voices[].id` in manifest order, yielding `[]` on any load error
- [x] 1.2 Unit-test the helper: a manifest with Russian + English voices keeps manifest order; a missing bundle and an unsupported `format_version` both yield an empty list
- [x] 1.3 Add the `get_teratts_voices` Tauri command (spawn_blocking over the resolved `teratts_bundle_dir`), register it in `lib.rs`, and cover the command contract with a test

## 2. Frontend selector

- [x] 2.1 Add the `getTerattsVoices()` wrapper to `src/lib/tauri.ts`
- [x] 2.2 Replace the hardcoded `TERATTS_VOICES` in `Settings.tsx` with the fetched catalogue: load it on dialog open and on the model-inventory refresh callback, fall back to the saved id when the catalogue does not list it
- [x] 2.3 Reword the `settings.teratts_voice.description` strings in `ru.ts` / `en.ts` (voices are no longer only Russian)
- [x] 2.4 Add Settings tests: every reported voice (including English ids) is offered and the saved one is preselected; a saved id missing from the catalogue stays selectable and survives save

## 3. Records

- [x] 3.1 Add the `[Unreleased]` CHANGELOG entry (English voices selectable)
- [x] 3.2 Validate the change specs, then sync and archive the change

## 4. Validation

- [x] 4.1 Run `cargo fmt` and `cargo clippy -- -D warnings` for `src-tauri`
- [x] 4.2 Run `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 4.3 Run `pnpm typecheck`, `pnpm test:unit`, and eslint/knip
- [x] 4.4 Run `pnpm dlx @fission-ai/openspec@1.6.0 validate --specs --strict` after archiving
- [x] 4.5 Confirm the catalogue path is covered end to end (manifest -> IPC -> selector) by the Rust IPC test and the Settings test; a live pass is deferred because the TeraTTS bundle is not installed in this workspace
