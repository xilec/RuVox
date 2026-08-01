# Proposal: unify-letter-tables

## Why

Two letter-name tables for English letters have drifted apart
(#120): `LETTER_MAP` in `abbreviations.rs` reads unknown all-caps
abbreviations with x/y/z as "экс/уай/зед", while `letter_name` in `code.rs`
(used for code identifiers and, since #109, lone letters in prose) reads
them "икс/вай/зет". The same letter sounds different depending on code path,
and the duplication violates the project rule that pronunciation tables have
one home (`ai/rules/code-quality.md`). `IT_TERMS` already leans to the
"икс" style ("xml" → "икс эм эль").

## What Changes

- **BREAKING (pronunciation)**: unknown all-caps abbreviations containing
  x/y/z change reading: "экс/уай/зед" → "икс/вай/зет" ("UX" → "ю икс",
  "XSS" → "икс эс эс"). Lone letters and identifiers keep their current
  reading (no change).
- The two tables are merged into a single shared `LETTER_NAMES` table in
  `english.rs` (the home of `IT_TERMS` and the transliteration tables);
  `abbreviations.rs` and `code.rs` both use it.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `text-pipeline`: the "English words, abbreviations, and transliteration"
  requirement now names one shared letter-name table with the canonical
  x/y/z readings "икс/вай/зет".

## Impact

- `src-tauri/src/pipeline/normalizers/english.rs` — new shared table.
- `src-tauri/src/pipeline/normalizers/abbreviations.rs` — `LETTER_MAP`
  removed, table imported; unit-test expectations updated (x/y/z readings).
- `src-tauri/src/pipeline/normalizers/code.rs` — `letter_name` delegates to
  the shared table; behavior unchanged.
- Golden fixtures unaffected (caps abbreviations in fixtures resolve via
  `IT_TERMS`, e.g. "xml" → "икс эм эль").

## Non-goals

- Changing any reading other than x/y/z in unknown all-caps abbreviations.
- A stop-list for over-matched artifacts ("e.g.", article "A") — noted in
  #120 as a possible ride-along, deliberately left for a separate change.
- Auditing `AS_WORD` or other dictionaries.
