# Delta: text-pipeline

## ADDED Requirements

### Requirement: Leading-dot decimals

The system SHALL read a bare decimal fraction written without the integer
part — a dot directly followed by digits, as in "Вес .5 кг" — as a proper
decimal: "ноль" + "точка" + the fractional digits read individually
(".5" → "ноль точка пять"), identical to how "0.5" is read. The dot MUST
NOT be preceded by a letter, digit, underscore, dot, or path separator:
those forms are float tails, dotted labels, version chains, and path
fragments owned by earlier phases and SHALL be left untouched. The
leading-dot decimal phase MUST run after versions (so "1.5" keeps its
version-path reading) and before operators and bare numbers.

#### Scenario: Leading-dot decimal after a space

- GIVEN the input "Вес .5 кг"
- WHEN the pipeline processes it
- THEN the output is "Вес ноль точка пять кг" and no literal digit remains

#### Scenario: Leading-dot decimal at text start

- GIVEN the input ".75 вероятность"
- WHEN the pipeline processes it
- THEN the output starts with "ноль точка семь пять"

#### Scenario: Dot preceded by a letter is untouched

- GIVEN the input "файл example.5"
- WHEN the pipeline processes it
- THEN the ".5" fragment is not read as a decimal (it belongs to a dotted
  label), and the digit is not expanded by the number phase either

## MODIFIED Requirements

### Requirement: Fixed phase order

The system SHALL execute normalization phases in a strictly fixed order
(`src-tauri/src/pipeline/mod.rs::process_with_char_mapping`): BOM removal;
fenced code blocks; quote normalization; dash normalization; whitespace
normalization; inline code; Markdown structure; URLs/emails/IPs/paths; sizes;
dates and times; percentages; ranges; versions; leading-dot decimals;
operators; special symbols; code identifiers; English words; numbers;
whitespace post-processing. The
order is load-bearing: URLs MUST be consumed before the number phase so an IP
is not torn into four numbers, percentages MUST be consumed before versions
so a decimal percentage is not torn apart by the version phase, versions MUST
precede bare numbers, leading-dot decimals MUST follow versions so "1.5"
keeps its version reading and MUST precede bare numbers so the fraction
digits are consumed before the number phase runs,
code identifiers MUST be split before English words,
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
