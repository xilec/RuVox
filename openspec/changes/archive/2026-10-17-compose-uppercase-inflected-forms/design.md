# Design: compose-uppercase-inflected-forms

## Context

`EnglishNormalizer::normalize` resolves a word in order: multi-word phrases,
exact `IT_TERMS`, bounded inflection composition (recognized suffix + curated
`IT_TERMS` stem), then digraph transliteration. The pipeline does not call
`normalize` for every word: `process_english_tracked` resolves a word itself and
checks the all-uppercase abbreviation branch before falling back to `normalize`,
so all-uppercase inflected forms never reach composition.

## Decisions

- Reuse the normalizer's composition path through a dedicated helper instead of
  duplicating the suffix list and stem lookup in the pipeline — one source of
  truth for the phonetics.
- The helper returns `Option<String>` and fires only for a recognized suffix with
  a curated stem. Arbitrary words are not transliterated by it, so the pipeline
  keeps letter-by-letter spelling for non-inflected tokens and leaves the
  unknown-words map untouched for them.
- Call the helper after the exact `IT_TERMS` hit and before the all-uppercase
  branch, matching the specification order. This makes all-caps consistent with
  the already-working mixed-case path (mixed-case "APIs" already composes via
  `normalize`).
- Keep `normalize()` output identical: the change is a pure extraction plus a new
  call site, so the canonical baseline must not drift.

## Rejected alternatives

- Call `normalize(word, true)` for every word before the all-uppercase branch:
  rejected — it would transliterate unknown abbreviations as words and record
  them as unknown words, changing `API`/`HTML`/`SQL`/`JSON`-style behavior.
- Move the all-uppercase branch after the whole `normalize` call: rejected for
  the same reason, and it would let `AS_WORD`/transliteration outrank pure
  abbreviations.
