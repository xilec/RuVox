# Delta: text-pipeline

## MODIFIED Requirements

### Requirement: Ranges and percentages

The system SHALL read numeric ranges `N-M` as "от <N> до <M>" with both
bounds in genitive case; bounds in the year band 1000–9999 SHALL use ordinal
genitive forms ("от двухтысячного до две тысячи двадцать четвёртого"). The
system SHALL read percentages `N%` as number words plus the correctly
declined "процент/процента/процентов"; decimal percentages SHALL be read
with "точка" between the integer and fractional digits. A percentage range
`N-M%` SHALL be read as a single unit — "от <N> до <M> процентов" with both
bounds in genitive case and the fixed genitive-plural "процентов" — and the
percentage-range phase MUST run before the plain percentage phase so the
trailing "%" is not consumed separately.

#### Scenario: Simple range

- GIVEN the input "10-20"
- WHEN the pipeline processes it
- THEN the range is read as "от десяти до двадцати"

#### Scenario: Percentage range

- GIVEN the input "Рост на 10-20% за квартал"
- WHEN the pipeline processes it
- THEN the range is read as "от десяти до двадцати процентов" and no bare
  "-" or "%" remains in the output

#### Scenario: Decimal percentage

- GIVEN the input "99.9%"
- WHEN the pipeline processes it
- THEN the percentage is read with "точка" for the fractional part and the
  word "процентов"

### Requirement: Fixed phase order

The system SHALL execute normalization phases in a strictly fixed order
(`src-tauri/src/pipeline/mod.rs::process_with_char_mapping`): BOM removal;
fenced code blocks; quote normalization; dash normalization; whitespace
normalization; inline code; Markdown structure; URLs/emails/IPs/paths; sizes;
dates and times; percentage ranges; percentages; ranges; versions; leading-dot decimals;
operators; special symbols; code identifiers; English words; numbers;
whitespace post-processing. The
order is load-bearing: URLs MUST be consumed before the number phase so an IP
is not torn into four numbers, percentage ranges MUST be consumed before
percentages so the trailing "%" is not read separately from its range,
percentages MUST be consumed before versions
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
