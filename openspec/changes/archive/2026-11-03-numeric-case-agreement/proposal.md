## Why

The numeric span layer declines the numeral after a preposition but the noun next
to it had only nominative and genitive forms: "к 5 кг" read "к пяти килограммов",
"при 10 %" read "при пяти процентов". Numbers after к/ко/о/об/при are common in
articles ("к 5 кг", "о 1 млн", "при 10 %"), and the mixed case is audible.

## What Changes

- Units, currencies, percentages, scale words and fraction tails gain dative and
  prepositional noun forms, derived from the nominative base (regular declension)
  with an explicit fallback for indeclinable nouns ("евро") and fixed units
  ("эм", "рем"): the noun stays fixed while the numeral declines.
- An adjectival fraction keeps a following unit in the singular genitive
  ("около 1/2 кг" → "около одной второй килограмма"); a simple fraction declines
  the unit with the surrounding case.
- Dotted groups above the cardinal limit or unparsable groups read digit by digit
  instead of dropping high digits, consistent with the Numbers requirement.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `text-pipeline`: one added requirement, "Numeric noun case agreement".

## Impact

`tts-normalize/src/pipeline/numeric/` (lexicon, read, morphology), the corpus
(17 locked cases), no dependency or API changes.

## Non-goals

- Sandhi "о" → "об" before a vowel is outside the numeric layer.
- Mixed quarters ("к 2¼ кг") keep the current reading.
