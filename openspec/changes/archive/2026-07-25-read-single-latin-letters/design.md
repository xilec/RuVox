# Design: read-single-latin-letters

## Context

The English-words phase (`process_english_tracked` in
`src-tauri/src/pipeline/mod.rs`) matches words with
`re_english_words = \b([A-Za-z][A-Za-z]+)\b` — the `[A-Za-z]+` tail requires
at least 2 letters. Single Latin letters (`a`, `I`, `x`) never match, stay
Latin in the output, and are silently dropped by Silero during synthesis.
The pipeline spec mandates Cyrillic-only output (the TTS constraint).

Letter-name pronunciation already exists in two places:
`AbbreviationNormalizer`'s `LETTER_MAP` (used for all-caps words ≥ 2) and
`CodeIdentifierNormalizer::spell_abbreviation` (single source of the same
English letter names inside identifiers). Per the project rule
"dictionaries have one home", the single-letter path must reuse an existing
table, not grow a third copy.

## Goals / Non-Goals

**Goals:**
- Single Latin letters in prose are read by English letter name
  (`a` → `эй`, `I` → `ай`, `x` → `икс`), case-insensitively.
- No Latin remains in pipeline output for inputs like "Переменная x равна 5".
- Behavior pinned by golden fixtures + unit tests.

**Non-Goals:**
- Changing multi-letter word resolution order or pronunciation.
- Single letters inside code identifiers (owned by the earlier
  code-identifier phase).
- User-configurable letter names (issue #10).

## Decisions

1. **Match single letters in the same phase, via the same regex.**
   Widen `re_english_words` to `\b([A-Za-z]+)\b` and branch on length-1
   matches in the collection closure.
   *Alternative:* a separate single-letter phase/regex — rejected: a second
   pass over the same text duplicates the replace-by-byte-range machinery and
   risks ordering surprises; the branch keeps one matching site.
2. **Pronounce via the identifier letter-name table, not transliteration.**
   Single letters are read by name ("икс", not "кс") — the established spoken
   convention for lone letters in code/technical prose (confirmed with the
   user). Implementation: route length-1 matches through
   `CodeIdentifierNormalizer::spell_abbreviation` (made `pub`), so lone
   letters in prose sound the same as letters inside identifiers.
   *Alternative 1:* digraph transliteration (`x` → `кс") — rejected: "кс" is
   not how a lone variable is read aloud.
   *Alternative 2:* `AbbreviationNormalizer` (`LETTER_MAP`) — rejected: its
   table has drifted to British readings (`x` → "экс", `y` → "уай",
   `z` → "зед") versus the identifier table (`икс`/`вай`/`зет`) and the
   approved behavior; unifying the tables changes multi-letter abbreviation
   pronunciation and is out of scope (see Open Questions).
3. **Not tracked as unknown words.** Letter-name spelling is a deterministic
   dictionary lookup, not a transliteration fallback, so it does not enter
   the unknown-words map (consistent with IT_TERMS / abbreviations).
4. **Word-boundary semantics unchanged.** `\b` already prevents matching
   inside longer tokens; digits-adjacent letters like "v1" are unaffected
   (the letter matches only when bounded, same as before).

## Risks / Trade-offs

- [Over-matching: legitimate single-letter artifacts like "e" in "e.g." or
  a stray "A" article get spelled] → Accepted: the pipeline targets technical
  prose and the previous behavior (raw Latin to Silero) was strictly worse —
  it produced silence. No Latin in output is the hard constraint.
- [Widening the regex changes match set for `process_english_tracked`] →
  Mitigated by golden fixtures for single-letter cases and a full golden
  suite re-run; multi-letter behavior is untouched by the branch.

## Open Questions

- `LETTER_MAP` (abbreviations.rs) and the identifier letter table (code.rs)
  have drifted (`x` → "экс" vs "икс", `y` → "уай" vs "вай", `z` → "зед" vs
  "зет"). Which reading is canonical for unknown all-caps abbreviations?
  Follow-up issue; this change only picks the identifier table for lone
  letters in prose.
