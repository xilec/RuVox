## 1. Contract and profiles

- [x] 1.1 Replace `Route.latin_prose` with `TextProfile.reads_latin_natively`;
  update `Route::any_native`, the doc comments and the invariant list.
- [x] 1.2 `TextProfile::validate`: require `"en"` for the capability, keep the
  non-Russian-language rule for the remaining native routes, and require the
  capability for `identifier: Native`; rename the identifier error variant.
- [x] 1.3 Profile unit tests: capability without English, identifier route
  without the capability, native code without native Latin reading, the matrix
  of valid combinations.
- [x] 1.4 Registry: set the flag on the four engine profiles and the test-only
  `native_en`; update the per-engine route tests and the Silero lockstep test.

## 2. Pipeline

- [x] 2.1 Add `EnglishReading` (`Auto` | `Native` | `Narrated`) with
  `from_config` / `as_config_str` / `resolve(capability) -> Reading`; unit-test
  the resolution matrix.
- [x] 2.2 Hold the setting in `TTSPipeline` (`set_english_reading`,
  accessor) and resolve the prose route once per `process_for_engine` run;
  gate the prose span recording and the English block on the resolved route.
- [x] 2.3 Keep native identifiers verbatim when prose is narrated: exclude
  identifier matches from the English phase; test both directions (prose
  transliterated, identifier native with its span and mapping).
- [x] 2.4 `builtin_contains_for` uses the resolved route, so the dictionary
  editor's override marker follows the setting.
- [x] 2.5 Pipeline tests: `auto` on a capable and an incapable profile,
  `native` fallback, `narrated` on TeraTTS (text, spans, `CharMapping`),
  the identifier protection, and the unchanged `native` path.

## 3. Config and commands

- [x] 3.1 `UIConfig.english_reading` + `UIConfigPatch` field, default
  `"auto"`, `canonical_english_reading`; canonicalize on load (storage service)
  and on patch (`apply_config_patch`).
- [x] 3.2 Replace `apply_configured_code_block_mode` with one
  `apply_configured_text_settings` applying both values; call it at startup
  and after every `update_config` save so the setting is live.
- [x] 3.3 Expose `reads_latin_natively` in `EngineAvailability` from the
  profile registry; probe test for all four engines.
- [x] 3.4 Tests: config serde/default/unknown value, the apply-settings
  mapping, and a live `update_config` synthesis test (rule resolution reaches
  the next run).

## 4. Frontend

- [x] 4.1 `UIConfig.english_reading` and `EngineAvailability.reads_latin_natively`
  in `src/lib/tauri.ts`.
- [x] 4.2 Settings: «Английский текст» segmented control bound to the setting,
  with the fallback hint when the selected engine cannot read Latin natively.
- [x] 4.3 RU/EN catalog strings for the label, the three choices, the
  description and the fallback hint.
- [x] 4.4 Settings tests: the control is initialized from the config, saving
  submits the chosen value, the hint appears for an incapable engine and not
  for a capable one.

## 5. Fixtures and docs

- [x] 5.1 Golden harness: an optional `.english.txt` sidecar applied by the
  harness, the classifier and the writer helper.
- [x] 5.2 Add `teratts/english_narrated` (sidecar `narrated`) and
  `silero/english_native_fallback` (sidecar `native`), review the generated
  text and char map by hand.
- [x] 5.3 CHANGELOG `[Unreleased]` entry; README/README.en wording if it
  documents the per-engine English behaviour.
- [x] 5.4 `ipc-commands` delta: `UIConfig.english_reading`, the
  `EngineAvailability.reads_latin_natively` payload (stamped from the profile),
  and `update_config` applying the setting without a restart.

## 6. Validation

- [x] 6.1 `cargo fmt` and `cargo clippy -- -D warnings` for the touched crates.
- [x] 6.2 `cargo test` for `tts-contract` and `src-tauri` (units, golden,
  span invariants).
- [x] 6.3 `pnpm typecheck`, `pnpm test:unit`, eslint.
- [x] 6.4 `openspec validate --strict`, then sync and archive the change.
