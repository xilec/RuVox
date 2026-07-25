# Delta: text-pipeline

## MODIFIED Requirements

### Requirement: Fixed phase order

The system SHALL execute normalization phases in a strictly fixed order
(`src-tauri/src/pipeline/mod.rs::process_with_char_mapping`): BOM removal;
fenced code blocks; quote normalization; dash normalization; whitespace
normalization; inline code; Markdown structure; URLs/emails/IPs/paths; sizes;
dates and times; percentages; ranges; versions; operators; special symbols;
code identifiers; English words; numbers; whitespace post-processing. The
order is load-bearing: URLs MUST be consumed before the number phase so an IP
is not torn into four numbers, percentages MUST be consumed before versions
so a decimal percentage is not torn apart by the version phase, versions MUST
precede bare numbers, code identifiers MUST be split before English words,
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

### Requirement: Numbers

The system SHALL read standalone integers as Russian cardinal number words
("123" → "сто двадцать три"), including thousands, millions, and billions
with correct declension and gender agreement ("тысяча" feminine). The number
phase SHALL skip digits that are adjacent to a dot, another digit, or a
Latin/Cyrillic letter, because those regions are owned by the earlier URL,
size, date, version, range, and code-identifier phases. Non-integer input and
integers that fail to parse SHALL be left unchanged. Number replacements
SHALL be applied positionally by byte range, so a match that is a substring
of another match (e.g. "1" inside "10") MUST NOT corrupt the longer number.

#### Scenario: Plain number

- GIVEN the input "Версия 3"
- WHEN the pipeline processes it
- THEN the number is read as "три"

#### Scenario: Number adjacent to a letter is skipped

- GIVEN a token like "v1" left over from earlier phases
- WHEN the number phase runs
- THEN the digit next to the letter is not expanded by the number phase

#### Scenario: Ratio with a multi-digit number

- GIVEN the input "Счёт 10:1 в нашу пользу."
- WHEN the pipeline processes it
- THEN the output is "Счёт десять:один в нашу пользу." with both numbers
  read as whole number words

#### Scenario: Repeated numbers replaced independently

- GIVEN the input "33 3"
- WHEN the pipeline processes it
- THEN the output reads both numbers correctly as "тридцать три три"
