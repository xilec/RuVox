## Context

The pipeline's English phase runs only for a `Narrate` Latin route and already
owns the per-word resolution order (`IT_TERMS` → inflection composition →
abbreviation rules → transliteration). The identifier phase runs before it and
owns camelCase/snake_case/kebab-case. A letter+digit token belongs to neither:
`re_english_words` requires a pure `[A-Za-z]+` run, and the number regexes
require a standalone digit run. The fragment therefore survives unchanged; for
Silero/`ttsd` the model alphabet drops it, and for a native engine it is
unannotated.

## Goals / Non-Goals

**Goals:**
- Narrate letter+digit tokens for a `Narrate` Latin route without regressing
  existing English-word or identifier readings.
- Leave such tokens to a `Native` route and annotate them for the adapter.
- Keep one owner per reading: curated tables stay the first choice, and the
  existing letter/number normalizers do the fallback work.

**Non-Goals:**
- URLs, versions, dates, ranges, units and other digit constructs already owned
  by earlier phases.
- Changing letter names or adding a new pronunciation table.
- Mixed-case identifiers (`getUserData2`); the identifier phase owns them.
- A per-token route field; the existing `latin_prose` route is the gate.

## Decisions

1. **Put the token in the English phase, not a new phase.** The phase already
   runs only for `Narrate` and already has the resolution order. A new phase
   would duplicate that order and complicate the fixed-phase-order contract.
2. **Whole-token reading first.** `IT_TERMS` contains `mp3` → "эм пэ три";
   splitting before the lookup would produce a worse "мп три". The mixed path
   checks the whole token, then splits.
3. **Split into maximal runs and use existing owners.** Letter runs use the
   same helper as pure words (single letter by name, `IT_TERMS`, inflection,
   abbreviation, transliteration); multi-letter code words consult
   `CODE_WORDS` (`sha` → "ша", `utf` → "ю ти эф"); digit runs use
   `NumberNormalizer::normalize_number`. This keeps the dictionaries as the
   single source of pronunciation.
4. **Exclude mixed-case tokens from the new pattern.** `getUserData2` must not
   be torn into `getUserData` + `2` before the identifier phase can read it.
   Lowercase and all-uppercase letter runs are safe; camelCase stays with the
   identifier rules.
5. **Annotate a native token as `CodeIdentifier`.** It is identifier-like, and
   the adapter's language segmentation sends it to the engine's native
   language. The number phase cannot split it because its digits are glued to
   letters, so no extra guard is needed.

## Risks / Trade-offs

- `p95` reads as "пи девяносто пять" because the shared letter-name table says
  "пи"; changing that table would affect every abbreviation. The issue's "пэ"
  spelling is not adopted to keep one canonical letter reading.
- `sha256` uses `CODE_WORDS` ("ша") rather than the generic transliteration
  ("сха"); this is the curated reading the identifier phase already uses.
- A mixed-case token with digits still falls through (e.g. `getUserData2`),
  because the identifier phase does not match it today. That is a separate
  identifier gap, not this change's scope.
