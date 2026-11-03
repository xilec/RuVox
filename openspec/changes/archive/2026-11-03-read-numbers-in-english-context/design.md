## Context

`tts-normalize/src/pipeline/numeric/` owns every numeric shape in one scan at
Phase 8: `parse::scan` finds candidates, `read` turns a shape plus the words
around it into a spoken form, and `NumericSpanNormalizer::process` replaces the
spans positionally. The layer runs before the identifier phase (15) and the
English phase (16), and it knows nothing about the effective prose route: every
cardinal comes out in Russian words.

The English phase runs after the layer and either transliterates Latin runs
(`Narrate`) or records them as prose-Latin spans (`Native`). A number has no
Latin to transliterate, so it is never covered by that decision — the source of
the defect.

Constraints: the numeric layer must stay a single linear scan (the pipeline has
a near-linear scaling requirement and a 1 MB perf budget); the layer has no
access to `recorded` spans; the normalized text carries no markup, only spans;
the narrated route must not pollute the dictionary's unknown-words map.

## Goals / Non-Goals

**Goals:**

- One engine-independent rule: a standalone cardinal in English context reads in
  English.
- Two renderings of the same reading: transliterated Cyrillic on the narrated
  route, digits in a prose-Latin span on the native route.
- Russian context byte-identical; one scan; no unknown-words pollution.
- Specs and fixtures that a unit test or a golden fixture can lock.

**Non-Goals:**

- Constructs (`50%`, `100 MB`, `1.2.3`, dates, ranges) — later changes.
- Spelling the reading as Latin text on the native route — #370.
- Comma-grouped thousands, code-like numbers, signed numbers, decimals.
- A year-specific register.

## Decisions

- **Context lives in the numeric layer (Phase 8), not in a new phase or a
  reordered pipeline.** The layer owns the spans and runs before the identifier
  and English phases, so it can decide per span and replace in the same pass.
  Rejected: moving the English phase before the numeric one — the English phase
  would then see digits and Russian unit words and would have to re-implement
  the construct classification; rejected: a post-pass that re-reads the English
  segments — it would need a second scan and would fight the layer's ownership
  of the digits.
- **Sentence language is computed over the original text, in one linear pass.**
  A phase-inserted Cyrillic fragment (a markdown list marker becomes "первое:")
  must not make an English sentence look Russian, so the map is built from
  `TrackedText::original` before the phases rewrite it, and each span is mapped
  back through `original_range`. Rejected: classifying on the current snapshot —
  the list-marker phase and the URL/inline-code phases have already rewritten
  parts of it.
- **Neighbours are classified on the current snapshot.** URLs, emails, paths and
  inline code are already consumed there, so they cannot masquerade as English
  prose; identifiers are recognised with the existing `find_identifier_ranges`
  and the user dictionary is consulted directly. Rejected: classifying on the
  original text — a URL would look like a Latin word. One exception proves the
  rule: a Cyrillic word the pipeline produced from digits or punctuation (the
  markdown list marker «1.» → «первое:») has no letters in its original range,
  so it is a label, not a neighbour, and the scan skips it — otherwise
  «1. 2024 was the year.» would read the item number in Russian. A Cyrillic
  reading of a URL, path or inline code keeps its Latin original and still
  vetoes the number. The scan is bounded by the sentence and paragraph edges
  plus a fixed window, so a run of integers cannot make the layer quadratic.
- **One reader, curated readings.** The English numeral words and their
  Cyrillic readings live in one table in `numeric/english` ("уан", "зиро",
  "сэвен" follow the `IT_TERMS` style), so the general transliteration rules
  cannot mispronounce "one", "two" or "four", the unknown-words map stays
  clean, and the native route's Latin words cannot drift from their narration.
  Rejected: extending `IT_TERMS` — the same words appear in English prose, so
  every fixture would change with a number fix; rejected: rule-based
  transliteration only — "one" would come out "оун".
- **The native rendering leaves the digits and records a prose-Latin span.**
  The engine reads the digits in English inside the English block; the
  `CharMapping` stays 1:1, which is also the best case for highlighting.
  Rejected: transliterated words on the native route — they would split the
  block exactly like the Russian numerals do; rejected: spelling Latin words now
  — that is #370's scope and it changes the text the preview shows.
- **The route is passed in, not re-derived.** The pipeline resolves
  `english_reading` + `reads_latin_natively` once and hands the numeric layer
  the effective route and the native language, the same way the English phase
  receives `narrate_latin`. Rejected: reading the profile inside the layer — two
  places resolving one policy.

## Risks / Trade-offs

- [The sentence guard can mis-split on abbreviations with dots] → the guard only
  needs "any Cyrillic word in the sentence"; a wrong split usually keeps the
  Cyrillic word on the number's side, and every case in the fixtures is checked
  against the expected reading.
- [TeraTTS digit reading inside an `en` segment is unverified] → the manual
  listen check and a probe note in the task; #370 removes the dependency by
  spelling the reading as text.
- [A new span kind source at Phase 8 could overlap identifier spans] → the
  number is a bare integer, the identifier patterns cannot cover digits, and
  `keep_specific_spans` resolves same-range overlaps; a test asserts no overlap.
- [Perf: one extra regex pass for identifier ranges] → measured against the
  1 MB budget; the sentence map is one linear scan.

## Migration Plan

No data or config migration: the behavior follows the active route, which is
already resolved per synthesis. Rollback is the revert of the change branch.

## Open Questions

- Year register ("twenty twenty-four" vs "two thousand twenty-four") — deferred;
  the first cut reads cardinals, the same reading feeds #370.
