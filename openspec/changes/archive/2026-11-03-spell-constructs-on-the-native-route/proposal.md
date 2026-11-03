## Why

The native prose route now spells a bare English-context number as Latin English
words (#370), but the construct cases still read as a Russian phrase. TeraTTS
reads Latin natively, so a Russian fragment inside an English sentence is the
only reason the adapter splits the block into `en` / `ru` / `en`, and every flip
costs a 200 ms join:

| Input | Current normalized text |
|---|---|
| `the API grew 50%` | `the API grew пятьдесят процентов` |
| `shipped 100 MB` | `shipped сто мегабайт` |
| `version 1.2.3` | `version один точка два точка три` |
| `on 12.05.2024` | `on двенадцатое мая две тысячи двадцать четвёртого года` |

The narrated route already renders the same constructs in English (#371), so the
two routes disagree about the sentence language.

## What Changes

- The construct claim stops being narrated-only: on the `Native` route an
  English-context construct is replaced with its Latin English reading and
  carries one prose-Latin span over the whole replacement.
- The wording is the one #371 transliterates: one lexicon, two renderings. The
  native arm drops the transliteration step and emits the Latin string.
- The native-spelling machinery of #370 (the `claimed` ranges, the
  `native_ranges`, `CharMapping` over the replacement) is reused as is, so one
  span covers the construct and the mapping maps every spelled word back to the
  source range.
- Russian context, the narrated register and every construct outside the
  families of #371 keep their readings.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `text-pipeline`: the Sizes, durations, and versions; Dates and times; Ranges
  and percentages; Numbers; and Span annotation output requirements gain the
  native construct rendering.

## Non-goals

- Compound and mixed constructs (`$5 per MB`, `10–20% of 1 GB`) and arithmetic.
- Any change to the narrated register or to the construct wording itself.
- Changes to the segmenter: it already turns a prose-Latin span into an `en`
  segment.

## Impact

- `tts-normalize/src/pipeline/numeric/mod.rs`: the construct claim runs on both
  routes, and the `Native` arm records the replacement range and the prose-Latin
  span.
- `tts-normalize/src/pipeline/tests.rs`, the TeraTTS engine test and the golden
  fixtures gain the native construct cases.
- No new API, no profile field, no engine change.
