# Tasks: Fix number normalization bugs

## Implementation

- [x] Swap the Versions and Percentages phases in
  `TTSPipeline::process_with_char_mapping` (`src-tauri/src/pipeline/mod.rs`)
  so Percentages runs first (#76).
- [x] In `process_numbers_tracked` (mod.rs), replace the value-based
  `tracked.replace(original, &replacement)` call with
  `tracked.replace_byte_range(start, end, &replacement)` (#75).
- [x] Update golden fixture `percentage_decimal` to the corrected output
  (`Точность девяносто девять точка пять процентов.`) and regenerate its
  `char_map.json`.
- [x] Add a unit test next to
  `pipeline_invalid_time_falls_through_to_numbers`: `Счёт 10:1 в нашу
  пользу.` → `Счёт десять:один в нашу пользу.`, plus a `33 3` case.
- [x] Add a golden fixture `ratio_score` for the `10:1` case
  (input/expected/char_map.json).

## Validation

- [x] `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml` — all
  unit tests and golden fixtures pass; no fixture other than
  `percentage_decimal` changed.
- [x] `nix develop -c just lint` — fmt, clippy, typecheck, ruff all green.
- [x] `nix develop -c pnpm dlx @fission-ai/openspec@1.6.0 validate
  fix-number-normalization --strict` passes.
