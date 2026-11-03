## MODIFIED Requirements

### Requirement: English words, abbreviations, and transliteration

The system SHALL replace every remaining English word with speakable
Cyrillic. Special language names `C++`, `C#`, `F#` (any case) SHALL be
replaced first ("си плюс плюс", "си шарп", "эф шарп"). Single Latin letters
(length-1 runs) SHALL be read by their English letter names via the shared
letter-name table ("a" → "эй", "I" → "ай", "x" → "икс"),
case-insensitively; they SHALL NOT be recorded in the unknown-words map.
Multi-word input SHALL have no reading of its own: the pipeline SHALL resolve
each English word individually, so a phrase reads exactly as the concatenation
of its words' readings and the reading SHALL NOT depend on how the words are
separated (a space, a tab, a collapsed space run, or a line break) or on the
letter case. The words of such phrases SHALL be curated individually in
`IT_TERMS` (`stack` → "стэк", `live` → "лайв", `dry` → "драй", `run` → "ран",
`edge` → "эдж"), so no phrase entry is needed to read them.
For remaining multi-letter words the resolution order SHALL be: `IT_TERMS`
dictionary ("api" → "эй пи ай", "github" → "гитхаб"), which SHALL hold
pronunciation readings only: a word whose spelling-rule transliteration already
reads correctly SHALL NOT be listed, and a Russian semantic translation SHALL
NOT be curated as its reading ("function" reads "фанкшин", not "функция"). An
entry SHALL remain only when removing it changes a reading on another
resolution path: it keeps an all-caps token from letter-by-letter spelling
("TEST" reads "тест", "DATA" reads "дата", "STRING" reads "стринг"), it shadows
a wrong inflection composition ("living" reads "ливинг", not "лайвинг"), or it
supplies a stem whose inflected forms would otherwise degrade ("drive" keeps
"driver"/"driving" readable), or it is an irregular reading the rules cannot
express ("kubernetes", "nginx"). When
an exact `IT_TERMS` entry does not exist but the word ends in a recognized
English inflection suffix (`s`, `es`, `ies`, `ed`, `ing`, `er`, `ers`, `able`,
`ally`, `ment`, `ments`, `ation`) and its stem is an `IT_TERMS` entry,
compose the curated stem reading with the spelling-rule transliteration of the
suffix ("supporting" → "саппорт" + "инг" = "саппортинг"). When the stripped stem
is not itself an `IT_TERMS` entry, the composition SHALL retry with the silent
"e" the stem dropped restored ("uses" → `use` + "es" = "юзес", "cached" →
`cache` + "ed" = "кэшед"), except for the "-ally" suffix, which attaches to an
"-al" base ("practical" → "practically") and SHALL keep the plain reading. The
retry SHALL read "-es" as the syllabic "ес" only when the stem that dropped the
"e" ends in a sibilant and otherwise SHALL leave the plain "-s" composition to
win ("packages" → "пэкэджес", not "пэкэджеес"). The "-ies" suffix
SHALL compose only when `stem + "y"` is an `IT_TERMS` entry, dropping a trailing
"й" from the base reading ("libraries" → `library` → "лайбрари" + "с" =
"лайбрарис"), so a stem that is not a "-y" base does not compose through it
("species" does not compose from `spec`). This composition
SHALL apply regardless of the word's letter case — including an
all-uppercase word — and SHALL be attempted before the all-uppercase
abbreviation rule. An all-uppercase token absent from `IT_TERMS`, `AS_WORD`,
and `SPECIAL_CASES` SHALL be read as a word by spelling-rule transliteration
when it is word-like — at least five letters and at least one vowel per four
letters, counting "y" as a vowel ("DEVELOPER" → "девелопер", "LINUX" →
"линукс", "JAVASCRIPT" → "джаваскрипт", "SYSTEM" → "систем"); a word-like token
SHALL be recorded in the unknown-words map because it is resolved by
transliteration. Every other all-uppercase token SHALL be spelled via
`AbbreviationNormalizer` (special cases like "ios" → "ай оу эс", `AS_WORD`
entries like "json" → "джейсон", otherwise letter-by-letter via the same
shared letter-name table); `AS_WORD` dictionary for mixed-case entries; and
finally spelling-rule transliteration in alphabetic runs.

Common function words SHALL be curated in a `FUNCTION_WORDS` table consulted
before `IT_TERMS` and before the spelling rules, and in any letter case: "the"
reads "зэ" and "and" reads "энд", while the rules alone read them as "те" and
"анд".

Spelling-rule transliteration SHALL be context-aware, not a fixed digraph
table. It SHALL apply, in order: whole-suffix rules (`-tion` → "шн", `-sion` →
"жн", `-ture` → "че", `-cial`/`-tial` → "шл", `-sure` → "жер"/"шер", `-gine` →
"джин", unstressed `-age` → "эдж", and the common
`-ive`/`-ate`/`-ade` endings); a silent final `e` that lengthens the nearest
preceding vowel ("file" → "файл", "page" → "пейдж", "type" → "тайп"); vowel
digraphs (`ai/ay` → "эй", `ea/ee/ie` → "и", `oa/ow` → "оу", `oo` → "у", `ou` →
"ау", `ue` → "ю", `ui` → "уи", `au/aw` → "о", `ew` → "ью", `oi/oy` → "ой");
soft `c`/`g` before `e`/`i`/`y`; voiced `s` between vowels and in `-sion`/
`-sure` while "system" stays unvoiced; `th` as "с" before a back vowel
("method" is an irregular exception: "мэтод") and "т" otherwise ("thin" →
"тин"); `ck` → "к", `qu` →
"кв", `x` → "кс"/"кз", and final `-le` → "л". Vowel reduction and stress
placement SHALL remain out of scope; irregular words SHALL stay in `IT_TERMS`.

Words resolved by transliteration SHALL be recorded in the unknown-words map,
which SHALL be cleared at the start of every `process_with_char_mapping`
call. Derived forms from known `IT_TERMS` stems SHALL NOT be recorded as
unknown words. The former `EnglishNormalizer::add_custom_terms` hook is
removed — user overrides are owned by the user-dictionary pre-pass, which
runs before this phase (see "User dictionary application in normalization").

#### Scenario: Silent e and vowel digraphs

- GIVEN the words `file`, `page`, `type`, and `rain`
- WHEN the pipeline transliterates them
- THEN the output reads `файл`, `пейдж`, `тайп`, and `рэйн`

#### Scenario: Soft c and g

- GIVEN the words `city`, `generic`, and `gitlab`
- WHEN the pipeline transliterates them
- THEN `city` and `generic` read with soft `с`/`дж`, and `gitlab` keeps its
  hard `г`

#### Scenario: Voiced s and suffix rules

- GIVEN the words `user`, `version`, `function`, `feature`, and `special`
- WHEN the pipeline transliterates them
- THEN the output reads `юзер`, `вержн`, `фанкшин`, `фиче`, and `спешл`

#### Scenario: Verified irregular readings

- GIVEN the words `method`, `function`, `above`, and `apple`
- WHEN the pipeline normalizes them
- THEN the output reads `мэтод`, `фанкшин`, `эбав`, and `эпл`; these readings
  are curated because the rules cannot distinguish their vowel/`th` contexts

#### Scenario: Rule readings replace redundant dictionary entries

- GIVEN the words `package` and `feature`
- WHEN the pipeline normalizes them
- THEN the output reads `пэкэдж` and `фиче`, not the former semantic
  translations `пакет` and `фича`; `package` keeps an entry only as a stem for
  `packages`, and `feature` has no entry

#### Scenario: Guarded all-caps tokens still use the dictionary

- GIVEN the tokens `TEST`, `DATA`, and `APP`
- WHEN the pipeline normalizes them
- THEN they read `тест`, `дата`, and `апп` from `IT_TERMS` instead of being
  spelled letter by letter

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

#### Scenario: Phrase reads by its curated words

- GIVEN the input "Вот stack trace из логов"
- WHEN the pipeline processes it
- THEN the words resolve through their curated single-word readings
  ("Вот стэк трейс из логов") with no phrase lookup involved

#### Scenario: Former phrase words are curated individually

- GIVEN the inputs "stack", "live", "dry", "run", "edge"
- WHEN the pipeline processes them
- THEN they are read as "стэк", "лайв", "драй", "ран", "эдж" from `IT_TERMS`
  and are not recorded in the unknown-words map

#### Scenario: Phrase reading is independent of separation

- GIVEN the inputs "stack trace", "Смотри stack  trace в логах" (a doubled
  space, collapsed by the whitespace phase before the English phase runs),
  "stack\ttrace" (a tab, collapsed the same way), and "stack\ntrace" (a line
  break, which the per-word pass treats as a boundary like any other)
- WHEN the pipeline processes them
- THEN the words read as "стэк" and "трейс" in every form — the separator
  between the words never changes the reading and never leaves Latin letters

#### Scenario: User dictionary entry outranks the curated reading

- GIVEN a user dictionary entry mapping "stack" to "стек"
- WHEN the pipeline processes "stack trace"
- THEN the entry wins over the built-in reading and the words resolve as
  "стек трейс"

#### Scenario: Soft-c transliteration fallback

- GIVEN the input "city events"
- WHEN the pipeline processes it
- THEN the unknown alphabetic components are read with the soft-c pre-pass
  before digraph transliteration

#### Scenario: Inflected IT-term form composed

- GIVEN the input "supporting"
- WHEN the pipeline processes it
- THEN the word is read as "саппортинг", using the curated `support` reading
  instead of the raw transliteration, and is not recorded in the
  unknown-words map

#### Scenario: All-caps inflected IT-term form composed

- GIVEN the input "SUPPORTING"
- WHEN the pipeline processes it
- THEN the word is read as "саппортинг", composing the curated `support`
  reading instead of spelling the letters out, and is not recorded in the
  unknown-words map

#### Scenario: Silent-e stem restored before composing

- GIVEN the inputs "using", "uses", "used", "caches", "cached"
- WHEN the pipeline processes them
- THEN they are read as "юзинг", "юзес", "юзед", "кэшес", "кэшед" from the
  curated `use`/`cache` readings with the silent "e" restored, and they are
  not recorded in the unknown-words map

#### Scenario: Non-sibilant "-es" keeps the plain plural

- GIVEN the inputs "packages" and "supports"
- WHEN the pipeline processes them
- THEN they are read as "пэкэджес" and "саппортс" — the syllabic "-es" for a
  sibilant stem and the plain "-s" for a non-sibilant one

#### Scenario: "-ally" is not a silent-e suffix

- GIVEN the input "practically"
- WHEN the pipeline processes it
- THEN it is read as "практикалли" — the plain transliteration, not a
  composition from a restored `practice` base ("практисалли")

#### Scenario: "-ies" composed only from a "-y" base

- GIVEN the inputs "libraries" and "species"
- WHEN the pipeline processes them
- THEN "libraries" is read as "лайбрарис" from the curated `library` reading
  with its trailing "й" dropped, while "species" does not compose from `spec`
  and falls back to transliteration

#### Scenario: Long vowel-rich all-caps token read as a word

- GIVEN the inputs "DEVELOPER", "LINUX", "UBUNTU", "JAVASCRIPT"
- WHEN the pipeline processes them
- THEN they are read as "девелопер", "линукс", "убунту", "джаваскрипт" via
  digraph transliteration instead of being spelled letter by letter, and are
  recorded in the unknown-words map

#### Scenario: Short or vowel-poor all-caps token stays spelled

- GIVEN the inputs "API", "HTML", "SQL"
- WHEN the pipeline processes them
- THEN they keep the letter-by-letter readings "эй пи ай", "эйч ти эм эл",
  "эс кью эл"

#### Scenario: Curated readings outrank the word-like rule

- GIVEN the inputs "KUBERNETES", "OAUTH"
- WHEN the pipeline processes them
- THEN "KUBERNETES" reads "кубернетис" and "OAUTH" reads "о аус" from
  `IT_TERMS`, and neither is transliterated through the word-like rule

#### Scenario: Semantic translation is not curated

- GIVEN the inputs "array", "object", "repository", "boolean", and "refactor"
- WHEN the pipeline processes them
- THEN they are read by transliteration ("аррэй", "обджект", "репозитори",
  "булин", "рефактор") because `IT_TERMS` holds pronunciation only, and they
  are recorded in the unknown-words map

#### Scenario: A semantic translation survives only as a caps guard

- GIVEN the input "string", whose curated reading was the translation "строка"
- WHEN the pipeline processes "string" and "STRING"
- THEN both read "стринг" — the entry keeps its transliteration reading only to
  stop the all-caps rule from spelling the token out, and it is not recorded in
  the unknown-words map

#### Scenario: Redundant curated reading is not listed

- GIVEN a word whose digraph transliteration already matches its curated
  reading, e.g. "index" or "commit"
- WHEN the pipeline processes it
- THEN the reading is the transliteration ("индекс", "коммит") with no
  `IT_TERMS` entry, and the word is recorded in the unknown-words map

#### Scenario: A no-op entry survives only as an all-caps guard

- GIVEN the words "test", "data", "app", "class", "web", "string", whose
  curated readings equal the transliteration
- WHEN an uppercase token such as "TEST" or "DATA" is processed
- THEN the curated entry still applies ("тест", "дата", "стринг") instead of the
  letter-by-letter spelling the all-caps rule would produce, so these entries
  are not redundant

#### Scenario: Function words use their curated readings

- GIVEN the Silero profile and the input "The API and the cache"
- WHEN the pipeline processes it
- THEN the article reads "зэ" both times and the conjunction reads "энд", not
  "те" and "анд"

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
- THEN the date reads "зэ твелфс ов мэй, твенти твенти-фо" and no Russian month
  or ordinal appears

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
- THEN the range reads "тен ту твенти пёрсэнт" and neither bound is a Russian
  numeral

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
- THEN the construct reads "уан хандрэд мегабайтс" and no Russian unit word
  appears
