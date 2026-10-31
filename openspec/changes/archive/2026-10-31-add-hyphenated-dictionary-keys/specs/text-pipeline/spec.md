## MODIFIED Requirements

### Requirement: Fixed phase order

The system SHALL execute normalization phases in a strictly fixed order
(`src-tauri/src/pipeline/mod.rs`): BOM removal; fenced code blocks; quote
normalization; dash normalization; whitespace normalization; inline code;
Markdown structure; URLs/emails/IPs/paths; sizes; dates and times; percentage
ranges; percentages; ranges; versions; leading-dot decimals; operators; special
symbols; hyphenated dictionary keys; code identifiers; user-dictionary
pre-pass; English words; numbers; whitespace post-processing; span resolution.
The order is load-bearing: URLs
MUST be consumed before the number phase so an IP is not torn into four
numbers, percentage ranges MUST be consumed before percentages so the trailing
"%" is not read separately from its range, percentages MUST be consumed before
versions so a decimal percentage is not torn apart by the version phase,
versions MUST precede bare numbers, leading-dot decimals MUST follow versions so
"1.5" keeps its version reading and MUST precede bare numbers so the fraction
digits are consumed before the number phase runs, hyphenated dictionary keys
MUST be resolved on the whole token before code-identifier splitting (so a
whole-token entry wins over its own parts and over built-in tables, on both the
narrating and the native identifier route), code identifiers MUST be split
before English words, the user-dictionary pre-pass MUST run after code-identifier
splitting (so single-token dictionary entries apply to split parts of
identifiers, never to whole identifiers) and before English-word resolution (so
user entries win over
every built-in table), abbreviations MUST be resolved before transliteration,
and multi-character operators MUST be processed longest-first.

The hyphenated-key phase SHALL be skipped when the dictionary has no key with a
hyphen, so a dictionary of plain words leaves the output byte-identical and
costs no extra scan.

The span resolution phase SHALL run last, after the final trim, so that every
span range refers to the text the engine will receive. A gated phase that the
profile declines SHALL be skipped without changing the order of the phases that
do run, and skipping it SHALL NOT let a later phase consume its fragment.

#### Scenario: IP address is not split into numbers

- GIVEN the input "Пинг 192.168.1.1"
- WHEN the pipeline processes it with the reference profile
- THEN the address is read as "сто девяносто два точка сто шестьдесят восемь
  точка один точка один" and not as four unrelated numbers

#### Scenario: Version is read as a whole

- GIVEN the input "версия v2.3.1"
- WHEN the pipeline processes it with the reference profile
- THEN the version is read as "два точка три точка один" and the version
  phase consumes the region before the number phase runs

#### Scenario: Decimal percentage is not consumed by the version phase

- GIVEN the input "Рост на 12.5% за квартал."
- WHEN the pipeline processes it with the reference profile
- THEN the percentage is read as "двенадцать точка пять процентов" and no
  bare "%" remains in the output

#### Scenario: camelCase split before English processing

- GIVEN the input "getUserData"
- WHEN the pipeline processes it with the reference profile
- THEN the identifier is split by the code-identifier phase into "гет юзер
  дата" instead of being transliterated as one opaque word

#### Scenario: Hyphenated dictionary entry wins before the identifier split

- GIVEN the entry `e-mail → имейл` and the input "напиши e-mail"
- WHEN the pipeline processes it with the reference profile
- THEN the output reads "напиши имейл" and the token is never split into
  identifier parts that would read "и мэйл"

#### Scenario: Empty user dictionary leaves output unchanged

- GIVEN an empty user dictionary and any input from the existing golden
  fixtures
- WHEN the pipeline processes it
- THEN the output is byte-identical to the output without the dictionary
  feature

#### Scenario: Span resolution runs after the final trim

- GIVEN an input whose normalized text has leading and trailing whitespace
- WHEN the pipeline processes it
- THEN every span range lies within the trimmed text and ends before its
  codepoint count
