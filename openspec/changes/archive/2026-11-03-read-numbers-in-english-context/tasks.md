## 1. English numeral reader

- [x] 1.1 Add `numeric/english.rs` with the cardinal word table (0–999 999 999, no "and", digit by digit above the range) and table tests: 0, 13, 21, 100, 101, 1000, 2024, 1 000 000, 999 999 999, 1 000 000 000.
- [x] 1.2 Give every numeral word a curated Cyrillic reading in the reader's table and a test that every word the reader emits has one.

## 2. English-context detection

- [x] 2.1 Add a sentence-language map over the original text (Cyrillic presence per sentence; sentence ends at terminal punctuation followed by a break or at a paragraph break) with unit tests, including a markdown list marker that a later phase rewrites.
- [x] 2.2 Add the side classifier (whitespace, punctuation and digits transparent; English prose, non-English word or sentence edge) reusing `find_identifier_ranges` and the user dictionary; tests for English on both sides, English plus an edge, a Russian word on one side, an identifier neighbour, a dictionary token and a comma between numbers.

## 3. Numeric layer integration

- [x] 3.1 Extend `NumericSpanNormalizer::process` with a routing parameter (effective route, native language, English-prose predicate, reading lookup) and apply the English reading to plain-integer cardinals in English context.
- [x] 3.2 Narrated route: replace the span with the transliterated English reading, record no span, and leave the unknown-words map untouched.
- [x] 3.3 Native route: keep the digits and record a prose-Latin span with the native language through `record_byte_ranges`.

## 4. Pipeline plumbing

- [x] 4.1 Pass the resolved route, the native language, the English-prose predicate and the user dictionary from `process_for_engine` into the numeric phase.
- [x] 4.2 Route tests for both profiles: narrated output, native digits plus spans, Russian context unchanged, and the `english_reading` variants.

## 5. Golden fixtures and documentation

- [x] 5.1 Add `silero/numbers_english` and `teratts/numbers_english` golden fixture cases and generate them with `RUVOX_FIXTURE_CASE=... cargo test --test golden_profiles`.
- [x] 5.2 Add a CHANGELOG `[Unreleased]` note.
- [ ] 5.3 Manual listen check: TeraTTS reads "In 2024 we shipped 3 releases." as one English block with English digits, and Silero reads the transliterated cardinals.

## 6. Validation

- [x] 6.1 `cargo fmt --all` and `cargo clippy -p tts-normalize --all-targets -- -D warnings`.
- [x] 6.2 `cargo test --manifest-path tts-normalize/Cargo.toml`.
- [x] 6.3 `cargo test --manifest-path src-tauri/Cargo.toml --test golden --test golden_profiles`.
- [x] 6.4 `openspec validate read-numbers-in-english-context --strict` (CLI 1.6.0).
- [x] 6.5 `cargo test --manifest-path src-tauri/Cargo.toml --test perf` within the 1 MB budget.
