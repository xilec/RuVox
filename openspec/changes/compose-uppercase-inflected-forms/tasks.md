# Tasks: compose-uppercase-inflected-forms

## 1. Normalizer helper

- [x] Extract the bounded inflection-composition path of
      `EnglishNormalizer::normalize` into a helper (e.g.
      `compose_inflection`) returning `Option<String>`; keep `normalize`
      output identical.
- [x] Unit test: the helper composes `supporting` → "саппортинг",
      `building` → "билдинг", `features` → "фичас" and returns `None` for a
      pure abbreviation such as `html`.

## 2. Pipeline order

- [x] In `process_english_tracked`, consult the helper after the exact
      `IT_TERMS` hit and before the all-uppercase abbreviation branch.

## 3. Behavior lock

- [x] Pipeline test: `SUPPORTING` → "саппортинг", `BUILDING` → "билдинг",
      `FEATURES` → "фичас"; `API`/`CPU`/`JSON` readings unchanged; the composed
      all-caps form is not recorded in the unknown-words map.

## 4. Spec + docs

- [x] Apply the `text-pipeline` delta (case-independence + all-caps scenario).
- [x] Add a `CHANGELOG.md` `[Unreleased]` note.

## 5. Validation

- [x] `nix develop -c just test`
- [x] `nix develop -c just lint` — all steps pass except `cargo deny`, which
      fails on a pre-existing advisory (`zip 4.6.1` via `tauri-plugin-updater`,
      RUSTSEC-2026-0369) unrelated to this change and not run in CI.
- [x] `nix develop -c pnpm dlx @fission-ai/openspec@1.6.0 validate compose-uppercase-inflected-forms --strict`
- [ ] Archive the change (`openspec archive`) before the PR.
