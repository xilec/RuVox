## Why

The manual listen pass over the English-context reading found six words whose
curated Cyrillic reading does not match how the English word sounds:

| Word | Current reading | Expected reading |
|---|---|---|
| the | «те» | «зэ» |
| and | «анд» | «энд» |
| three | «сри» | «фри» |
| thirty | «сёти» | «фёти» |
| hundred | «хандред» | «хандрэд» |
| percent | «персент» | «пёрсэнт» |

The numeric layer and the general English phase read these words from curated
tables, so the fix is data, not rules.

## What Changes

- The cardinal table reads "three" as «фри», "thirty" as «фёти» and "hundred"
  as «хандрэд»; the construct table reads "percent" as «пёрсэнт», the date
  article "the" as «зэ», and the ordinal derivatives "thirtieth"/"hundredth" as
  «фётис»/«хандрэдс».
- A new `FUNCTION_WORDS` table in the English normalizer curates the two
  function words the spelling rules get wrong ("the" → «зэ», "and" → «энд»)
  and is consulted before `IT_TERMS`.
- Only the Cyrillic readings change: the Latin words are untouched, so the
  native route, its fixtures and every `CharMapping` stay byte-identical.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `text-pipeline`: the English words requirement gains the `FUNCTION_WORDS`
  rule; the Dates and times, Ranges and percentages, and Numbers scenarios that
  pin a transliterated example are updated to the new readings.

## Non-goals

- Changing the spelling rules or the Latin wording of a construct.
- The native route and its fixtures.
- Readings of words outside the six above.

## Impact

- `tts-normalize/src/pipeline/normalizers/english.rs` (`FUNCTION_WORDS`) and
  `src/pipeline/mod.rs` (lookup order).
- `tts-normalize/src/pipeline/numeric/english/mod.rs` and
  `numeric/english/constructs.rs` (readings only).
- Silero golden fixtures with the affected words; the TeraTTS fixtures are
  unchanged.
