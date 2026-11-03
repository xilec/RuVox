## Context

The numeric layer turns a construct into one span (`numeric/parse.rs`) and reads
it in Russian (`numeric/read.rs`). #369 added the English-context decision,
#370 made the native route spell a bare cardinal and annotate it, and #371 added
the English construct lexicon and wired it into the narrated arm only. The
native route still reads a construct as a Russian phrase, which splits the
English block at the engine boundary.

## Goals / Non-Goals

**Goals**

- A construct that #371 already reads in English is spelled as Latin English
  words on the native route and continues the English block.
- One span covers the whole replacement, and the mapping maps every spelled word
  back to the source construct, so highlighting selects the construct.
- The narrated output and every Russian-context reading stay byte-identical.

**Non-Goals**

- New wording: the native renderer is `constructs::read` without the
  transliteration step.
- Compound/mixed constructs, arithmetic, and any change to the segmenter.

## Decisions

- **The construct claim runs on both routes.** `needs_context` includes a
  construct regardless of the route, and the span claim no longer gates on
  `NumberRoute::Narrate`. The route decides the rendering, not the claim.
- **The native arm mirrors the bare-number arm.** `EnglishClaim::Construct`
  becomes `(String, EnglishNumber)`-like in the dispatch: the `Native` branch
  pushes the range into `claimed`, the Latin string into `replacements`, and the
  span range into `native_ranges`; `record_byte_ranges` then annotates the
  replacement with one `ProseLatin` span. `CharMapping` resolves each spelled
  word to the whole source construct, as it already does for a spelled number.
- **One lexicon, two renderings.** The narrated arm keeps
  `english::transliterate(&latin)`; the native arm pushes `latin`. No new table,
  no route flag inside `constructs`.
- **`expand_units` keeps its meaning.** A measurement construct is still claimed
  only when the profile expands units; the spelling path inherits that gate from
  `constructs::read`.
- **Russian context is untouched.** The claim only fires inside
  `english_context`; the Russian reader and the fallback branch are unchanged.

## Risks

- **A spelled construct near an identifier.** The Latin words could be
  re-scanned by the identifier phase; the `claimed` ranges (the same mechanism
  #370 uses for a spelled number) exclude them, and a test covers a construct
  beside `getUserData`.
- **Segment count.** One span per construct is the same shape as a spelled
  number, so the segmenter sees one `en` run; the engine test asserts a single
  `en` segment.
- **Mapping drift.** Every spelled word maps to the source range because the
  replacement is recorded like #370's; the pipeline test asserts the mapping
  covers the construct.

## Migration

No migration: profiles, settings and stored texts are unchanged. The behavior
change is confined to the native prose route.

## Open Questions

- Whether a spelled construct should keep the source punctuation (`50%` →
  `fifty percent,`): the punctuation stays the post-processing phase's job, as
  in #370.
