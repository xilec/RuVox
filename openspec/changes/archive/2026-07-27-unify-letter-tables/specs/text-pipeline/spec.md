# Delta spec: text-pipeline

## MODIFIED Requirements

### Requirement: English words, abbreviations, and transliteration

The system SHALL replace every remaining English word with speakable
Cyrillic. Special language names `C++`, `C#`, `F#` (any case) SHALL be
replaced first ("си плюс плюс", "си шарп", "эф шарп"). Single Latin letters
(length-1 runs) SHALL be read by their English letter names via the shared
letter-name table ("a" → "эй", "I" → "ай", "x" → "икс"),
case-insensitively; they SHALL NOT be recorded in the unknown-words map.
For remaining multi-letter words the resolution order SHALL be: `IT_TERMS`
dictionary ("api" → "эй пи ай", "github" → "гитхаб"); all-uppercase words of
length ≥ 2 via `AbbreviationNormalizer` (special cases like "ios" →
"ай оу эс", `AS_WORD` entries like "json" → "джейсон", otherwise
letter-by-letter via the same shared letter-name table); `AS_WORD`
dictionary for mixed-case entries; and finally digraph-first
transliteration (`sh` → "ш", `tion` → "шн", longest match first). Custom
terms registered via `EnglishNormalizer::add_custom_terms` SHALL override
`IT_TERMS`. Words resolved by transliteration SHALL be recorded in the
unknown-words map, which SHALL be cleared at the start of every
`process_with_char_mapping` call.

The letter-name table SHALL have a single home shared by the abbreviation
spelling, code-identifier spelling, and lone-letter reading paths; its
canonical readings for x/y/z SHALL be "икс", "вай", "зет" — so an unknown
abbreviation and a lone letter sound the same ("x" and "X" both → "икс").

#### Scenario: Uppercase abbreviation spelled out

- GIVEN the input "через API"
- WHEN the pipeline processes it
- THEN the abbreviation is read letter by letter as "эй пи ай" and not
  transliterated as a word

#### Scenario: Unknown abbreviation with x/y/z

- GIVEN the input "формат XYZ"
- WHEN the pipeline processes it
- THEN the abbreviation is read "икс вай зет" — the same letter names as
  for lone letters

#### Scenario: IT term from dictionary

- GIVEN the input "на github"
- WHEN the pipeline processes it
- THEN the word is read as "гитхаб"

#### Scenario: Unknown word transliterated

- GIVEN an English word absent from all dictionaries, e.g. "workflow"
- WHEN the pipeline processes it
- THEN the word is transliterated to Cyrillic via the digraph rules and
  recorded in the unknown-words map

#### Scenario: Single Latin letter read by letter name

- GIVEN the input "Переменная x равна 5"
- WHEN the pipeline processes it
- THEN the letter is read as "икс" ("Переменная икс равна пять") and no Latin
  characters remain in the output

#### Scenario: Single letters of any case

- GIVEN the input "пункты a и I"
- WHEN the pipeline processes it
- THEN the letters are read as "эй" and "ай" ("пункты эй и ай")
