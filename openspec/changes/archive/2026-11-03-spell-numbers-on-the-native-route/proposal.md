## Why

The English-context rule (#369) reads a standalone number in English, but on the
native prose route the digits stay in the text and only a prose-Latin span marks
them. The TeraTTS profile declares `Digits::Expanded`, so leaving digits
contradicts the profile contract: the preview shows digits while the narration
says English words, and the spoken form is whatever the engine's English
frontend decides — "two thousand twenty-four", "twenty twenty-four", or nothing
at all when a frontend does not read digits. The pipeline already knows the
reading; it should write it down.

## What Changes

- On the native prose route an English-context number is replaced with the Latin
  spelling of its reading (`3` → `three`, `2024` → `twenty twenty-four`) and
  annotated with a prose-Latin span over the replacement, so the text continues
  the English block, matches what is spoken and no language flip appears at the
  number.
- A four-digit value from 1000 to 2099 uses the year register: two two-digit
  pairs ("twenty twenty-four", "nineteen ninety-nine"), "oh" for 01–09
  ("nineteen oh five") and "hundred" for 00 ("nineteen hundred"), while a round
  thousand keeps the cardinal ("two thousand").
- The narrated route and Russian context stay unchanged: the reader, the claim
  predicate and the transliteration stay where they are.
- The `CharMapping` maps every spelled word of the reading back to the source
  digit range, so highlighting a number selects the whole reading.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `text-pipeline`: the Numbers requirement states the native rendering as a
  spelled replacement; the Span annotation output requirement says the span
  covers the spelled words and the mapping maps them to the digits; the English
  reading setting requirement describes the route accordingly.

## Non-goals

- Constructs — units, percent, currency, dates, times, versions, ranges,
  ordinals — keep their current readings (#371, #372).
- The narrated register and its golden fixture stay byte for byte.
- Engine-side digit handling stays out of scope: the pipeline keeps deciding the
  spoken form.

## Impact

- Affected specs: `text-pipeline` (Numbers, English reading setting, Span
  annotation output).
- Affected code: `tts-normalize/src/pipeline/numeric/{mod,english}.rs`, the
  TeraTTS golden fixtures (`numbers_english`, `shell_redirect`), `CHANGELOG.md`.
- User-visible: the preview and the synthesis input show English words instead of
  digits for such numbers on a Latin-native engine; highlighting a number
  selects its whole reading.
