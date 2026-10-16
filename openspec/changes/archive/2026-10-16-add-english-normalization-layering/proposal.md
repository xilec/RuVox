# Proposal: add-english-normalization-layering

## Why

The English normalizer gained an explicit layered resolution model in the
previous change: multi-word phrases, `IT_TERMS`, abbreviations, `AS_WORD`
entries, and digraph transliteration. It also gained a soft-c pre-pass
before raw transliteration. This behavior currently lands in implementation
and fixtures, but the public `text-pipeline` specification only describes
`IT_TERMS` followed by abbreviations and transliteration, so the actual
resolution order and fallback phonetics are not specified.

## What Changes

- Document multi-word phrase resolution before `IT_TERMS`.
- Document the `IT_TERMS` curated readings and the fact that they are used
  before abbreviations, `AS_WORD`, and transliteration.
- Document the soft-c pre-pass (`ce` -> `се`, `ci` -> `си`, `cy` -> `си`)
  applied to alphabetic runs before digraph transliteration.
- Document compilation of unknown English words via the unknown-words map.
- Document representative edge cases and existing continuation behavior.

## Capabilities

### Modified Capabilities
- text-pipeline: English-word resolution becomes explicitly layered and its
  fallback phonetics are specified.

## Impact

- `openspec/specs/text-pipeline/spec.md`
- implementation and tests are unchanged by this docs-only change
