# Design: unify-letter-tables

## Context

`LETTER_MAP` (`abbreviations.rs`, 26 entries, HashMap) and `letter_name`
(`code.rs`, match on uppercase char) duplicate the same knowledge — English
letter names — with drift in exactly three letters (x/y/z). Callers:

- `AbbreviationNormalizer`: unknown all-caps words length ≥ 2 → spelled via
  `LETTER_MAP`.
- `CodeIdentifierNormalizer::spell_abbreviation`: all-caps parts of code
  identifiers; also called by the English phase for lone single letters in
  prose (#109).

## Goals / Non-Goals

**Goals:**

- One canonical letter-name table, one home (per `ai/rules/code-quality.md`:
  "Dictionaries and normalization tables have one home").
- Canonical x/y/z readings "икс/вай/зет" (decision: variant В2 of the #120
  discussion — smallest behavior radius, consistent with #109 and with
  `IT_TERMS` "xml" → "икс эм эль").

**Non-Goals:**

- Touching readings other than x/y/z; touching `AS_WORD`, `IT_TERMS`.
- Changing behavior of lone letters / identifiers (stays "икс/вай/зет").

## Decisions

### Table lives in `english.rs`

`pub(crate) static LETTER_NAMES: [(char, &str); 26]` next to `IT_TERMS` and
the transliteration tables — `english.rs` is the existing home of
pronunciation data. A sorted array with lookup (linear scan over 26 entries
is fine and keeps it dependency-free) rather than another `HashMap`.

### Both consumers delegate

- `abbreviations.rs`: `LETTER_MAP` deleted; the spelling path calls the
  shared lookup. Fallback for non-listed characters stays as today
  (verbatim) — unreachable for ASCII letters.
- `code.rs`: `letter_name` becomes a lookup into the shared table; the
  `_ => "?"` arm stays for non-letters (existing behavior).

### Behavior change is limited to unknown caps abbreviations

"UX", "XSS", "WXYZ" and similar change to "икс/вай/зет" readings. Everything
else (lone letters, identifiers, `IT_TERMS` entries) is byte-identical.

## Risks / Trade-offs

- Users accustomed to "экс уай зед" for abbreviations hear "икс вай зет" —
  accepted per the #120 decision; pinned by a delta-spec scenario.
