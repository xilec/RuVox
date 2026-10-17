# Proposal: compose-uppercase-inflected-forms

## Why

`process_english_tracked` checks the all-uppercase abbreviation branch before it
consults `EnglishNormalizer::normalize`, where bounded inflection composition
lives. An all-uppercase inflected form of a known IT term is therefore spelled
letter by letter instead of composing the curated base reading:

- `SUPPORTING` → "эс ю пи пи оу ар ти ай эн джи" (expected "саппортинг")
- `BUILDING` → "би ю ай эл ди ай эн джи" (expected "билдинг")
- `FEATURES` → spelled out (must read like lowercase "features" → "фичас")

The `text-pipeline` specification already orders inflection composition before
the all-uppercase abbreviation rule, so code and spec disagree.

## What Changes

- Expose the bounded inflection-composition path of `EnglishNormalizer::normalize`
  as a reusable helper that returns a reading only when the word ends in a
  recognized inflection suffix and its stem is a curated `IT_TERMS` entry.
- Consult that helper in `process_english_tracked` after the exact `IT_TERMS`
  hit and before the all-uppercase abbreviation branch, so an inflected form
  composes in any letter case.
- Keep exact `IT_TERMS` matches first and pure abbreviations letter-by-letter;
  `normalize()` output stays identical (the helper is pure extraction).
- Add a `text-pipeline` scenario for the all-caps inflected form and lock the
  behavior with unit tests.

## Non-goals

- No change to the suffix list, the stem requirement, or the phonetic reading of
  any suffix.
- No change to the all-uppercase abbreviation path for tokens that are not
  inflected forms of a known term (`API`, `HTML`, `SQL`, `JSON`, `CPU` keep their
  current readings).
- No dictionary-content changes; the semantic/no-op cleanup is tracked separately.

## Capabilities

### Modified Capabilities
- text-pipeline: the inflection-composition rule is stated to apply independently
  of letter case and to precede the all-uppercase abbreviation rule, with a
  scenario locking the all-caps case.

## Impact

- `src-tauri/src/pipeline/normalizers/english.rs` — helper extraction
- `src-tauri/src/pipeline/mod.rs` — `process_english_tracked` resolution order
- `openspec/specs/text-pipeline/spec.md` — delta
- `src-tauri/src/pipeline` tests, `CHANGELOG.md` `[Unreleased]`
