# Proposal: add-english-inflected-forms

## Why

RuVox's English normalizer already handled base forms of IT terms via `IT_TERMS`, but common inflected variants (`supported`, `supporting`, `features`, `building`, `nullable`) fell through to the raw digraph fallback or were spoken inconsistently. This made spoken English nouns/verbs lose their curated canonical readings, even when the base word was known. Adding bounded morphology handling gives a competent, consistent reading for common forms without tracking every possible form in the dictionary.

## What Changes

- Extend `EnglishNormalizer::normalize` to compose a canonical reading when an English word ends in a common inflection suffix and its stem appears in `IT_TERMS`: `base reading` + `phonetic reading of the suffix`.
- Keep exact `IT_TERMS` matching first, so an explicit form-specific reading always overrides derivation.
- Keep raw transliteration as the final fallback for unknown bases and for words that do not end in a supported suffix.
- Add a compiled-in `sync` term, preserving the existing `sync` reading, so inflected variants resolve consistently instead of drifting through soft-c rules.
- Add representative unit coverage and regenerate the canonical baseline and golden expected readings to lock the changed outputs.

## Capabilities

### Modified Capabilities
- text-pipeline: English word resolution gains bounded derivative-form composition for known IT-term bases, while preserving the existing mixed/abbreviation/transliteration fallback order.

## Impact

- `src-tauri/src/pipeline/normalizers/english.rs`
- `src-tauri/tests/baseline_transliteration.rs` fixture data
- `src-tauri/tests/fixtures/pipeline/snake_case.*`
- `CHANGELOG.md`
