## Context

#369 reads an English-context number in English and renders it by the effective
prose route: the narrated route transliterates the reading into Cyrillic, the
native route leaves the digits and annotates them as prose Latin. The native
route therefore breaks the `Digits::Expanded` contract of the profiles that use
it, the preview and the narration disagree, and the engine's English frontend
chooses the spoken form. The pipeline already owns the reading, so the native
route should spell it.

## Goals / Non-Goals

**Goals.** The native route spells an English-context number in Latin; the text
continues the English block, and highlighting the number selects the whole
reading. The narrated route and Russian context stay byte for byte.

**Non-goals.** Constructs (units, percent, currency, dates, times, versions,
ranges, ordinals) keep their readings. The reader's cardinal register, the claim
predicate and the transliteration do not change. Engine-side digit handling stays
out of the pipeline.

## Decisions

- **Spell at the point where the native rule already records the span.** The
  layer pushes a replacement with the spelled text and records the prose-Latin
  range over the same digits; `resolve_spans` widens a span to every output
  codepoint whose original range it covers, and `build_mapping` maps every
  replacement codepoint to the digit range. No new mapping machinery, and the
  position-mapping invariant "output codepoints of a replacement map to the full
  original range" does the work.
- **One span per number.** Phase 16 also records prose-Latin spans for the
  spelled words; all of them resolve to the same original range as the numeric
  span, and `keep_specific_spans` keeps the earlier one, so exactly one span per
  number covers the whole reading.
- **Year register for four-digit values the Russian reader treats as a year (1000–2199, the `parse::is_year` range).** Two two-digit pairs
  ("twenty twenty-four", "nineteen ninety-nine"), "oh" for 01–09 ("nineteen oh
  five"), "hundred" for 00 ("nineteen hundred"), cardinal for a round thousand
  ("two thousand"). Rejected: the cardinal everywhere — the engine's natural
  reading of a year is the pair form and the issue's acceptance asks for it;
  rejected: a year detector — the layer sees a bare integer, not a date
  component, so the register follows the value's shape.
- **One reader, two renderings.** `english.rs` keeps `cardinal` (Latin, cardinal
  register, used by the narrated route through `transliterate`) and adds
  `native` (year-aware). The claim predicate stays identical, so a number that
  is not claimed keeps today's reading.
- **Rejected: spelling on the narrated route.** It would need the spelled text
  transliterated again and duplicate the reader; the narrated route renders the
  cardinal directly.

## Risks / Trade-offs

- The year register is a heuristic: "1234 items" reads "twelve thirty-four" on
  the native route while the narrated route keeps "one thousand two hundred
  thirty-four". Documented; revisit if a count-heavy fixture appears.
- TeraTTS golden fixtures with English-context numbers change (`numbers_english`,
  `shell_redirect`) — intended, regenerated in the same change.
- A user-dictionary entry over such digits cannot override the spelled reading;
  dictionary keys are not prose and are skipped by the claim rule anyway.

## Migration Plan

No storage, config or setting change: the behaviour follows the existing route
decision, so an installation updates in place.

## Open Questions

None.
