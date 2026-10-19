# Tasks: restore-dropped-e-and-y-in-inflections

## 1. Silent-e restoration

- [x] In `EnglishNormalizer::compose_inflection`, after the direct `IT_TERMS`
      stem hit, retry with the stem plus a restored silent "e" (`us` → `use`,
      `cach` → `cache`) and compose from that base.
- [x] Gate the syllabic "-es" reading on a sibilant ending of the stem before
      the restored "e" so `uses` → "юзес" and `caches` → "кэшес" while
      non-sibilant bases keep the plain "-s" path (`features` → "фичас").
- [x] Exclude "-ally" from restoration (`practical` + "ly", not `practice`) so
      `practically` keeps "практикалли"; lock it with a unit test.

## 2. "-ies" from "-y"

- [x] Compose "-ies" only when `stem + "y"` is an `IT_TERMS` entry, dropping a
      trailing "й" from the base reading (`repositories` → "репозиторис").
- [x] Confirm `species` no longer composes from `spec` and falls back to raw
      transliteration.

## 3. Behavior lock

- [x] Unit tests for the issue table: `using` → "юзинг", `uses` → "юзес",
      `used` → "юзед", `caches` → "кэшес", `cached` → "кэшед",
      `repositories` → "репозиторис".
- [x] Regression tests: `features` → "фичас", `supports` → "саппортс",
      `building` → "билдинг"; `compose_inflection` still returns `None` for
      pure abbreviations (`html`, `API`, `json`) and for `species`.
- [x] Confirm unknown-words tracking still records only raw-fallback words
      (composed forms are not recorded).
- [x] Update the three intended `baseline.csv` rows (`driver`, `driving`,
      `packaging`) and record the drift in `design.md`.
- [x] Pin the behavior end to end with an `english_inflected_silent_e` golden
      pipeline fixture (mixed-case `using`/`uses`/`used`/`caches`/`cached`/
      `repositories`, plus `FEATURES` and `species` negatives).

## 4. Spec + docs

- [x] Apply the `text-pipeline` delta (restoration rules + scenarios).
- [x] Add a `CHANGELOG.md` `[Unreleased]` note.

## 5. Validation

- [x] `nix develop -c just test`
- [x] `nix develop -c just lint`
- [x] `nix develop -c pnpm dlx @fission-ai/openspec@1.6.0 validate restore-dropped-e-and-y-in-inflections --strict`
- [ ] Archive the change (`openspec archive`) before the PR.
