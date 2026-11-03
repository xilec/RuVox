## MODIFIED Requirements

### Requirement: Fixed phase order

The system SHALL execute normalization phases in a strictly fixed order
(`tts-normalize/src/pipeline/mod.rs`): BOM removal; fenced code blocks; quote
normalization; whitespace normalization; inline code; Markdown structure;
URLs/emails/IPs/paths; numeric spans; dash normalization; operators;
alphanumeric abbreviations; special symbols; hyphenated dictionary keys; code
identifiers; user-dictionary pre-pass; English words; whitespace
post-processing; span resolution.

The numeric span phase SHALL own every numeric shape — sizes and durations,
dates and times, percentage ranges, percentages, ranges, versions,
leading-dot decimals, bare numbers, years, ordinals, money, phones and
reference numbers — in one scan. The size, date/time, percentage-range,
percentage, range, version, leading-dot-decimal and bare-number phases SHALL
NOT exist separately, so two numeric phases can never race for the same
digits.

The order is load-bearing: URLs, emails, IPs and file paths MUST be consumed
before the numeric span phase, because an address is one token whose reading
belongs to the URL/path normalizer and a dotted address ("192.168.0.1") must
not be scanned as a version number. Dash normalization MUST run after the
numeric span phase: a range keeps the dash the source used ("5–10"), and
replacing the dash earlier would make the numeric span overlap an
already-replaced range, which the tracked text drops as double processing.
Hyphenated dictionary keys MUST be resolved on the whole token before
code-identifier splitting (so a whole-token entry wins over its own parts and
over built-in tables, on both the narrating and the native identifier route),
code identifiers MUST be split before English words, the user-dictionary
pre-pass MUST run after code-identifier splitting (so single-token entries
apply to split parts of identifiers, never to whole identifiers) and before
English-word resolution (so user entries win over every built-in table),
abbreviations MUST be resolved before transliteration, and multi-character
operators MUST be processed longest-first.

The hyphenated-key phase SHALL be skipped when the dictionary has no key with a
hyphen, so a dictionary of plain words leaves the output byte-identical and
costs no extra scan.

The numeric span phase SHALL run only when the profile expands digits. With
`units: Native` it SHALL NOT claim a construct that carries a unit,
percentage, currency or scale word, so the digits stay with the symbol. The
span resolution phase SHALL run last, after the final trim, so that every span
range refers to the text the engine will receive. A gated phase that the
profile declines SHALL be skipped without changing the order of the phases
that do run, and skipping it SHALL NOT let a later phase consume its fragment.

#### Scenario: IP address is not split into numbers

- GIVEN the input "Пинг 192.168.1.1"
- WHEN the pipeline processes it with the reference profile
- THEN the address is claimed by the URL/email/IP/path phase before the
  numeric span phase runs, and is read as "сто девяносто два точка сто
  шестьдесят восемь точка один точка один" instead of four unrelated numbers

#### Scenario: Version is read as a whole

- GIVEN the input "версия v2.3.1"
- WHEN the pipeline processes it with the reference profile
- THEN the numeric span phase reads the whole version as "два точка три точка
  один" and no digit of it is read separately

#### Scenario: Decimal percentage is not consumed by the version phase

- GIVEN the input "Рост на 12.5% за квартал."
- WHEN the pipeline processes it with the reference profile
- THEN the percentage is read as "двенадцать точка пять процентов" and no
  bare "%" remains in the output

#### Scenario: Percentage range is read as a whole

- GIVEN the input "Рост на 10-20% за квартал"
- WHEN the pipeline processes it with the reference profile
- THEN the span is classified as a percentage range in the same scan and reads
  "Рост на от десяти до двадцати процентов за квартал", with neither a bare
  dash nor a bare "%" left in the output

#### Scenario: Dash normalization runs after the numeric span phase

- GIVEN the input "5–10 кг" with an en dash
- WHEN the pipeline processes it with the reference profile
- THEN the numeric span owns the source en dash and reads "от пяти до десяти
  килограммов", and no en dash remains in the output

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

### Requirement: Sizes, durations, and versions

The numeric span layer SHALL read size and duration values (`KB`, `MB`, `GB`,
`TB`, `ms`, `sec`, `min`, `hr`, `px`, `em`, `rem`, `vh`, `vw`, and their
Cyrillic counterparts кб/мб/гб/тб) as number words plus a correctly declined
unit ("мегабайт/мегабайта/мегабайт"), applying the feminine numeral forms
("одна", "две") for feminine units. A unit with a fixed reading ("em" → "эм",
"rem" → "рем", "vh"/"vw" → letter names) SHALL NOT decline. A decimal with a
size unit SHALL keep the technical "точка" register instead of a natural
fraction ("1.5em" → "один точка пять эм"). A construct carrying a size or
duration unit SHALL be claimed as one span only when the profile expands
units; with `units: Native` the whole construct stays with the symbol.

The layer SHALL read semantic versions (`v2.3.1`, `1.2.0-beta`) as number words
joined by "точка", with pre-release suffixes `alpha`, `beta`, `rc`, `dev`,
`stable`, `release` read via the suffix table ("альфа", "бета", "эр си", "дев",
"стейбл", "релиз"). Digits glued to the suffix word SHALL be read as a number
word right after it ("1.2.0-beta2" → "один точка два точка ноль бета два"),
and a numeric group after a dot that follows the suffix SHALL be read after
the suffix word too ("1.0.0-beta.2" → "один точка ноль точка ноль бета
два"). A letter-plus-digit token without a dot
("v2") is not a version; the alphanumeric-token phase reads the whole token
instead.

#### Scenario: Size with declension

- GIVEN the input "Файл весит 100MB"
- WHEN the pipeline processes it with the reference profile
- THEN the size is read as "сто мегабайт"

#### Scenario: Duration

- GIVEN the input "таймаут 50ms"
- WHEN the pipeline processes it with the reference profile
- THEN the duration is read as "пятьдесят миллисекунд"

#### Scenario: Pre-release version

- GIVEN the input "1.2.0-beta"
- WHEN the pipeline processes it with the reference profile
- THEN the version is read as "один точка два точка ноль бета"

#### Scenario: Version with a prefix

- GIVEN the input "v2.3.1"
- WHEN the pipeline processes it with the reference profile
- THEN the prefix letter is consumed with the version and the output reads
  "два точка три точка один"

#### Scenario: Fixed size unit keeps the technical decimal

- GIVEN the input "1.5em"
- WHEN the pipeline processes it with the reference profile
- THEN the decimal keeps "точка" and the fixed unit does not decline:
  "один точка пять эм"

#### Scenario: Unit-carrying construct stays with the engine

- GIVEN a profile with `digits: Expanded` and `units: Native`
- WHEN the pipeline processes "512 KB"
- THEN the whole construct stays as it is and no part of it is read

### Requirement: Dates and times

The numeric span layer SHALL read calendar values as "<день-ordinal> <месяц в
родительном падеже> <год-ordinal> года": ISO dates (`YYYY-MM-DD`), European
dotted dates (`DD.MM.YYYY`), dotted ISO (`YYYY.MM.DD`), slashed dates in both
orders, written dates ("31 декабря 2024"), and dates without a year ("5
марта"). The day SHALL be a neuter ordinal; the year SHALL be a masculine
genitive ordinal. The day SHALL read nominative when the date is a label, the
genitive after "от", "до", "с", "со", "из", "около" and "после", the
dative after "к" and "ко", the prepositional after "о", "об" and "при", and
the accusative after "по".
An optional "года"/"год"/"г." after a written date's year SHALL be
consumed with it. A two-digit year SHALL read as 20YY, and a day-first
European order SHALL be assumed when the date is ambiguous.

A four-digit year in 1900–2100 followed by a hyphen and a one- or
two-digit month
("2024-12") SHALL read as a year-month — "<month in the nominative> <year
genitive ordinal> года" — and MUST NOT be read as a numeric range.

A dotted date-like value with an out-of-range day, month or year SHALL NOT be
read as a calendar date; its dotted groups fall to the version reading
("32.13.2024" → "тридцать два точка тринадцать точка две тысячи двадцать
четыре").

The layer SHALL read clock times `HH:MM` and `HH:MM:SS` with two-digit minutes
as number words plus "час/часа/часов", "минута/минуты/минут" and
"секунда/секунды/секунд"; zero minutes and seconds SHALL be omitted ("12:00" →
"двенадцать часов"). A time that is one bound of a `с … до …` range SHALL take
the genitive, and a left bound with zero minutes and seconds SHALL drop the
hour noun ("с 10:00 до 18:00" → "с десяти до восемнадцати часов"). A time with
an out-of-range component SHALL NOT be a time and SHALL fall through to the
number reading ("25:00" → "двадцать пять:ноль"). Date and time values SHALL be
claimed by the numeric span phase in the same scan, so they cannot race the
other numeric shapes, and the layer SHALL leave every word around the span
untouched, including a capital letter.

#### Scenario: ISO date

- GIVEN the input "Релиз 2024-12-31"
- WHEN the pipeline processes it with the reference profile
- THEN the date is read as "тридцать первое декабря две тысячи двадцать
  четвёртого года"

#### Scenario: European dotted date

- GIVEN the input "Дата: 15.01.2024"
- WHEN the pipeline processes it with the reference profile
- THEN the date is read as "пятнадцатое января две тысячи двадцать четвёртого
  года"

#### Scenario: Date after a preposition takes the required case

- GIVEN the inputs "Отчёт от 15.01.2024" and "До 31.12.2024 осталось два дня"
- WHEN the pipeline processes them with the reference profile
- THEN the days read in the genitive: "Отчёт от пятнадцатого января две тысячи
  двадцать четвёртого года" and "До тридцать первого декабря две тысячи
  двадцать четвёртого года осталось два дня"

#### Scenario: Date pair after с … по … keeps the capital

- GIVEN the input "С 01.01.2024 по 31.12.2024"
- WHEN the pipeline processes it with the reference profile
- THEN the left date reads in the genitive and the right date in the
  accusative: "С первого января две тысячи двадцать четвёртого года по
  тридцать первое декабря две тысячи двадцать четвёртого года", and the
  capital "С" before the span is untouched

#### Scenario: Written date

- GIVEN the inputs "31 декабря 2024" and "5 марта"
- WHEN the pipeline processes them with the reference profile
- THEN they read as "тридцать первое декабря две тысячи двадцать четвёртого
  года" and "пятое марта"

#### Scenario: Two-digit year is the 2000s

- GIVEN the input "15.01.24"
- WHEN the pipeline processes it with the reference profile
- THEN the two-digit year reads as 2024 and the date reads "пятнадцатое января
  две тысячи двадцать четвёртого года"

#### Scenario: Year-month is not a range

- GIVEN the input "2024-12"
- WHEN the pipeline processes it with the reference profile
- THEN it reads as "декабрь две тысячи двадцать четвёртого года" and not as a
  range with two bounds

#### Scenario: Clock time omits zero minutes and seconds

- GIVEN the inputs "10:30" and "08:00:00"
- WHEN the pipeline processes them with the reference profile
- THEN they read as "десять часов тридцать минут" and "восемь часов"

#### Scenario: Time range drops the left hour noun

- GIVEN the input "с 10:00 до 18:00"
- WHEN the pipeline processes it with the reference profile
- THEN it reads as "с десяти до восемнадцати часов"

#### Scenario: Invalid time falls through to numbers

- GIVEN the input "В 25:00 встреча"
- WHEN the pipeline processes it with the reference profile
- THEN the time is not recognized and the digits are read by the number rules
  as "В двадцать пять:ноль встреча"

### Requirement: Ranges and percentages

The numeric span layer SHALL read numeric ranges `N–M` with a hyphen, an en
dash or an em dash, with or without surrounding spaces, as "от <N> до <M>"
with both bounds in the genitive. Bounds that are plain integers and both fall
in 1000–2199 SHALL use the ordinal genitive ("от двухтысячного до две
тысячи двадцать четвёртого"). A year range followed by "годы", "гг.", "год",
"годах" or "годов" SHALL read "с <ordinal genitive> по <ordinal accusative>
год" ("1812–1814 годы" → "с тысяча восемьсот двенадцатого по тысяча
восемьсот четырнадцатый год").
integers and whose right bound is the left bound plus one, with nothing attached
after it, SHALL keep
the hyphenated pair reading with both bounds as cardinals ("2-3 раза" →
"два-три раза"). A range may share one trailing unit, percentage or currency;
it SHALL be read in the plural genitive ("от пяти до десяти килограммов", "от
десяти до двадцати процентов"), and a fixed unit SHALL keep its own form. A
fractional bound SHALL read as a fractional numeral ("от полутора до двух").

The layer SHALL read percentages `N%` with or without a space as number words
plus the correctly declined "процент/процента/процентов". A decimal
percentage SHALL keep "точка" between the integer and fractional digits and
read "процентов". A percentage range `N–M%` SHALL read as one unit "от … до …
процентов" with both bounds in the genitive and the fixed genitive plural. A
number after a genitive trigger — "от", "до", "с", "со", "из", "около",
"более", "менее", "свыше", "после", "для", "без", "у", "порядка", "младше",
"старше" — SHALL read in the genitive, so "от 5 до 10", "с 5 до 10" and "более
5 %" read their bounds in the genitive.

The layer SHALL read approximation prefixes and suffixes together with the
number: "~" → "около" plus the genitive ("около пяти"), "≈" → "примерно"
("примерно пять"), "±" → "плюс-минус" ("плюс-минус пять"), and a trailing "+"
→ "и больше" ("пять и больше").

#### Scenario: Simple range

- GIVEN the input "10-20"
- WHEN the pipeline processes it with the reference profile
- THEN the range is read as "от десяти до двадцати"

#### Scenario: En-dash range with a unit

- GIVEN the input "5–10 кг"
- WHEN the pipeline processes it with the reference profile
- THEN the range reads "от пяти до десяти килограммов" and both bounds are in
  the genitive

#### Scenario: Percentage range

- GIVEN the input "Рост на 10-20% за квартал"
- WHEN the pipeline processes it with the reference profile
- THEN the range is read as "от десяти до двадцати процентов" and no bare
  "-" or "%" remains in the output

#### Scenario: Percentage agreement

- GIVEN the inputs "1 %", "2 %", "22 %" and "0 %"
- WHEN the pipeline processes them with the reference profile
- THEN the noun agrees with the count: "один процент", "два процента",
  "двадцать два процента" and "ноль процентов"

#### Scenario: Decimal percentage

- GIVEN the input "99.9%"
- WHEN the pipeline processes it with the reference profile
- THEN the percentage is read as "девяносто девять точка девять процентов"

#### Scenario: Year range reads ordinal bounds

- GIVEN the input "2020–2024"
- WHEN the pipeline processes it with the reference profile
- THEN both bounds are ordinals in the genitive: "от две тысячи двадцатого до
  две тысячи двадцать четвёртого"

#### Scenario: Year range followed by a year word reads с … по …

- GIVEN the input "2020–2024 годы"
- WHEN the pipeline processes it with the reference profile
- THEN it reads as "с две тысячи двадцатого по две тысячи двадцать четвёртый
  год" and no bare dash remains

#### Scenario: Adjacent counts keep the hyphen

- GIVEN the inputs "2-3 раза" and "5–6 страниц"
- WHEN the pipeline processes them with the reference profile
- THEN they read as hyphenated pairs: "два-три раза" and "пять-шесть страниц"

#### Scenario: Decimal range bound

- GIVEN the input "1,5–2 кг"
- WHEN the pipeline processes it with the reference profile
- THEN the range reads "от полутора до двух килограммов"

#### Scenario: Approximated number

- GIVEN the inputs "~5 минут", "≈ 5 минут", "±5" and "5+"
- WHEN the pipeline processes them with the reference profile
- THEN they read as "около пяти минут", "примерно пять минут", "плюс-минус
  пять" and "пять и больше"

#### Scenario: Genitive trigger makes the bound genitive

- GIVEN the inputs "от 5 до 10", "с 5 до 10" and "более 5 %"
- WHEN the pipeline processes them with the reference profile
- THEN they read as "от пяти до десяти", "с пяти до десяти" and "более пяти
  процентов"

### Requirement: Numbers

The numeric span layer SHALL read standalone integers as Russian cardinal
number words with gender agreement from the head noun that follows ("42 книги"
→ "сорок две книги", "1 рубль" → "один рубль") and with the genitive case
after a genitive trigger ("от 5" → "от пяти"). A lone scale word SHALL drop
the leading "одна"/"один": "1000" → "тысяча", "1 500" → "тысяча пятьсот",
"1 000 000" → "миллион", while "2 000" → "две тысячи" keeps the numeral. A
space- or NBSP-separated group of exactly three digits SHALL be part of the
same number, not a second number ("10 000" → "десять тысяч", "1 234,56" → "одна
тысяча двести тридцать четыре целых пятьдесят шесть сотых").

A decimal fraction with a comma and a physical unit SHALL read as a natural
quantity ("1,5 кг" → "полтора килограмма", "2,5 км" → "два с половиной
километра", "0,25 кг" → "ноль целых двадцать пять сотых килограмма").
The denominator SHALL agree with the numerator ("0,1" → "ноль целых одна
десятая", "2,25 кг" → "две целых двадцать пять сотых килограмма"). A bare
decimal written with a dot SHALL keep the technical register "точка" ("3.14" →
"три точка четырнадцать", "0.5" → "ноль точка пять"). A dot decimal with a
size unit SHALL keep the technical register and read the unit ("1.5em" → "один
точка пять эм"). A slash fraction SHALL read as a simple fraction ("1/2" →
"одна вторая", "2/3" → "две третьих"), and a Unicode mixed fraction SHALL
read naturally ("3½" → "три с половиной"). A leading minus or plus sign SHALL
read "минус"/"плюс" ("-5" → "минус пять", "−5" → "минус пять", "+5" → "плюс
пять"), with U+2212 treated as a minus.

A zero-padded token of three or more digits SHALL read digit by digit ("007" →
"ноль ноль семь") and stay a code, while an ordinary integer such as "404"
reads as a cardinal ("четыреста четыре"). A value above 999 999 999 999
SHALL read digit by digit instead of dropping high digits
("1000000000000" → "один ноль ноль ноль ноль ноль ноль ноль ноль ноль
ноль ноль ноль"). Digits adjacent to a letter, an
underscore, a slash or a dotted label SHALL not be claimed by the layer; those
fragments belong to the alphanumeric-token or address phases. A number
immediately before a terminal dot SHALL be read normally. Readings SHALL be
applied positionally over one snapshot so a span cannot corrupt another span
or a longer number.

#### Scenario: Plain number

- GIVEN the input "Версия 3"
- WHEN the pipeline processes it with the reference profile
- THEN the number is read as "три"

#### Scenario: Number adjacent to a letter is skipped

- GIVEN a token like "v1" left over from earlier phases
- WHEN the pipeline processes it with the reference profile
- THEN the numeric span layer does not claim the digit next to the letter; the
  alphanumeric-token phase reads the whole token ("ви один")

#### Scenario: Ratio with a multi-digit number

- GIVEN the input "Счёт 10:1 в нашу пользу."
- WHEN the pipeline processes it with the reference profile
- THEN the output is "Счёт десять:один в нашу пользу." with both numbers read
  as whole number words

#### Scenario: Dot between digits remains a separator

- GIVEN a fragment like "3.14" that no earlier address or version phase consumed
- WHEN the pipeline processes it with the reference profile
- THEN the dot separates the digits inside one number and the fragment reads as
  one decimal, "три точка четырнадцать", never as two unrelated numbers

#### Scenario: Cardinal agrees with the head noun

- GIVEN the inputs "42 книги", "1 рубль" and "2 человека"
- WHEN the pipeline processes them with the reference profile
- THEN the numerals agree with the head nouns: "сорок две книги", "один
  рубль" and "два человека"

#### Scenario: Grouped digits are one number

- GIVEN the inputs "10 000 рублей" and "1 234,56"
- WHEN the pipeline processes them with the reference profile
- THEN each group belongs to its number: "десять тысяч рублей" and "одна
  тысяча двести тридцать четыре целых пятьдесят шесть сотых"

#### Scenario: A lone scale word drops the leading one

- GIVEN the inputs "1000", "1 500" and "1 000 000"
- WHEN the pipeline processes them with the reference profile
- THEN the scale word reads without the numeral: "тысяча", "тысяча пятьсот"
  and "миллион"

#### Scenario: Decimal with a physical unit reads naturally

- GIVEN the inputs "1,5 кг", "2,5 км" and "0,25 кг"
- WHEN the pipeline processes them with the reference profile
- THEN the fractions close into natural quantities: "полтора килограмма",
  "два с половиной километра" and "ноль целых двадцать пять сотых
  килограмма"

#### Scenario: Bare decimal keeps the technical register

- GIVEN the inputs "3.14" and "0.5"
- WHEN the pipeline processes them with the reference profile
- THEN they read with "точка": "три точка четырнадцать" and "ноль точка пять"

#### Scenario: Fractions

- GIVEN the inputs "1/2", "2/3" and "3½"
- WHEN the pipeline processes them with the reference profile
- THEN they read as "одна вторая", "две третьих" and "три с половиной"

#### Scenario: Signed numbers

- GIVEN the inputs "-5", "−5" and "+5"
- WHEN the pipeline processes them with the reference profile
- THEN they read as "минус пять", "минус пять" and "плюс пять"

#### Scenario: Zero-padded token is a code

- GIVEN the inputs "агент 007" and "ответ 404"
- WHEN the pipeline processes them with the reference profile
- THEN the code reads digit by digit and the ordinary integer as a cardinal:
  "агент ноль ноль семь" and "ответ четыреста четыре"

#### Scenario: Ratio with a colon

- GIVEN the input "Счёт 10:1 в нашу пользу."
- WHEN the pipeline processes it with the reference profile
- THEN the colon does not make a time and both sides read as whole number
  words: "Счёт десять:один в нашу пользу."

#### Scenario: Repeated numbers replaced independently

- GIVEN the input "42 42"
- WHEN the pipeline processes it with the reference profile
- THEN both numbers read as "сорок два сорок два" and neither replacement
  corrupts the other

#### Scenario: Number before a sentence-ending dot

- GIVEN the input "Встреча в 5."
- WHEN the pipeline processes it with the reference profile
- THEN the number reads as "пять" and the output is "Встреча в пять."

### Requirement: Leading-dot decimals

The numeric span layer SHALL read a bare decimal fraction written without the
integer part — a dot directly followed by digits, as in "Вес .5 кг" — as a
proper decimal: "ноль" + "точка" + the fractional digits read individually
(".5" → "ноль точка пять"), identical to how "0.5" is read. The dot MUST NOT
be preceded by a letter, digit, underscore, dot, or path separator: those
forms are float tails, dotted labels, version chains, and path fragments owned
by other phases and SHALL be left untouched. The leading-dot shape SHALL be
classified inside the numeric span layer; there SHALL NOT be a separate
leading-dot phase, and a leading-dot decimal SHALL NOT claim a physical unit,
so "Вес .5 кг" keeps "кг" as written.

#### Scenario: Leading-dot decimal after a space

- GIVEN the input "Вес .5 кг"
- WHEN the pipeline processes it with the reference profile
- THEN the output is "Вес ноль точка пять кг" and no literal digit remains

#### Scenario: Leading-dot decimal at text start

- GIVEN the input ".75 вероятность"
- WHEN the pipeline processes it with the reference profile
- THEN the output starts with "ноль точка семь пять"

#### Scenario: Dot preceded by a letter is untouched

- GIVEN the input "файл example.5"
- WHEN the pipeline processes it with the reference profile
- THEN the ".5" fragment is not read as a decimal (it belongs to a dotted
  label), and the digit is not expanded by the number rules either

## ADDED Requirements

### Requirement: Years, ordinals, and decades

A four-digit value in the year band 1000–2199 SHALL read as a masculine
ordinal when the text makes it a year: a following year word ("год", "года",
"году", "годом", "годе", "годы", "годов", "годах"), a preceding month
name in the nominative or
a season/period word ("лето", "весна", "осень", "зима", "начало", "конец",
"середина", "весной"), or a `с/со/от … по/до` frame. The year word SHALL
select the case: "год" and "годы" nominative, "года" and "годов" genitive,
"году" dative after "к"/"ко"/"по" and prepositional otherwise, "годом"
instrumental, "годе" and "годах" prepositional. A year that is not made a year
by context SHALL stay a cardinal number. A year followed by the abbreviation
"г." SHALL read as a genitive ordinal plus "года". Years 1900–1999 SHALL drop
the leading "одна" and keep "тысяча" ("1900" → "тысяча девятисотый", "1999" →
"тысяча девятьсот девяносто девятый"); "2000" SHALL read as one word
("двухтысячный").

An ordinal suffix — a hyphen plus a case ending from "-й", "-я", "-е", "-ю",
"-го", "-му", "-м", "-ми", "-х", "-ых", "-ые", "-ым", "-ом", "-ем", "-им",
"-ой", "-ый", "-ое", "-ая", "-ую", "-его", "-ому", "-ыми" — SHALL select
the gender, number and case of the ordinal ("5-й этаж" → "пятый этаж", "2-я
попытка" → "вторая попытка", "21-го числа" → "двадцать первого числа", "к
1-му мая" → "к первому мая", "перед 3-м этажом" → "перед третьим этажом", "в
3-ю очередь" → "в третью очередь", "1-ми" → "первыми"). A bare "-й" SHALL be
resolved from the head noun: the feminine form with the preposition's case
before a feminine head ("из 5-й главы" → "из пятой главы"), otherwise the
masculine nominative ("5-й этаж" → "пятый этаж"). A bare "-х" SHALL read the
cardinal genitive by default ("из 2-х частей" → "из двух частей", "в 2-х
шагах" → "в двух шагах") and the ordinal plural only before a class-like head
("в 5-х классах" → "в пятых классах"). A plain number after a rank or class
head noun ("глава", "часть", "том", "раздел", "квартал", "этаж") SHALL read as
an ordinal and agree with the head's form ("глава 2" → "глава вторая", "том 2"
→ "том второй", "часть 3" → "часть третья").

Round years ending in zero in 1900–2099, and round two-digit values 20–90,
SHALL read as decade words when the suffix is "-х"/"-ых" or "-е"/"-ые" ("в
1990-х" → "в девяностых", "1990-е годы" → "девяностые годы", "в 2000-х" → "в
двухтысячных", "в 1910-х годах" → "в десятых годах", "в 2010-х годах" →
"в две тысячи десятых годах", "в 90-е" → "в девяностые"). A two-digit decade
SHALL read as a decade only when the next word is a year word or absent ("в
30-х годах" → "в
тридцатых годах"); otherwise the suffix SHALL read as an ordinal ("20-е
место" → "двадцатое место"). An uppercase Roman numeral from II to XXXIX
before "век", "веке", "века" or "веков" SHALL read as an ordinal in that
word's case ("в XX веке" → "в двадцатом веке"), and the Arabic suffix form
SHALL agree the same way ("в 20-м веке" → "в двадцатом веке").

#### Scenario: Year word selects the case

- GIVEN the inputs "в 2024 году", "с 1999 года" and "к 2030 году"
- WHEN the pipeline processes them with the reference profile
- THEN the years read as ordinals in the required case: "в две тысячи двадцать
  четвёртом году", "с тысяча девятьсот девяносто девятого года" and "к две
  тысячи тридцатому году"

#### Scenario: Year after a month or season

- GIVEN the inputs "лето 2024" and "в начале 2024 года"
- WHEN the pipeline processes them with the reference profile
- THEN the year reads genitive plus "года": "лето две тысячи двадцать
  четвёртого года" and "в начале две тысячи двадцать четвёртого года"

#### Scenario: Year abbreviation

- GIVEN the input "Выпуск 2024 г."
- WHEN the pipeline processes it with the reference profile
- THEN the year reads "две тысячи двадцать четвёртого года"

#### Scenario: 1900 and 2000 do not carry одна

- GIVEN the inputs "в 1900 году" and "2000 год"
- WHEN the pipeline processes them with the reference profile
- THEN they read as "в тысяча девятисотом году" and "двухтысячный год"

#### Scenario: Ordinal suffix selects gender, number and case

- GIVEN the inputs "5-й этаж", "2-я попытка", "1-е место", "21-го числа", "к
  1-му мая", "перед 3-м этажом", "в 3-ю очередь" and "1-ми"
- WHEN the pipeline processes them with the reference profile
- THEN the suffixes select the forms: "пятый этаж", "вторая попытка", "первое
  место", "двадцать первого числа", "к первому мая", "перед третьим этажом",
  "в третью очередь" and "первыми"

#### Scenario: Bare -й resolves by the head noun

- GIVEN the inputs "5-й этаж", "из 5-й главы" and "в 5-й главе"
- WHEN the pipeline processes them with the reference profile
- THEN the first is masculine nominative and the other two are feminine in the
  preposition's case: "пятый этаж", "из пятой главы" and "в пятой главе"

#### Scenario: Bare -х resolves by the head noun

- GIVEN the inputs "из 2-х частей", "в 2-х шагах" and "в 5-х классах"
- WHEN the pipeline processes them with the reference profile
- THEN the default is the cardinal genitive and a class-like head takes the
  ordinal plural: "из двух частей", "в двух шагах" and "в пятых классах"

#### Scenario: Rank head noun reads a plain number as an ordinal

- GIVEN the inputs "глава 2", "том 2" and "часть 3"
- WHEN the pipeline processes them with the reference profile
- THEN the number reads as an ordinal agreeing with the head: "глава вторая",
  "том второй" and "часть третья"

#### Scenario: Decades

- GIVEN the inputs "в 1990-х годах", "1990-е годы", "в 90-е" and "в 30-х
  годах"
- WHEN the pipeline processes them with the reference profile
- THEN they read as "в девяностых годах", "девяностые годы", "в девяностые"
  and "в тридцатых годах"

#### Scenario: Non-year head keeps the ordinal

- GIVEN the input "20-е место"
- WHEN the pipeline processes it with the reference profile
- THEN it reads as the ordinal "двадцатое место", not as a decade

#### Scenario: Centuries

- GIVEN the inputs "в XX веке" and "в 20-м веке"
- WHEN the pipeline processes them with the reference profile
- THEN both read as "в двадцатом веке"

### Requirement: Money, units, and identifiers

The numeric span layer SHALL own a prefix or trailing unit, currency, scale
word or reference prefix together with its number, so in the nominative context
the noun agrees with the count (singular nominative, singular genitive, plural
genitive). A currency
symbol (`₽`, `$`, `€`, `£`) SHALL read as the currency word before or after the
number, with spaces allowed ("$5" and "5 $" both → "пять долларов", "€ 20" →
"двадцать евро"), and a prefix form SHALL agree with the count ("$1" → "один
доллар"), and count agreement SHALL hold in every case the context selects
("$2" → "два доллара", "$21" → "двадцать один доллар", "1,5" → "полтора
доллара", "около $1" → "около одного доллара"). Money words (руб., коп.,
долл., евро) SHALL read the same way. A
scale word (тыс., млн, млрд) SHALL read as the scale noun declined by count,
with an optional currency after it ("5 тыс. ₽" → "пять тысяч рублей", "1,5 млн
$" → "полтора миллиона долларов"). A scale word in the genitive SHALL keep
its case form ("около 1 млн рублей" → "около миллиона рублей").

Physical units (кг, км, см, мм, м, л, мл, г), degrees Celsius with or without
the degree sign, and speed units (км/ч, м/с) SHALL read with the unit's
declined forms and its spoken tail ("20 °C" → "двадцать градусов Цельсия",
"100 км/ч" → "сто километров в час", "9 м/с" → "девять метров в секунду").
Physical-unit constructs SHALL be claimed only when the profile expands units;
with `units: Native` the number stays with its symbol. The same agreement
SHALL hold in the genitive context ("около 21 кг" → "около двадцати одного
килограмма", "более 1 кг" → "более одного килограмма").

A reference prefix SHALL read together with its number: "№" → "номер" (a glued
number is allowed), "стр." → "страница", "табл." → "таблица", "рис." →
"рисунок", "гл." → "глава" with a feminine ordinal ("гл. 2" → "глава вторая"),
"тел." → "телефон" followed by the phone groups, and "ИНН"/"арт." → the number
digit by digit ("ИНН 7707083893" → "ИНН семь семь ноль семь ноль восемь три
восемь девять три"). The "г." homograph SHALL be resolved by its companion: a
year value makes it the year abbreviation, a capitalised word makes it the
city abbreviation ("г. Москва" → "город Москва"), and any other number makes
it the gram unit.

A phone number SHALL read in groups: three to six groups of up to seven
digits each, at least ten digits in total, separated by spaces, parentheses or
hyphens; a group longer than four digits SHALL be allowed only as the final
local number; a leading "+" SHALL read as "плюс" (an unlabeled run is claimed
only at a word start); a group of three or more digits starting with zero
SHALL read digit by digit; and a "тел." phone SHALL read with the word
"телефон" first, its first group starting with 8 or +.

#### Scenario: Currency symbol before and after the number

- GIVEN the inputs "100 ₽", "$5", "100 $" and "€ 20"
- WHEN the pipeline processes them with the reference profile
- THEN they read as "сто рублей", "пять долларов", "сто долларов" and
  "двадцать евро"

#### Scenario: Currency words

- GIVEN the inputs "100 руб." and "50 коп."
- WHEN the pipeline processes them with the reference profile
- THEN they read as "сто рублей" and "пятьдесят копеек"

#### Scenario: Money amount with groups and a scale word

- GIVEN the inputs "2 500 ₽", "5 тыс. ₽" and "1,5 млн $"
- WHEN the pipeline processes them with the reference profile
- THEN they read as "две тысячи пятьсот рублей", "пять тысяч рублей" and
  "полтора миллиона долларов"

#### Scenario: Physical unit agreement

- GIVEN the inputs "5 км", "2 кг" and "1,5 л"
- WHEN the pipeline processes them with the reference profile
- THEN the units agree with the count: "пять километров", "два килограмма" and
  "полтора литра"

#### Scenario: Units with a spoken tail

- GIVEN the inputs "20 °C", "5°C", "100 км/ч" and "9 м/с"
- WHEN the pipeline processes them with the reference profile
- THEN they read as "двадцать градусов Цельсия", "пять градусов Цельсия",
  "сто километров в час" and "девять метров в секунду"

#### Scenario: References

- GIVEN the inputs "№ 5", "стр. 15", "табл. 3", "рис. 3" and "гл. 2"
- WHEN the pipeline processes them with the reference profile
- THEN they read as "номер пять", "страница пятнадцать", "таблица три",
  "рисунок три" and "глава вторая"

#### Scenario: Digit-by-digit identifiers

- GIVEN the inputs "ИНН 7707083893" and "арт. 123456"
- WHEN the pipeline processes them with the reference profile
- THEN the numbers read digit by digit: "ИНН семь семь ноль семь ноль восемь
  три восемь девять три" and "артикул один два три четыре пять шесть"

#### Scenario: Phone reads in groups

- GIVEN the input "тел. +7 999 123-45-67"
- WHEN the pipeline processes it with the reference profile
- THEN it reads as "телефон плюс семь девятьсот девяносто девять сто двадцать
  три сорок пять шестьдесят семь"

#### Scenario: City abbreviation

- GIVEN the input "г. Москва"
- WHEN the pipeline processes it with the reference profile
- THEN the abbreviation reads "город" and the output is "город Москва"
