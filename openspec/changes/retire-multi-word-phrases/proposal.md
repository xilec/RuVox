# Proposal: retire-multi-word-phrases

## Why

`MULTI_WORD_PHRASES` (15 curated phrase readings) is unreachable in production
text: `process_english_tracked` resolves text with `re_english_words()` =
`\b([A-Za-z]+)\b` and calls `EnglishNormalizer::normalize` one word at a time,
so a phrase never reaches the normalizer as a whole string (#287).

Making the table reachable is the wrong fix, because the table does not encode
phrase-level knowledge. Comparing each curated row with the concatenation of its
words' readings shows that eleven of the fifteen rows are already exactly that
concatenation, and the remaining four differ only in five of their words:

| input | per-word | curated | word that differs |
|-------|----------|---------|-------------------|
| `stack trace` | стак трейс | стэк трейс | `stack`: стак / стэк |
| `live reload` | ливе релоуд | лайв релоуд | `live`: ливе / лайв |
| `dry run` | дри рун | драй ран | `dry`: дри / драй, `run`: рун / ран |
| `edge case` | едге кейс | эдж кейс | `edge`: едге / эдж |

A phrase entry is also a fragile way to fix a word: it only fires for the exact
separator the matcher knows, so `stack\ntrace` still reads "стак\nтрейс" and the
reading depends on invisible formatting. The reading belongs to the word, not to
the gap between words.

## What Changes

- Curate the five words that need it as ordinary `IT_TERMS` entries:
  `stack` → "стэк", `live` → "лайв", `dry` → "драй", `run` → "ран",
  `edge` → "эдж".
- Retire the phrase table: delete `MULTI_WORD_PHRASES`, the exact-phrase lookup
  in `EnglishNormalizer::normalize`, its whole-phrase test cases, and the
  multi-word rows of the transliteration baseline (the baseline is a word-level
  fixture).
- Make the spec tell the truth: multi-word input has no reading of its own, the
  phrase-first claim and its scenario are replaced by "words are curated
  individually" and "the separator never changes the reading".
- Pin the readings with unit tests for all fifteen former phrases and their five
  curated words, pipeline tests (space, doubled space, line break, user
  dictionary precedence), and a golden fixture.

## Non-goals

- No phrase-level feature at all: merged compounds (`front end` → "фронтенд"),
  heteronym disambiguation by context, and user-dictionary entries with spaces
  are separate changes with separate designs; the last one is a follow-up issue.
- No changes to `IT_TERMS` entries other than these five additions, and none to
  inflection composition, abbreviations, or the user-dictionary pre-pass.
- No transliteration-algorithm change: the digraph rules stay as they are, so
  the five words are curated rather than derived (#282 owns the reading audit).

## Capabilities

### Modified Capabilities
- text-pipeline: the English phase reads every word individually; the curated
  phrase table and its phase-order clause are removed, and the words that relied
  on a phrase entry become curated single words.

## Impact

- `src-tauri/src/pipeline/normalizers/english.rs` — five `IT_TERMS` entries, the
  phrase table and its lookup removed, unit tests
- `src-tauri/src/pipeline/mod.rs` — pipeline tests for the former phrases
- `src-tauri/tests/fixtures/pipeline/english_phrases_words.*` — new golden fixture
- `src-tauri/tests/fixtures/transliteration/baseline.csv` — fifteen multi-word
  rows removed, four word rows updated
- `openspec/specs/text-pipeline/spec.md` — delta
- `CHANGELOG.md` `[Unreleased]`
