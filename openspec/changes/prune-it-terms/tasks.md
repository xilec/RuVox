# Tasks: prune-it-terms

## 1. Audit

- [x] 1.1 Compare every `IT_TERMS` entry with `transliterate_simple` of the same
      word and classify the 228 entries: 155 pronunciation fixes, 7 semantic
      translations, 66 no-ops.
- [x] 1.2 For each no-op, compare the reading of every generated inflection
      (`s`, `es`, `ies`, `ed`, `ing`, `er`, `ers`, `able`, `ment`, `ments`,
      `ation`, `ally`) composed from the stem with the transliteration of the
      inflected word: zero differences.
- [x] 1.3 Compare each removed word's reading before and after removal: only
      the 7 semantic translations, `method`, and `living` changed.

## 2. Implementation

- [x] 2.1 Remove the 7 semantic translations and the 65 no-ops from
      `IT_TERMS`.
- [x] 2.2 Remove `method` so it reads "месод" (`th` → "с").
- [x] 2.3 Restore `living` with a comment explaining that it shadows the
      `live` + "-ing" composition.
- [x] 2.4 Add regression tests for the pruned words' readings and for their
      tracking as unknown words.
- [x] 2.5 Update the tests that referenced the removed entries
      (`it_terms_*`, `-ies` composition, dictionary `overrides_builtin`, the
      pipeline composed-forms case).

## 3. Fixtures and docs

- [x] 3.1 Regenerate `dictionary_baseline.csv` with `UPDATE_BASELINE=1`.
- [x] 3.2 Replace the `repositories`/`queries` rows in `inflected_forms.csv`
      with `libraries`, and update the golden fixture
      `english_inflected_silent_e`.
- [x] 3.3 Update the spec requirement where `repositories` was the "-ies"
      example.
- [x] 3.4 Add the `[Unreleased]` CHANGELOG note and align the #291 entry.

## 4. Gates

- [x] 4.1 `just test`, `just lint`, `just validate`.
- [x] 4.2 Manual listening check of `function`, `string`, `array`, `object`,
      `repository`, `boolean`, `refactor`, `method`, `class`, `index`.
