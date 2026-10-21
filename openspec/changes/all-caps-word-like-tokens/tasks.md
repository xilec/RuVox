# Tasks: all-caps-word-like-tokens

## 1. Implementation

- [x] 1.1 Add `is_word_like` (length floor + one vowel per four letters, "y"
  counted) and the table-aware `AbbreviationNormalizer::reads_as_word` in
  `src-tauri/src/pipeline/normalizers/abbreviations.rs`.
- [x] 1.2 In `process_english_tracked`, read a word-like all-caps token through
  `EnglishNormalizer::normalize(word, true)` and keep the letter-by-letter
  branch for everything else.
- [x] 1.3 Curate `windows` → "виндовс" and `ascii` → "аски" in `IT_TERMS` so the
  heuristic never mispronounces them.

## 2. Tests

- [x] 2.1 Unit-test the calibration table: `DEVELOPER`, `LINUX`, `UBUNTU`,
  `WINDOWS`, `JAVASCRIPT`, `SYSTEM`, `BROWSER` are word-like; `API`, `HTML`,
  `SQL`, `JSON`, `GPU`, `JWT`, `NGINX`, `SPARK` are not.
- [x] 2.2 Pipeline tests: word-like tokens read as words and are tracked as
  unknown; short/vowel-poor tokens stay spelled (`SPARK` locks the threshold
  itself); `KUBERNETES`, `OAUTH`, `WINDOWS` (IT_TERMS) and `MACOS`
  (SPECIAL_CASES) keep their curated readings and are not tracked.
- [x] 2.3 Golden fixture `english_word_like_all_caps` (input, expected,
  char_map) for the mixed-sentence case.
- [x] 2.4 `CHANGELOG.md` entry under `[Unreleased]`.

## 3. Gates

- [x] 3.1 `just test`
- [x] 3.2 `just lint`
- [ ] 3.3 `just validate` after archiving the change.

## 4. Manual pass

- [ ] 4.1 Listen to the calibration sentences and confirm the readings:
  `Собрали DEVELOPER и JAVASCRIPT.`, `LINUX и UBUNTU`, `Через API и HTML`,
  `SYSTEM WINDOWS`, `KUBERNETES OAUTH`. Also confirm the collateral tokens
  now read as words: `FIXME`, `README`, `BROWSER`.
