## MODIFIED Requirements

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

A size, duration or version construct whose context is English (the Numbers rule)
SHALL be read in English on the narrated route: the numeral uses the English
cardinal register, a size or duration unit takes its English singular at exactly
one and its English plural otherwise ("1 MB" → "one megabyte", "100 MB" → "one
hundred megabytes"), a unit with a fixed reading keeps its English name ("1.5em"
→ "one point five em"), and a version or dot decimal joins its groups with
" point ", reading a leading-zero group digit by digit ("1.2.3" → "one point two
point three"). The reading SHALL be transliterated into Cyrillic by the effective
prose route, and a construct carrying a size or duration unit SHALL be claimed
only when the profile expands units.

In English context a size, duration or version construct SHALL be spelled as
Latin English words on the native route, replacing the construct in the text: a
size or duration unit takes its English singular at exactly one and its English
plural otherwise ("100 MB" → "one hundred megabytes"), a unit with a fixed
reading keeps its English name ("1.5em" → "one point five em"), and a version or
dot decimal joins its groups with " point ", reading a leading-zero group digit
by digit ("1.2.3" → "one point two point three"). The replacement SHALL be
covered by one prose-Latin span (see Span annotation output) so the construct
continues the English block, and a size or duration construct SHALL be spelled
only when the profile expands units.

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

#### Scenario: English-context size reads the English unit

- GIVEN the Silero profile and the input "the cache holds 50 MB"
- WHEN the pipeline processes it
- THEN the size reads "фифти мегабайтс" and no Russian unit word appears

#### Scenario: English-context size is spelled on the native route

- GIVEN the TeraTTS profile and the input "the cache holds 50 MB"
- WHEN the pipeline processes it
- THEN the output reads "the cache holds fifty megabytes" and one prose-Latin
  span with language `en` covers the replacement

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

A date or time construct whose context is English (the Numbers rule) SHALL be
read in English on the narrated route: a date reads "the <day ordinal> of
<Month>, <year register>" ("12.05.2024" → "the twelfth of May, twenty
twenty-four"), a year-month reads "<Month> <year register>" ("2024-12" →
"December twenty twenty-four"), and a clock time reads "<hour> <minutes>" with
"oh" before a one-digit minute and the hour alone at zero minutes ("09:30" →
"nine thirty", "09:05" → "nine oh five"), with non-zero seconds appended as "and
<n> seconds". The readings SHALL be transliterated into Cyrillic on the narrated
route.

In English context a date or time construct SHALL be spelled as Latin English
words on the native route: a date reads "the <day ordinal> of <Month>, <year
register>" ("12.05.2024" → "the twelfth of May, twenty twenty-four"), a
year-month reads "<Month> <year register>" ("2024-12" → "December twenty
twenty-four"), and a clock time reads "<hour> <minutes>" with "oh" before a
one-digit minute and the hour alone at zero minutes ("09:30" → "nine thirty",
"09:05" → "nine oh five"), with non-zero seconds appended as "and <n> seconds".
The replacement SHALL be covered by one prose-Latin span.

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

#### Scenario: English-context date reads the English form

- GIVEN the Silero profile and the input "the release landed on 12.05.2024"
- WHEN the pipeline processes it
- THEN the date reads "зе твелфс ов мэй, твенти твенти-фо" and no Russian month
  or ordinal appears

#### Scenario: English-context date is spelled on the native route

- GIVEN the TeraTTS profile and the input "the release landed on 12.05.2024"
- WHEN the pipeline processes it
- THEN the output reads "the release landed on the twelfth of May, twenty
  twenty-four" and one prose-Latin span covers the date

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

A range or percentage construct whose context is English (the Numbers rule) SHALL
be read in English on the narrated route: a range reads "<left> to <right>"
("10-20" → "ten to twenty"), a range whose bounds are calendar years reads both
bounds in the year register ("2020–2024" → "twenty twenty to twenty
twenty-four"), a percentage reads the cardinal plus the invariant "percent"
("50%" → "fifty percent", "1%" → "one percent"), and a shared trailing unit,
percentage or currency attaches to the right bound ("10–20%" → "ten to twenty
percent"). The English reading carries no genitive agreement: a genitive trigger
in an English sentence does not change the wording.

In English context a range or percentage construct SHALL be spelled as Latin
English words on the native route: a range reads "<left> to <right>" ("10-20" →
"ten to twenty"), a range whose bounds are calendar years reads both bounds in
the year register ("2020–2024" → "twenty twenty to twenty twenty-four"), a
percentage reads the cardinal plus the invariant "percent" ("50%" → "fifty
percent"), and a shared trailing unit, percentage or currency attaches to the
right bound ("10–20%" → "ten to twenty percent"). The replacement SHALL be
covered by one prose-Latin span.

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

#### Scenario: English-context range and percentage

- GIVEN the Silero profile and the input "growth of 10-20%"
- WHEN the pipeline processes it
- THEN the range reads "тен ту твенти персент" and neither bound is a Russian
  numeral

#### Scenario: English-context range is spelled on the native route

- GIVEN the TeraTTS profile and the input "growth of 10-20%"
- WHEN the pipeline processes it
- THEN the output reads "growth of ten to twenty percent" and one prose-Latin
  span covers the range

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

A standalone integer that the layer reads as a cardinal SHALL be read in English
instead when its context is English: the nearest word on each side SHALL be
English prose, or English prose on one side and a sentence edge on the other
(start or end of the text, a paragraph break, terminal punctuation), and the
sentence SHALL contain no Cyrillic word. Whitespace, punctuation and other
digits SHALL be transparent while the neighbour is found, so a comma does not
end the sentence. A Russian word on either side SHALL keep the Russian reading;
a Latin identifier, a URL, an inline code fragment or a user-dictionary token
SHALL NOT count as English prose. The English reading SHALL use the cardinal
register with no "and" between hundreds and tens, and a value above 999 999 999
SHALL read digit by digit.

The English reading SHALL be rendered by the effective prose route (see the
English reading setting requirement): the `Narrate` route SHALL transliterate it
into Cyrillic with the curated numeral readings, and the `Native` route SHALL
spell the reading in Latin words, replace the digits with the spelling and
annotate the replacement with a prose-Latin span (see Span annotation output), so
the English block is not split and the text matches what is spoken. On the
`Native` route, a four-digit value the Russian reader treats as a year
(1000–2199) SHALL use the year register — two two-digit pairs ("2024" → "twenty
twenty-four", "1905" → "nineteen oh five", "1900" → "nineteen hundred") —
while a round thousand keeps the cardinal ("2000" → "two thousand").

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

The English-context rule SHALL apply to a numeric construct as a whole, not only
to a standalone integer: a number that owns a unit, percentage, currency, date,
time, version, range, reference or ordinal suffix SHALL be claimed as one span,
and the neighbour and sentence tests SHALL run against that span's boundaries
("the API grew 50%" reads the whole "50%" as English prose, while "Установи 5
кг." stays Russian). The construct reading SHALL be the English wording of the
whole construct (see Sizes, durations, and versions; Dates and times; Ranges and
percentages), transliterated into Cyrillic on the narrated route. A construct
outside those families — a phone number, a Roman numeral, a city abbreviation, a
comma decimal, a slash fraction or a pre-release version suffix — SHALL keep its
Russian reading.

On the `Native` route such a construct SHALL be spelled as Latin English
words, replace the source construct in the text and carry one prose-Latin span
(see Span annotation output), so the construct continues the English block; the
`Narrate` route SHALL transliterate it into Cyrillic and SHALL produce no
span. A construct outside those families SHALL keep its Russian reading on both
routes.

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

#### Scenario: English-context number on the native route

- GIVEN the TeraTTS profile and the input "In 2024 we shipped 3 releases."
- WHEN the pipeline processes it
- THEN the text is `In twenty twenty-four we shipped three releases.`, each
  reading carries a prose-Latin span with language `en`, and no digits, Russian
  numeral or language flip remain at the numbers

#### Scenario: English-context number on the narrated route

- GIVEN the Silero profile and the input "In 2024 we shipped 3 releases."
- WHEN the pipeline processes it
- THEN the numbers read as the English cardinals "two thousand twenty-four" and
  "three", transliterated into Cyrillic, and no span is emitted

#### Scenario: English-context number at the sentence start

- GIVEN the TeraTTS profile and the input "3 releases shipped."
- WHEN the pipeline processes it
- THEN the number reads in English even though the sentence starts with it

#### Scenario: Russian sentence with an English name keeps the number Russian

- GIVEN the reference profile and the input "Установи Python 3."
- WHEN the pipeline processes it
- THEN the number reads "три", because the sentence contains a Cyrillic word

#### Scenario: Identifier next to a number is not English prose

- GIVEN the TeraTTS profile and the input "getUserData 3"
- WHEN the pipeline processes it
- THEN the number keeps the Russian reading, because the identifier is not
  English prose

#### Scenario: English-context construct on the narrated route

- GIVEN the Silero profile and the input "The file is 100 MB."
- WHEN the pipeline processes it
- THEN the construct reads "уан хандред мегабайтс" and no Russian unit word
  appears

#### Scenario: English-context construct on the native route

- GIVEN the TeraTTS profile and the input "Version 1.2.3 shipped on 12.05.2024."
- WHEN the pipeline processes it
- THEN the version and the date read as Latin English words, each construct
  carries one prose-Latin span with language `en`, and no Russian word appears
  inside the sentence

### Requirement: Span annotation output

The pipeline output SHALL carry, besides the normalized text and its
`CharMapping`, a list of span annotations. Each span SHALL state its kind (code
identifier, prose Latin run, fenced code block, dictionary override), its
language when the fragment is native, and its range (see `position-mapping`
for the coordinate space and the invariants).

A fragment SHALL be annotated only when the pipeline leaves it to the engine: a
prose Latin run only under the effective prose route `Native` (see the English
reading setting requirement), an identifier or a code block only when the
profile routes its kind `Native`; a narrated fragment SHALL produce no native
span. The span's language SHALL be the non-Russian language the profile
declares (for TeraTTS and Piper, `en`). A dictionary-override span SHALL carry
no language and SHALL stay Russian regardless of the prose route. A native
code-block span SHALL cover exactly the code text handed to the engine.

An English-context number on the `Native` route SHALL be replaced with the Latin
spelling of its reading and SHALL carry a prose-Latin span with the profile's
native language over the replacement; the mapping SHALL map every spelled word
back to the source digits; the `Narrate` route SHALL produce no span for it.

An English-context numeric construct on the `Native` route SHALL likewise be
replaced with its Latin English reading and SHALL carry one prose-Latin span
over the whole replacement; the mapping SHALL map every spelled word back to the
source construct range, and the `Narrate` route SHALL transliterate the
construct and SHALL produce no span for it.

Annotations SHALL express decisions, never markup: no span SHALL contain tag
syntax, and the normalized text SHALL remain the exact synthesis input.

#### Scenario: Dictionary override in a narrating profile

- GIVEN the reference profile and a user dictionary entry that rewrites a
  standalone Latin token
- WHEN the pipeline processes a text containing that token
- THEN a dictionary-override span covers the replacement in the output

An override applied *inside* a narrated identifier is not annotated
separately: the identifier phase replaces the whole identifier as one unit, so
its `char_map` entry maps every replacement codepoint to the whole original
identifier. Producing a part-level span would require part-wise replacement and
would change the mapping of every identifier in the corpus — that is #308's
territory, and this change keeps the mapping stable.

#### Scenario: Native prose and identifier spans

- GIVEN a profile that reads Latin natively and the effective prose route
  `Native`
- WHEN the pipeline processes `Kubernetes и getUserData()`
- THEN `Kubernetes` carries a prose-Latin span with language `en` and
  `getUserData` carries a code-identifier span with language `en`

#### Scenario: Narrated prose keeps the identifier span

- GIVEN a profile that reads Latin natively and routes identifiers `Native`,
  and the English reading setting `narrated`
- WHEN the pipeline processes `Kubernetes и getUserData()`
- THEN no prose-Latin span is produced, and the output still carries the
  code-identifier span over the verbatim `getUserData`

#### Scenario: Native code-block span

- GIVEN the TeraTTS profile, the read mode and a text containing a fenced
  code block
- WHEN the pipeline processes it
- THEN one code-block span with language `en` covers exactly the code text in
  the output, and no prose or identifier span overlaps it

#### Scenario: Narrating profile emits no native spans

- GIVEN the Silero profile
- WHEN the pipeline processes the same text
- THEN every Latin fragment is transliterated and no span carries a language

#### Scenario: No markup leaks into the text

- GIVEN any profile and any fixture
- WHEN the pipeline processes it
- THEN the normalized text contains no tag inserted by the pipeline, and every
  span range falls inside the text

#### Scenario: English-context number on the native route is annotated

- GIVEN the TeraTTS profile and the input "In 2024 we shipped 3 releases."
- WHEN the pipeline processes it
- THEN each reading carries a prose-Latin span with language `en`, the spans do
  not overlap the surrounding prose spans, and every spelled word maps back to
  the source digit range

#### Scenario: English-context construct on the native route is annotated

- GIVEN the TeraTTS profile and the input "The API grew 50%."
- WHEN the pipeline processes it
- THEN "50%" reads "fifty percent", one prose-Latin span with language `en`
  covers the whole replacement, and every spelled word maps back to the source
  range
