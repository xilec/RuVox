# Tasks: silero-native-default-engine

## Implementation

- [x] 1. `src-tauri/src/storage/schema.rs`: change `default_engine()` → `"silero_native"`, `default_speaker()` → `"aidar"`, `default_sample_rate()` → `24000`; update the field doc comments (engine default, shared sample-rate note) and the schema tests asserting the old defaults.
- [x] 2. `src/dialogs/Settings.tsx`: update `initialValues` (`engine: 'silero_native'`, `speaker: 'aidar'`, `sample_rate: 24000`), the `sampleRateTouchedRef` comment, and the Piper option label (drop «по умолчанию»).
- [x] 3. Doc comments: `src-tauri/src/lib.rs` (`build_engine` — `engine = "piper"` no longer the default), `src/lib/tauri.ts` (UIConfig engine doc), `ai/rules/conventions.md` (engine-roles line: Silero Native is the default, Piper the fallback).
- [x] 4. Frontend tests: adjust `src/lib/engineSelection.test.ts` / any test asserting piper-as-default if they rely on the changed defaults.

## Verification

- [x] 5. `nix develop -c just test` (Rust + TS + Python) green.
- [x] 6. `nix develop -c just lint` green.
- [x] 7. `nix develop -c pnpm dlx @fission-ai/openspec validate --specs --strict` green.
- [x] 9. Reviewer fixes: Settings save omits `engine` while coerced (`buildSettingsPatch` + unit tests), `save_and_load_config` uses non-default values, CHANGELOG entry for the default switch; `ui` spec delta for the coerce-save behavior.
- [ ] 8. Manual check: run the app with the current user config (explicit values — must be unaffected), then with a config whose `engine`/`speaker`/`sample_rate` keys are removed, and confirm it starts on Silero Native / aidar / 24000 (bundle present on this machine).
