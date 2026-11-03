## 1. Year-aware native reader

- [x] 1.1 Add `english::native` (year register for 1000–2099, cardinal otherwise) with table tests: 3, 1900, 1905, 1999, 2000, 2024, 2099, 2100, 1234, 1 000 000.
- [x] 1.2 Return an explicit reading from `NumericSpanNormalizer::english_words` (cardinal value or digit-by-digit string) so the narrated and native renderings share one claim and one parse.

## 2. Native rendering

- [x] 2.1 On the `Native` route push the spelled replacement and keep recording the prose-Latin range over the digits; leave the narrated arm unchanged.
- [x] 2.2 Route tests: native text `In 2024 we shipped 3 releases.` → `In twenty twenty-four we shipped three releases.` with two spans, exact narrated output unchanged, Russian context unchanged.

## 3. Span and mapping

- [x] 3.1 Pipeline test: each spelled reading yields one prose-Latin span covering the replacement (no overlap with the neighbouring prose spans).
- [x] 3.2 `CharMapping` test: every word of a reading maps to the source digit range, and highlighting a word selects the whole reading.
- [x] 3.3 TeraTTS `segments_for` test: the processed sentence yields one `en` segment and no join.

## 4. Golden fixtures

- [x] 4.1 Regenerate `teratts/numbers_english` (spelled readings) and `teratts/shell_redirect` (`head -fifty`); keep `silero/numbers_english` byte for byte.
- [x] 4.2 Add a CHANGELOG `[Unreleased]` note.

## 5. Validation

- [x] 5.1 `cargo fmt --all` and `cargo clippy -p tts-normalize --all-targets -- -D warnings`.
- [x] 5.2 `cargo test --manifest-path tts-normalize/Cargo.toml`.
- [x] 5.3 `cargo test --manifest-path src-tauri/Cargo.toml --test golden --test golden_profiles`.
- [x] 5.4 `openspec validate spell-numbers-on-the-native-route --strict`.
- [x] 5.5 `cargo test --manifest-path src-tauri/Cargo.toml --test perf` within the 1 MB budget.
