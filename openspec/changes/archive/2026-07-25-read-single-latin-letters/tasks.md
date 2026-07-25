# Tasks: read-single-latin-letters

## 1. Implementation

- [x] 1.1 Widen `re_english_words` in `src-tauri/src/pipeline/mod.rs` from
  `\b([A-Za-z][A-Za-z]+)\b` to `\b([A-Za-z]+)\b` (single letters included).
- [x] 1.2 In `process_english_tracked`, branch length-1 matches to
  `CodeIdentifierNormalizer::spell_abbreviation` (make it `pub`) — letter-name
  spelling consistent with identifiers — before the
  IT_TERMS/uppercase/AS_WORD/transliteration chain; do not record them in the
  unknown-words map.

## 2. Tests

- [x] 2.1 Unit tests in `pipeline/mod.rs`: "Переменная x равна 5" →
  "Переменная икс равна пять"; "пункты a и I" → "пункты эй и ай"; single
  letters are not added to the unknown-words map.
- [x] 2.2 Golden fixture(s) in `src-tauri/tests/fixtures/pipeline/`:
  `english_single_letter` (input/expected/char_map) per the pipeline testing
  gate.
- [x] 2.3 Full gates green: `cargo fmt`, `cargo clippy --no-deps --
  -D warnings`, `cargo test` (unit + golden).

## 3. Spec

- [x] 3.1 `openspec validate read-single-latin-letters --strict` passes.
