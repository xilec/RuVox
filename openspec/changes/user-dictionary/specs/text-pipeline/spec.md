## MODIFIED Requirements

### Requirement: Fixed phase order

The system SHALL execute normalization phases in a strictly fixed order
(`src-tauri/src/pipeline/mod.rs::process_with_char_mapping`): BOM removal;
fenced code blocks; quote normalization; dash normalization; whitespace
normalization; inline code; Markdown structure; URLs/emails/IPs/paths; sizes;
dates and times; percentage ranges; percentages; ranges; versions; leading-dot decimals;
operators; special symbols; code identifiers; user-dictionary pre-pass;
English words; numbers; whitespace post-processing. The
order is load-bearing: URLs MUST be consumed before the number phase so an IP
is not torn into four numbers, percentage ranges MUST be consumed before
percentages so the trailing "%" is not read separately from its range,
percentages MUST be consumed before versions
so a decimal percentage is not torn apart by the version phase, versions MUST
precede bare numbers, leading-dot decimals MUST follow versions so "1.5"
keeps its version reading and MUST precede bare numbers so the fraction
digits are consumed before the number phase runs,
code identifiers MUST be split before English words, the user-dictionary
pre-pass MUST run after code-identifier splitting (so dictionary entries
apply to split parts of identifiers, never to whole identifiers) and before
English-word resolution (so user entries win over every built-in table),
abbreviations MUST be resolved before transliteration, and multi-character
operators MUST be processed longest-first.

#### Scenario: IP address is not split into numbers

- GIVEN the input "Пинг 192.168.1.1"
- WHEN the pipeline processes it
- THEN the address is read as "сто девяносто два точка сто шестьдесят восемь
  точка один точка один" and not as four unrelated numbers

#### Scenario: Version is read as a whole

- GIVEN the input "версия v2.3.1"
- WHEN the pipeline processes it
- THEN the version is read as "два точка три точка один" and the version
  phase consumes the region before the number phase runs

#### Scenario: Decimal percentage is not consumed by the version phase

- GIVEN the input "Рост на 12.5% за квартал."
- WHEN the pipeline processes it
- THEN the percentage is read as "двенадцать точка пять процентов" and no
  bare "%" remains in the output

#### Scenario: camelCase split before English processing

- GIVEN the input "getUserData"
- WHEN the pipeline processes it
- THEN the identifier is split by the code-identifier phase into "гет юзер
  дата" instead of being transliterated as one opaque word

#### Scenario: Empty user dictionary leaves output unchanged

- GIVEN an empty user dictionary and any input from the existing golden
  fixtures
- WHEN the pipeline processes it
- THEN the output is byte-identical to the output without the dictionary
  feature

## ADDED Requirements

### Requirement: User dictionary application in normalization

User dictionary entries SHALL win over every built-in table at every lookup
site: the prose pre-pass (before English-word resolution), code-identifier
parts (before `CODE_WORDS`), URL word reading (before `IT_TERMS`), and code
blocks read aloud (through the identifier path). The pre-pass SHALL match
tokens of Latin letters and digits containing at least one letter —
including alnum tokens no other phase captures, such as "IPv6" or "mp3" —
and replace only exact case-insensitive key hits; non-hits SHALL flow into
the English-word phase unchanged. Lookups in identifiers and URLs SHALL use
the split parts/words those phases already produce.

#### Scenario: Entry overrides an IT term in prose

- GIVEN the entry `docker → докер` ("docker" normally transliterates)
- WHEN the input "запусти docker" is processed
- THEN the output reads "докер"

#### Scenario: Entry overrides letter spelling of an abbreviation

- GIVEN the entry `SQL → эс ку эль`
- WHEN the input "запрос к SQL" is processed
- THEN the output reads "эс ку эль" instead of the built-in letter-by-letter
  reading

#### Scenario: Alnum token is normalized via the dictionary

- GIVEN the entry `IPv6 → айпи ви шесть` and the input "сеть IPv6 работает"
- WHEN the pipeline processes it
- THEN the output reads "айпи ви шесть" — before this feature no phase
  captured alnum tokens and raw Latin reached the TTS engine

#### Scenario: Entry applies to a part of a code identifier

- GIVEN the entry `kubectl → куб контрол`
- WHEN the input "команда kubectl_apply" is processed
- THEN the identifier is split and the "kubectl" part reads "куб контрол"

#### Scenario: Entry applies inside a URL

- GIVEN the entry `github → хаб`
- WHEN the input "см. https://github.com/ruvox" is processed
- THEN the host component reads "хаб" instead of the built-in "гитхаб"

#### Scenario: Entry applies to code read aloud

- GIVEN the entry `user → юзер` and code block narration mode "full"
- WHEN a fenced code block containing `user_id = 1` is processed
- THEN the "user" part reads "юзер"

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
transliteration (`sh` → "ш", `tion` → "шн", longest match first). Words
resolved by transliteration SHALL be recorded in the unknown-words map,
which SHALL be cleared at the start of every `process_with_char_mapping`
call. The former `EnglishNormalizer::add_custom_terms` hook is removed —
user overrides are owned by the user-dictionary pre-pass, which runs before
this phase (see "User dictionary application in normalization").

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
