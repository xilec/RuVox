# Tasks: Read numbers before a sentence-ending dot

## Implementation

- [x] `process_numbers_tracked` (`src-tauri/src/pipeline/mod.rs:698`):
  tighten `followed_ok` — a trailing dot disqualifies the number only when
  the dot is directly followed by another digit.

## Tests

- [x] Unit tests: `Встреча в 5.` → `Встреча в пять.`; mid-sentence terminal
  dot (`Пункт 3. Далее`); `3.14` fragment keeps both parts unexpanded;
  `5.5` skipped as before.
- [x] Golden fixture `number_sentence_dot` (input/expected/char_map.json).
- [x] Confirm no existing golden fixture changes output.

## Validation

- [x] `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml` green.
- [x] `nix develop -c just lint` green.
- [x] `nix develop -c pnpm dlx @fission-ai/openspec@1.6.0 validate
  number-before-terminal-dot --strict` green.
