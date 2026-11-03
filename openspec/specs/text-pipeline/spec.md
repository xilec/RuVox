# Text Pipeline Specification

## Purpose

Covers the normalization pipeline (`TTSPipeline`, implemented in Rust under
`tts-normalize/src/pipeline/`) that converts technical text — Markdown, code,
URLs, numbers, dates, English words — into Cyrillic-only Russian text suitable
for Silero TTS, which cannot read English or special characters. The pipeline
is the mandatory preprocessing step before every synthesis. Character-level
position mapping produced alongside the normalized text is specified
separately in the `position-mapping` capability.
## Requirements
### Requirement: Pipeline entry points and integration

The system SHALL normalize all text through `TTSPipeline` before TTS
synthesis. `TTSPipeline::process_for_engine(input, &profile)` SHALL be the
primary entry point and SHALL return the normalized text, its `CharMapping` and
its span annotations. `TTSPipeline::process_with_char_mapping(input)` SHALL
remain as a compatibility wrapper equivalent to
`process_for_engine(input, REFERENCE_PROFILE)`, returning text and mapping
only, where the reference profile is the Silero one. The system SHALL NOT
expose a bare `process(input)` that discards the mapping.

The pipeline SHALL be shared as `Arc<Mutex<TTSPipeline>>` in `AppState` and
invoked from the `add_text_entry` / `add_clipboard_entry` commands
(normalization before synthesis) and from `preview_normalize` (normalization
preview dialog). Both call sites SHALL read the profile from the active engine
in the same lock-free read that supplies the code-block narration mode and the
voice, so the text is normalized for the engine the caller saw. A later switch
between normalization and synthesis is fixed by the generation snapshot, not
by holding the pipeline lock, which no longer spans both steps. The preview
SHALL therefore show the text the engine seen by the dialog will speak.

When normalization yields an empty string, the system SHALL reject synthesis
with the user-visible error "нормализация вернула пустой текст".

#### Scenario: Normalization before synthesis

- GIVEN the user adds the text "Вызови getUserData() через API"
- WHEN `add_text_entry` runs the normalization phase with the reference profile
- THEN the pipeline produces "Вызови гет юзер дата() через эй пи ай"
  (parentheses pass through verbatim; only the identifier and the English
  word are normalized) and synthesis proceeds with the normalized text

#### Scenario: Empty input

- GIVEN an empty input string
- WHEN `process_for_engine` is called
- THEN the system returns an empty string, an empty `CharMapping` and no spans,
  without running any phase

#### Scenario: Whitespace-only input rejected

- GIVEN input that contains only whitespace
- WHEN the pipeline processes it
- THEN the normalized result is empty and the synthesis command fails with
  "нормализация вернула пустой текст"

#### Scenario: process and process_with_char_mapping are consistent

- GIVEN any input text
- WHEN both `process_for_engine(input, REFERENCE_PROFILE)` and
  `process_with_char_mapping(input)` are called on it
- THEN both return the same text and the same `CharMapping`

#### Scenario: Preview follows the active engine

- GIVEN the active engine's profile routes prose Latin runs `Native`
- WHEN `preview_normalize` runs
- THEN the previewed text keeps the Latin run instead of transliterating it

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

### Requirement: Input pre-normalization

The system SHALL strip a leading BOM (`U+FEFF`) from the input. The system
SHALL normalize quotation marks «, », ", " to `"` and ', ' to `'`, and
SHALL normalize em-dash (—) and en-dash (–) to `-`. The system SHALL
collapse three or more consecutive newlines to exactly two and collapse runs
of spaces/tabs to a single space before any content phase runs.

#### Scenario: Quotes and dashes normalized

- GIVEN the input "Слово — «кавычки»"
- WHEN the pipeline processes it
- THEN the em-dash becomes "-" and the guillemets become straight quotes in
  the output

#### Scenario: Blank-line collapse

- GIVEN input paragraphs separated by four consecutive newlines
- WHEN the pipeline processes it
- THEN the output contains at most two consecutive newlines between the
  paragraphs

### Requirement: Fenced code blocks

The system SHALL process fenced code blocks (` ```lang ... ``` `) before all
other content phases. A block tagged `mermaid` SHALL be replaced with the
exact marker string "Тут мермэйд диаграмма" and MUST NOT be read aloud, in
every narration mode and under every profile.

For every other block the pipeline's code block narration mode decides whether
the block is read at all, and the active profile's code-block route decides
who reads it. The application sets the mode from the `code_block_mode` config
value (`"brief"` default, `"read"` optional; the wiring lives in the
`ipc-commands` capability):

- **Brief** (`"brief"`): the block is replaced with "далее следует пример
  кода на <язык>" (language looked up in `LANGUAGE_NAMES`) or "далее следует
  блок кода" when no language tag is present. The route is not consulted.
- **Full, `Narrate`** (`"read"`): block contents are tokenized and read aloud
  with identifiers, operators, brackets, and integer literals normalized to
  spoken Russian.
- **Full, `Native`** (`"read"`): the block's content is handed to the engine
  as text. Its surrounding whitespace is trimmed and internal whitespace runs
  are collapsed to one space; the `CharMapping` and word timestamps keep
  pointing into the original fenced block.

A `Native` block SHALL NOT be rewritten by any narrating phase: the English,
identifier, operator, symbol, number and user-dictionary phases SHALL NOT see
it, and a `Native` block SHALL NOT be annotated as native prose or as
identifiers. Its codepoints map to the whole source fenced block, which is the
same granularity the full Russian narration already has.

The pipeline's implicit default mode SHALL be Brief, matching the config
default, so a freshly constructed pipeline never contradicts the product
default. The former inline mode-switch directives (`<!-- ruvox-code: … -->`)
SHALL no longer be interpreted: such comments are processed as ordinary text
by the symbol phases exactly like any other punctuation, and the narration
mode is owned solely by the setting.

#### Scenario: Mermaid block replaced with marker

- GIVEN the input "```mermaid\ngraph TD\nA-->B\n```"
- WHEN the pipeline processes it in any mode
- THEN the entire block is replaced with "Тут мермэйд диаграмма"

#### Scenario: Brief mode is the default

- GIVEN a pipeline constructed without an explicit mode and the input
  "```python\nprint('hi')\n```"
- WHEN the pipeline processes it
- THEN the block is replaced with "далее следует пример кода на пайтон"

#### Scenario: Brief mode without a language tag

- GIVEN the pipeline in Brief mode and the input "```\nx = 1\n```"
- WHEN the pipeline processes it
- THEN the block is replaced with "далее следует блок кода"

#### Scenario: Code block read in full mode

- GIVEN the reference (Silero) profile, the pipeline configured in Full mode
  and the input "```python\nprint('hi')\n```"
- WHEN the pipeline processes it
- THEN the block content is read as "принт открывающая скобка хи
  закрывающая скобка"

#### Scenario: Native code keeps its content and line breaks do not glue words

- GIVEN the TeraTTS profile, the read mode and the input
  "```python\ndef hello():\n    print('hi')\n```"
- WHEN the pipeline processes it
- THEN the output is "def hello(): print('hi')" — one space between the two
  lines, no Russian reading of any token

#### Scenario: Mode-switch directive

- GIVEN the pipeline in Full mode and the text
  "<!-- ruvox-code: brief -->\n```python\nprint('hi')\n```"
- WHEN the pipeline processes it
- THEN the block is still read in full (the directive comment does not change
  the mode) and the comment is normalized as ordinary text; the exact output
  is pinned by a golden fixture

### Requirement: Markdown constructs

The system SHALL normalize Markdown structure before content phases: inline
code spans (`` `code` ``) SHALL be processed as code (snake_case, kebab-case,
or camelCase normalization applied to the span content, with Greek letters
and arrows pre-expanded); ATX headings (`#` through `######`) SHALL be
stripped while the heading text is kept; links `[text](url)` SHALL be reduced
to their link text, with the URL removed and the text left available for
later normalization phases; numbered list markers `N.` at line start SHALL be
replaced with the Russian ordinal followed by a colon ("первое:", "второе:",
…, "десятое:" for 1–10, and the cardinal number words for larger numbers).

#### Scenario: Heading stripped

- GIVEN the input "## Установка"
- WHEN the pipeline processes it
- THEN the output is "Установка" with no hash characters

#### Scenario: Link reduced to text

- GIVEN the input "См. [документацию](https://example.com/docs)"
- WHEN the pipeline processes it
- THEN the output contains "документацию" and does not contain the URL

#### Scenario: Numbered list marker

- GIVEN the input "1. Первый пункт"
- WHEN the pipeline processes it
- THEN the marker (including its trailing space) is replaced so the line
  reads "первое:Первый пункт"

#### Scenario: Inline code normalized as identifier

- GIVEN the input "Вызови `get_user` здесь"
- WHEN the pipeline processes it
- THEN the inline code span is read as "гет юзер"

### Requirement: URLs, emails, IP addresses, and file paths

The system SHALL normalize URLs with schemes `http`, `https`, `ftp`, `ssh`,
and `git` before any number processing: the scheme SHALL be read via the
protocol table ("https" → "эйч ти ти пи эс"), followed by "двоеточие слэш
слэш"; domain parts SHALL be joined with "точка" with known TLDs read via
`TLD_MAP` ("com" → "ком"); a numeric port SHALL follow "двоеточие"; path
segments SHALL follow "слэш"; query parameters SHALL follow "вопросительный
знак" with "=" read as "равно"; fragments SHALL follow "решётка". Alphabetic
segments SHALL be transliterated (via `IT_TERMS` first, then digraph-based
transliteration). Email addresses SHALL be read with "собака" for `@`, with
dots, underscores, hyphens, and digits in the local part read as "точка",
"андерскор", "дефис", and number words. IPv4 addresses SHALL be read as four
number words joined by "точка". File paths SHALL be read with "слэш" (or
"бэкслэш" for Windows paths), "тильда" for `~`, "точка" / "точка точка" for
`.` / `..`, drive letters plus "двоеточие" for Windows drives, and "точка"
before file extensions.

The system SHALL also detect scheme-less URLs — `www.`-prefixed domains and
bare domains whose last label is a known TLD from `TLD_MAP` — with optional
path, query, and fragment, and SHALL read them like schemed URLs without the
scheme prefix: domain labels joined with "точка" (known TLDs via `TLD_MAP`),
path segments after "слэш", query after "вопросительный знак", fragment
after "решётка". Detection MUST NOT match dotted names whose last label is
not in `TLD_MAP` (filenames like `file.txt`, `test.spec.ts`, `config.yaml`),
MUST NOT match numeric dotted forms (versions `1.2.3`, dates), and MUST NOT
re-match domains already consumed as part of a schemed URL or an email
address.

URL components (host labels, path segments, query keys and values, fragment)
and email local parts SHALL be percent-decoded after the structural splits
(`/`, `&`, `=`, `@`) and before lexical processing: valid percent-encoded
UTF-8 sequences SHALL decode to their text (so an encoded Cyrillic file name
is read as that name), `%20` and other encoded ASCII SHALL decode to their
characters read by the existing rules, and encoded characters MUST NOT
change the URL structure (a decoded `%2F` is not a path separator). A `+`
inside query components SHALL be read as a word separator (space); a `+` in
email local parts, path segments, and fragments SHALL be read as "плюс".
Invalid percent sequences (truncated, non-hex, or non-UTF-8 byte runs) MUST
NOT leak a literal `%`: the `%` SHALL be read as "процент" and the following
characters read normally. No literal `%` or `+` SHALL remain in the
normalized output.

#### Scenario: HTTPS URL

- GIVEN the input "https://github.com/user/repo"
- WHEN the pipeline processes it
- THEN the output contains "эйч ти ти пи эс двоеточие слэш слэш гитхаб точка
  ком слэш юзер слэш репо"

#### Scenario: www-prefixed URL

- GIVEN the input "Сайт www.example.com недоступен"
- WHEN the pipeline processes it
- THEN the output contains "ввв точка экзампл точка ком" and no literal "."
  between the domain labels remains

#### Scenario: Bare domain with path

- GIVEN the input "документация на docs.python.org/3/tutorial"
- WHEN the pipeline processes it
- THEN the output contains "докс точка пайтон точка орг слэш три слэш
  тьюториал"

#### Scenario: Filename is not a bare domain

- GIVEN the input "открой file.txt и config.yaml"
- WHEN the pipeline processes it
- THEN neither name is read with "точка" domain separators (their suffixes
  are not in `TLD_MAP`)

#### Scenario: Version is not a bare domain

- GIVEN the input "версия 1.2.3"
- WHEN the pipeline processes it
- THEN the version is read as "один точка два точка три" by the version
  phase, not treated as a domain

#### Scenario: Email address

- GIVEN the input "user@example.com"
- WHEN the pipeline processes it
- THEN the output contains "собака" between the local part and the domain,
  and the domain is read with "точка ком"

#### Scenario: Percent-encoded space in path

- GIVEN the input "https://example.com/hello%20world"
- WHEN the pipeline processes it
- THEN the output contains "слэш хеллоу ворлд" ("hello" via `IT_TERMS`) and
  no literal "%" remains

#### Scenario: Percent-encoded Cyrillic file name

- GIVEN the input "https://example.com/%D1%84%D0%B0%D0%B9%D0%BB"
- WHEN the pipeline processes it
- THEN the output contains "слэш файл" (the bytes decode to "файл")

#### Scenario: Plus in query is a space

- GIVEN the input "https://example.com/search?q=hello+world"
- WHEN the pipeline processes it
- THEN the output contains "к равно хеллоу ворлд" and no literal "+" remains

#### Scenario: Plus in email local part

- GIVEN the input "user+tag@example.com"
- WHEN the pipeline processes it
- THEN the output contains "юзер плюс таг собака экзампл точка ком" and no
  literal "+" remains

#### Scenario: Encoded percent sign and truncated sequence

- GIVEN the input "https://example.com/100%25done%2"
- WHEN the pipeline processes it
- THEN "%25" decodes to "%" read as "процент", the truncated "%2" reads its
  "%" as "процент" followed by "два", and no literal "%" remains

#### Scenario: Unix file path

- GIVEN the input "/home/user/file.txt"
- WHEN the pipeline processes it
- THEN the output reads the path with "слэш" separators and "точка" before
  the extension

#### Scenario: Windows path with drive letter

- GIVEN the input "C:\Users\Admin\file.txt"
- WHEN the pipeline processes it
- THEN the output starts with "си двоеточие бэкслэш"

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
- THEN the date reads "зе твелфс ов мэй, твенти твенти-фо" and no Russian month
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
- THEN the range reads "тен ту твенти персент" and neither bound is a Russian
  numeral

### Requirement: Operators and special symbols

The system SHALL replace the multi-character operators `===`, `!==`, `->`,
`=>`, `>=`, `<=`, `!=`, `==`, `&&`, `||` with their spoken forms ("===" →
"строго равно", "->" → "стрелка"), processing them longest-first via
`TRACKED_OPERATOR_KEYS`; a single `=` MUST NOT be replaced at this phase so
mathematical formulas stay intact. The system SHALL replace Greek letters
(`GREEK_LETTERS`), math symbols (`MATH_SYMBOLS`), and arrow symbols
(`ARROW_SYMBOLS`) with their Russian names, surrounded by spaces. A tilde
directly before a number SHALL be read as "около" ("~46" → "около 46").

The same phase SHALL read a bare `<` and `>` that survived the operator phase
as "меньше" and "больше", also surrounded by spaces, so a comparison, a shell
redirect or a pasted markup tag is spoken instead of reaching the engine. The
readings come from the shared `SYMBOLS` table (the same one the inline-code and
identifier normalizers use), so a single table entry owns the wording.
Multi-character operators take priority: they are consumed by the operator
phase that runs first, so only the remaining single brackets are read here.
Invariant: the text handed to an engine SHALL NOT contain a stray `<` or `>`
outside a native code-block span (those spans are restored after every
narrating phase, and the TeraTTS adapter neutralises their tag delimiters
before wrapping).

#### Scenario: Strict equality operator

- GIVEN the input "a === b"
- WHEN the pipeline processes it
- THEN the operator is read as "строго равно" and not as three separate
  equals signs

#### Scenario: Greek letters

- GIVEN the input "α = β"
- WHEN the pipeline processes it
- THEN the letters are read as "альфа" and "бета" and the single "=" is left
  for later phases, not consumed by the operator phase

#### Scenario: Tilde as approximately

- GIVEN the input "~46 секунд"
- WHEN the pipeline processes it
- THEN the tilde is read as "около" before the number

#### Scenario: Comparison operator keeps its two-character reading

- GIVEN the input "если a >= b"
- WHEN the pipeline processes it
- THEN the pair is read as "больше или равно" and never as a separate
  "больше" plus "равно"

#### Scenario: Shell redirect is narrated

- GIVEN the input "grep -r key /root/.ssh 2>/dev/null"
- WHEN the pipeline processes it with any engine profile
- THEN the redirect's `>` is read as "больше" and the normalized text
  contains no `<` or `>`; under `silero`, `piper` and `teratts` the whole
  fragment reads "два больше слэш дев слэш налл", while
  `native_en` keeps the digit native

#### Scenario: Pasted markup tag is narrated

- GIVEN the input "Тег <div> тут"
- WHEN the pipeline processes it with any engine profile
- THEN the output contains neither `<` nor `>`: the opening bracket reads
  "меньше" in front of the tag name and the closing one reads "больше"

#### Scenario: Native code block keeps its tag delimiters

- GIVEN a fenced code block routed `Native` whose content is `Vec<T>`
- WHEN the pipeline processes it
- THEN the bracket narration does not touch the restored code text, and only
  the adapter neutralises the delimiters before wrapping

### Requirement: Code identifiers

The system SHALL split and normalize code identifiers before English word
processing: camelCase and PascalCase identifiers SHALL be split at case
boundaries; snake_case and SCREAMING_CASE SHALL be split at underscores;
kebab-case SHALL be split at hyphens. Each part SHALL be resolved through the
`CODE_WORDS` dictionary ("get" → "гет") with a transliteration fallback, and
numeric parts SHALL be read as Russian number words.

#### Scenario: camelCase identifier

- GIVEN the input "getUserData"
- WHEN the pipeline processes it
- THEN the identifier is read as "гет юзер дата"

#### Scenario: snake_case identifier

- GIVEN the input "max_retry_count"
- WHEN the pipeline processes it
- THEN each underscore-separated part is read as a separate spoken word

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
- THEN the construct reads "уан хандред мегабайтс" and no Russian unit word
  appears

### Requirement: Output post-processing

After all content phases the system SHALL collapse runs of spaces to a single
space, remove spaces before punctuation `.,!?;:`, remove spaces adjacent to
newlines, and trim leading/trailing whitespace from the final result. When
trimming removes characters, the system SHALL adjust the `CharMapping` so its
`char_map` length stays equal to the codepoint count of the trimmed result.

#### Scenario: Trailing whitespace trimmed with mapping intact

- GIVEN the input "  привет мир  "
- WHEN `process_with_char_mapping` is called
- THEN the result has no leading or trailing whitespace and `char_map.len()`
  equals the codepoint count of the result

### Requirement: Golden regression fixtures

The system's pipeline behavior SHALL be pinned by golden fixtures in
`src-tauri/tests/fixtures/pipeline/`: each case consists of
`<case>.input.txt`, `<case>.expected.txt`, and `<case>.char_map.json`, and
the suite SHALL be executed by
`cargo test --manifest-path src-tauri/Cargo.toml --test golden`. The fixtures
SHALL cover numbers, sizes, durations, versions, dates, times, ranges,
percentages, English words, abbreviations, code identifier styles,
URLs/emails/IPs/paths, Markdown constructs, code blocks in both narration
modes (brief by default and full via an explicitly set mode), mermaid,
symbols, operators, whitespace handling, and mixed paragraphs.

#### Scenario: Golden suite passes

- GIVEN the current pipeline implementation
- WHEN `cargo test --manifest-path src-tauri/Cargo.toml --test golden` runs
- THEN every fixture's pipeline output matches its `.expected.txt` and its
  `CharMapping` matches `.char_map.json`

### Requirement: Near-linear scaling of normalization

Normalization SHALL run in time that grows near-linearly with input size.
Each pipeline phase MUST apply its replacements in a single pass over the
text (one string rebuild per phase, not per replacement), and position-map
bookkeeping MUST be served from a sorted interval index (binary search)
without cloning the replacement history per query. Per-replacement
complexity MUST NOT depend on the total document length or on the number of
previously applied replacements.

#### Scenario: Large replacement-heavy input normalizes within budget

- GIVEN an input of ~1 MB of dense, replacement-heavy markup (tags,
  attributes, URLs, entities, mixed Cyrillic/Latin text)
- WHEN `process_with_char_mapping` runs on it
- THEN normalization completes within 10 seconds and returns a correct
  normalized text with a consistent `CharMapping`

#### Scenario: Doubling the input scales near-linearly

- GIVEN the same class of dense replacement-heavy input at sizes n and 2n
- WHEN both are normalized
- THEN the measured time for 2n is less than 4x the time for n (a quadratic
  implementation grows ~4x or worse per doubling)

### Requirement: Input length limit

The pipeline SHALL NOT impose a length-based rejection: input of any length
SHALL be accepted and normalized in full without truncation, regardless of the
active TTS engine. Inference-side memory safety is owned by the TTS engines,
which synthesize long text in bounded chunks (see the `silero-native-engine`
and `piper-engine` capabilities); normalization itself scales near-linearly
with input length.

#### Scenario: Input at the limit is fully normalized

- GIVEN an input of exactly 100 000 codepoints
- WHEN the pipeline processes it
- THEN the whole input is normalized with no content dropped

#### Scenario: Oversized input is normalized when Silero is active

- GIVEN the active TTS engine is Silero and an input longer than 100 000
  codepoints
- WHEN the pipeline processes it
- THEN the whole input is normalized with no content dropped and no
  length-based rejection occurs

#### Scenario: Oversized input is normalized when Piper is active

- GIVEN the active TTS engine is Piper and an input longer than 100 000
  codepoints
- WHEN the pipeline processes it
- THEN the whole input is normalized with no content dropped and no
  length-based rejection occurs (Piper synthesis runs in bounded chunks)

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

### Requirement: User dictionary application in normalization

User dictionary entries SHALL win over every built-in table at every lookup
site: the prose pre-pass (before English-word resolution), code-identifier
parts (before `CODE_WORDS`), URL word reading (before `IT_TERMS`), and code
blocks read aloud (through the identifier path). The pre-pass SHALL match
tokens of Latin letters and digits containing at least one Latin letter —
including alnum tokens no other phase captures, such as "IPv6" or "mp3" — and,
when the dictionary holds at least one Cyrillic key, tokens of Russian Cyrillic
letters and digits containing at least one Cyrillic letter. Each candidate
pattern SHALL run only when the dictionary has a key of its script, and the two
patterns SHALL be disjoint, so one text token is matched by at most one of
them. Replacement SHALL apply only to exact case-insensitive key hits;
non-hits SHALL flow into the English-word phase unchanged. Lookups in
identifiers and URLs SHALL use the split parts/words those phases already
produce.

The replacement SHALL be the entry's `to` as typed. When the profile declares
`Stress::None`, the replacement SHALL be the spoken form instead — the same
text with the stress mark removed; otherwise the mark SHALL stay, so the engine
applies it. A malformed mark never reaches this phase: validation rejects it on
save and skips it on load. The mark SHALL be inserted as part of the
replacement region, so no later phase reads it as an operator or a symbol, and
`CharMapping` SHALL map it to the original word like the rest of the
replacement.

#### Scenario: Entry overrides an IT term in prose

- GIVEN the entry `docker → докер` ("docker" normally transliterates)
- WHEN the input "запусти docker" is processed
- THEN the output reads "докер"

#### Scenario: Entry applies to a Russian word

- GIVEN the entry `замок → зам+ок` and a profile declaring `Stress::Auto`
- WHEN the input "старый замок стоит" is processed
- THEN the output reads "старый зам+ок стоит" and every other Russian word is
  untouched

#### Scenario: Latin-only dictionary skips the Cyrillic scan

- GIVEN a dictionary whose keys are all Latin
- WHEN a Russian document is processed
- THEN the Cyrillic candidate pattern is not run and the output matches the
  output of the same document under the previous implementation

#### Scenario: Entry overrides letter spelling of an abbreviation

- GIVEN the entry `SQL → эс ку эль`
- WHEN the input "запрос к SQL" is processed
- THEN the output reads "эс ку эль" instead of the built-in letter-by-letter
  reading

#### Scenario: Alnum token is normalized via the dictionary

- GIVEN the entry `IPv6 → айпи ви шесть` and the input "сеть IPv6 работает"
- WHEN the pipeline processes it
- THEN the output reads "айпи ви шесть"

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

#### Scenario: Stress mark reaches an engine that supports it

- GIVEN the entry `Москва → М+осква` and a profile declaring `Stress::Auto`
- WHEN the input "в Москва сегодня" is processed
- THEN the output contains "М+осква" and no phase reads the `+` as an operator

#### Scenario: Stress mark is dropped for an engine without support

- GIVEN the same entry and a profile declaring `Stress::None`
- WHEN the input is processed
- THEN the output contains "Москва" without the mark

### Requirement: Canonical readings are consistent across dictionaries

The curated dictionaries SHALL agree for every word they share. `IT_TERMS`,
`abbreviations::AS_WORD`, `abbreviations::SPECIAL_CASES`, and the
`code_words` table of the code identifier normalizer SHALL NOT hold two
different readings for the same lowercase key. Letter names used by curated
acronyms SHALL match `letter_name`, so a curated acronym and the spelling
fallback produce the same Cyrillic.

#### Scenario: A word present in two dictionaries reads the same

- GIVEN the words `json`, `yaml`, `ajax`, `crud`, `imap`, `oauth`, `jpeg`,
  `ios`, `iot`, `macos`, and `graphql`, which all appear in `IT_TERMS` and in
  one of the abbreviation tables
- WHEN the dictionaries are compared key by key
- THEN every pair of readings is identical

#### Scenario: Curated acronyms use the same letter names as spelling

- GIVEN the inputs "HTML", "XML", "SQL", and "GRAPHQL"
- WHEN the pipeline processes them
- THEN they read "эйч ти эм эл", "икс эм эл", "эс кью эл", and "граф кью эл" —
  the letter name "эл" that the spelling fallback also produces

#### Scenario: Code identifiers reuse the prose readings

- GIVEN the identifiers "hello", "image", and "node"
- WHEN the code identifier normalizer processes them
- THEN they read "хеллоу", "имидж", and "нод" — the same readings the prose
  path uses

#### Scenario: An acronym is not curated into an unpronounceable form

- GIVEN the input "png"
- WHEN the pipeline processes it
- THEN it reads "пи эн джи" instead of the unpronounceable "пнг", and "PNG"
  keeps the same reading instead of falling through to letter spelling

### Requirement: Engine text profile

The system SHALL describe what each engine can read in a data-only text
profile containing: the language codes the engine accepts natively; whether the
engine reads Latin prose natively (`reads_latin_natively`); the fragment route
for identifiers and fenced code blocks (see the next requirement); whether the
engine reads digits natively; whether it reads units natively; its stress
support (`None`, `Auto`, `Manual`); the codepoints it accepts (`Any` or an
explicit alphabet); its chunking limit and pause policy (see `text-chunking`);
and the markup syntax it wants around native spans (`None`, `RuEnTags`,
`LangTag`).

The profile SHALL contain no behavior — no callbacks, no trait objects — and
SHALL be reachable two ways: from an engine instance through
`TtsEngine::text_profile()`, and from the static per-engine registry keyed by
`EngineKind` that the pipeline and tests use without an engine in hand. The
registry SHALL declare a profile for every `EngineKind` variant, and the
definition SHALL live outside the application crate so that an engine crate can
share it.

Invariant: `reads_latin_natively` SHALL require `"en"` among the profile's
languages — the flag promises a native English reading, and a profile that
cannot name the language it reads has no way to annotate the fragment.

Invariant: a profile that routes an identifier or a fenced code block `Native`
SHALL list at least one language other than Russian; declaring a native reading
without a language to read it in is a specification error, not a runtime state.
The first non-Russian language is also the language a native span carries.

Invariant: `units: Expanded` SHALL require `digits: Expanded` — a unit reading
spells its number out ("пятьдесят процентов"), so the pipeline cannot expand a
unit while the engine reads the number.

Invariant: `identifier: Native` SHALL require `reads_latin_natively` — an
engine that reads identifiers natively reads Latin by definition, so a profile
may not claim the identifier route without the capability that justifies it.

A profile that violates an invariant SHALL be rejected by `TextProfile::validate`
rather than handled at runtime.

#### Scenario: Every engine kind has a profile

- GIVEN the registry
- WHEN it is asked for each `EngineKind` variant
- THEN a profile is returned for every variant, and the profiles differ only
  where the engines genuinely differ

#### Scenario: Native routing requires a non-Russian language

- GIVEN a profile with `code_block: Native` and `languages: ["ru"]`
- WHEN the profile invariants are checked
- THEN the check fails
- AND a profile with only `identifier: Native` and `languages: ["ru"]` fails
  the same way

#### Scenario: Native Latin capability requires English

- GIVEN a profile with `reads_latin_natively: true` and `languages: ["ru"]`
- WHEN the profile invariants are checked
- THEN the check fails, because the profile promises a language it does not
  list
- AND the same profile with `languages: ["ru", "en"]` is accepted

#### Scenario: Incoherent profile combinations are rejected

- GIVEN a profile with `digits: Native` and `units: Expanded`
- WHEN the profile invariants are checked
- THEN the check fails, because a unit reading needs the number in words
- AND a profile with `identifier: Native` and `reads_latin_natively: false`
  fails too

#### Scenario: Native code without native prose is valid

- GIVEN a profile with `code_block: Native`, `identifier: Narrate`,
  `reads_latin_natively: false` and a non-Russian language
- WHEN the profile invariants are checked
- THEN the profile is accepted, because code reading and Latin prose reading
  are independent capabilities

#### Scenario: Profile is data, not behavior

- GIVEN a profile value
- WHEN it is copied, serialized for a generation snapshot, or compared
- THEN no closure, engine handle or I/O is involved

### Requirement: Fragment route

The profile SHALL decide, per fragment kind, who reads the fragment:
`Narrate` turns the fragment into Russian text inside the pipeline (the
compensation a Russian-only engine needs); `Native` leaves the fragment intact
and lets the adapter read it in its own language.

The route SHALL cover two kinds: a code identifier (including identifiers
split out of identifiers, per the existing code-identifier requirement), and a
fenced code block. A Latin run in prose SHALL NOT be a profile route: the
profile declares only whether the engine can read it
(`reads_latin_natively`), and the user's English reading setting decides
whether it should (see the English reading setting requirement). Abbreviations
and URLs/emails SHALL NOT carry their own route until evidence for one exists;
the route struct SHALL grow one field per kind when that evidence arrives.

The Latin prose block is one unit of compensation for a Russian-only reader:
transliteration, the built-in term dictionary, abbreviation readings and
single-letter naming SHALL all be gated by the effective prose route together,
because a single route cannot honestly claim that an engine reads "Kubernetes"
natively but needs "API" spelled out in Cyrillic.

URLs and emails SHALL keep their structural narration regardless of the prose
route: their reading is punctuation words ("точка", "слэш"), not a
transliteration of a prose word. Reading them natively needs a route of their
own (#311).

The code-block route SHALL apply only in the read narration mode (see the
fenced code block requirement). In brief mode a `Native` block is still
announced, because the mode is the user's decision not to read code at all,
and the route only chooses the reader.

A fragment routed `Native` SHALL be annotated (see the span requirement) and
SHALL NOT be rewritten by any narrating phase. A fragment routed `Narrate`
SHALL NOT be annotated as native, and SHALL NOT be presented to the adapter as
something to read. A native identifier SHALL stay verbatim even when the
effective prose route is `Narrate`: the identifier route is the profile's
decision and the English block SHALL NOT re-read it.

#### Scenario: Identifier routed Native survives verbatim

- GIVEN a profile with the identifier route set to `Native`
- WHEN the pipeline processes "Вызови getUserData() через API"
- THEN "getUserData" stays Latin in the output, carries an identifier span with
  its language, and no Russian reading of it is emitted

#### Scenario: Prose Latin run routed Native survives verbatim

- GIVEN a profile that reads Latin natively and the effective prose route
  `Native`
- WHEN the pipeline processes "Kubernetes нужен кластер"
- THEN "Kubernetes" stays Latin and is annotated as a native prose run

#### Scenario: Native identifier survives a narrated prose route

- GIVEN a profile that reads Latin natively and routes identifiers `Native`,
  and the English reading setting `narrated`
- WHEN the pipeline processes "Kubernetes и getUserData()"
- THEN "Kubernetes" is transliterated with no native span, while "getUserData"
  stays Latin and carries an identifier span

#### Scenario: Code block routed Native survives verbatim

- GIVEN a profile with the code-block route set to `Native`, English among its
  languages, and the read mode
- WHEN the pipeline processes a text containing
  "```rust\nlet x = 42;\nif x == 0 { return; }\n```"
- THEN the output contains "let x = 42; if x == 0 { return; }" — the digit and
  the operator are not read in Russian — and a code-block span with language
  `en` covers that text

#### Scenario: Brief mode outranks a Native code route

- GIVEN a profile with the code-block route set to `Native` and the brief mode
- WHEN the pipeline processes the same text
- THEN the block is replaced with the announcement and carries no native span

#### Scenario: Narrating profile transliterates as today

- GIVEN the reference (Silero) profile
- WHEN the pipeline processes "Вызови getUserData() через API"
- THEN the output is "Вызови гет юзер дата() через эй пи ай" and no native span
  is produced

#### Scenario: Unrouted kinds keep their structural narration

- GIVEN a profile that routes identifiers and code blocks `Native`
- WHEN the pipeline processes a text containing a URL and an email
- THEN those fragments are narrated in Russian exactly as today

#### Scenario: Abbreviations follow the prose route

- GIVEN a profile that reads Latin natively and the effective prose route
  `Native`
- WHEN the pipeline processes "API нужен отдельно"
- THEN "API" stays Latin, because abbreviation reading is part of the same
  compensation as transliteration and no separate route exists yet

### Requirement: Shared, gated and adapter blocks

Every normalization rule SHALL have exactly one owner, and that owner SHALL run
at most once per fragment. The owners SHALL be:

- **Shared blocks** (run for every profile, because they decide structure or
  identify fragments rather than pronunciation): BOM removal, quote and dash
  normalization, whitespace normalization, Markdown structure, extraction of
  fenced code blocks, inline code, URLs/emails/IPs/paths, operators, special
  symbols, user-dictionary application, punctuation post-processing, and the
  final trim.
- **Gated blocks** (run only when the profile or the resolved English reading
  asks): the Latin prose block (transliteration, term dictionary,
  abbreviations, single-letter naming) whenever the effective prose route is
  `Narrate`, and the Russian narration of an identifier when the profile routes
  it `Narrate`; digit expansion — numbers, dates, times, ranges, versions,
  leading-dot decimals and tilde-approximation — for `digits: Expanded`; unit
  expansion — sizes, percentages and percentage ranges — for
  `units: Expanded`; chunking (see `text-chunking`).
- **Adapter steps** (performed by the engine implementation, never by the
  pipeline): filtering against the engine charset, collapsing newlines before a
  model tokenizer that cannot handle them, rendering markup around native
  spans, and issuing the synthesis calls. Stress rendering is the next
  consumer of this boundary and arrives with its own change (#312).

The gated number and unit blocks SHALL be skipped as a whole when the profile
declares the corresponding capability `Native`: a skipped block SHALL leave its
fragment untouched, and no other block SHALL re-process it.

#### Scenario: Gated digits block leaves a number untouched

- GIVEN a profile with `digits: Native`
- WHEN the pipeline processes "42 сервера"
- THEN the output contains "42" unchanged and the number phase did not run on it

#### Scenario: Units and digits are independent

- GIVEN a profile with `digits: Expanded` and `units: Native`
- WHEN the pipeline processes "рост 12.5% и 42 сервера"
- THEN "42" is read as "сорок два" while "12.5%" stays as it is, sign included

#### Scenario: A number is not split from its unit sign

- GIVEN a profile with `units: Native`
- WHEN the pipeline runs its number block on "рост 50%"
- THEN the number is left untouched: "50" is not read on its own, because the
  unit phase that owns the fragment did not run and the "%" would be stranded

#### Scenario: Nothing is applied twice

- GIVEN any profile and a text that reaches the transliteration block
- WHEN the pipeline processes it
- THEN applying the pipeline a second time to its output changes nothing that
  the first run produced (transliteration and number expansion are idempotent),
  and fragments already narrated are not re-read by a later phase

#### Scenario: Adapter step is not a pipeline phase

- GIVEN a profile whose engine drops codepoints outside its alphabet
- WHEN the pipeline processes a text containing such a codepoint
- THEN the pipeline does not remove it; the adapter filters it

A profile-level assertion that the output already fits the declared alphabet is
not part of this change: the Silero alphabet lives in the model bundle
(`frontend.json`, loaded only when `SILERO_NATIVE_BUNDLE` is set), so the check
belongs to the bundle-gated engine suite. `charset` is declared data that no
pipeline phase reads.

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

### Requirement: Alphanumeric abbreviations are narrated whole

The pipeline SHALL recognize a token that mixes a Latin abbreviation with
digits: an all-caps prefix of at least two letters or the literal `IPv`,
followed by digits and optional dotted groups (`IPv6`, `IPv4`,
`HTTP/2`), and SHALL read it as one Russian unit before the number
and symbol phases. Letters SHALL be read by their English names from the
shared letter-name table, digits as Russian numerals, and `/` as `слэш`.

This normalization SHALL run for every prose route, including `Native`: a
mixed-script token is not a pure Latin run, the model alphabet cannot read it,
and leaving its Latin half in place would drop the digits. A pure-letter
abbreviation (`API`, `JSON`) SHALL keep following the prose route and SHALL
stay native under a `Native` profile. The path phase SHALL NOT treat the `/`
inside such a token as a filesystem path separator.

#### Scenario: IPv6 is read in full

- GIVEN the input `IPv6`
- WHEN the pipeline processes it with either a narrating or a Native prose profile
- THEN the output reads `ай пи ви шесть` and contains no Latin letters or digits

#### Scenario: HTTP/2 is read as one token

- GIVEN the input `HTTP/2`
- WHEN the pipeline processes it
- THEN the output reads `эйч ти ти пи слэш два`, not `HTTPслэш два`

#### Scenario: Pure abbreviation follows the prose route

- GIVEN the TeraTTS profile and the input `API`
- WHEN the pipeline processes it
- THEN `API` stays Latin and carries an English span

#### Scenario: Mixed-script output is rejected by the test suite

- GIVEN any fixture containing an alphanumeric abbreviation
- WHEN it is processed with a Native prose profile
- THEN the normalized text contains no token with both Latin and Cyrillic
  letters

### Requirement: Letter-digit tokens

The pipeline SHALL recognize a token that mixes ASCII Latin letters and digits
(`p95`, `v2`, `x86`, `sha256`, `H264`, `3d`) in the English-word phase.

When the active profile routes Latin prose `Narrate`, the token SHALL be
replaced by speakable Cyrillic. A curated whole-token reading SHALL win when
one exists (`mp3` → "эм пэ три"). Otherwise the token SHALL be split into
maximal letter and digit runs, and each run SHALL be read by its existing
owner: a single letter by its letter name, a curated code word by its
`CODE_WORDS` reading, any other letter run by the English-word resolution
order, and a digit run as Russian numerals. The readings SHALL be joined with
single spaces, and the output SHALL contain no ASCII Latin letters.

When the active profile routes Latin prose `Native`, the token SHALL survive
unchanged and SHALL be annotated as a code-identifier span carrying the
profile's native language; the number phase SHALL NOT split it.

A mixed-case token (`getUserData2`) SHALL remain owned by the identifier phase
and SHALL NOT be split by this rule.

#### Scenario: Lowercase token is narrated

- GIVEN the input `p95` and the Silero profile
- WHEN the pipeline processes it
- THEN the output is `пи девяносто пять` and contains no Latin letters

#### Scenario: Curated whole-token reading wins

- GIVEN the input `mp3` and the Silero profile
- WHEN the pipeline processes it
- THEN the output is `эм пэ три`, not a split reading

#### Scenario: Code-word letter run is used

- GIVEN the input `sha256` and the Silero profile
- WHEN the pipeline processes it
- THEN the letter run reads `ша` and the digits read `двести пятьдесят шесть`

#### Scenario: Native route leaves and annotates the token

- GIVEN the input `p95 и sha256` and a profile that routes Latin prose `Native`
- WHEN the pipeline processes it
- THEN the text is unchanged and both tokens carry code-identifier spans with
  the profile's native language

#### Scenario: Mixed-case identifier is not split

- GIVEN the input `getUserData2` and the Silero profile
- WHEN the pipeline processes it
- THEN the new letter-digit rule does not split it; the identifier phase keeps
  ownership

### Requirement: English reading setting

The system SHALL persist a user preference for how Latin prose is read as
`UIConfig.english_reading`: `"auto"` (default), `"native"` or `"narrated"`.
Unknown values SHALL canonicalize to `"auto"` on load and on patch, the same
way `code_block_mode` does.

At the start of every synthesis the pipeline SHALL resolve one effective prose
route from that setting and the active profile's `reads_latin_natively`
capability:

| setting | engine reads Latin natively | effective route |
|---------|-----------------------------|-----------------|
| `auto` | yes | `Native` |
| `auto` | no | `Narrate` |
| `native` | yes | `Native` |
| `native` | no | `Narrate` (fallback) |
| `narrated` | any | `Narrate` |

`auto` SHALL reproduce the behaviour the engine had before this setting
existed, so an existing installation does not change on upgrade. Requesting
`native` on an engine without the capability SHALL fall back to narration
without failing and without rewriting the stored value. The setting SHALL be
independent of the selected engine: a choice made under one engine SHALL
survive switching to another and SHALL apply to the next synthesis without an
application restart. The setting SHALL NOT route identifiers or fenced code
blocks; those keep their profile routes.

The resolved route SHALL also render an English-context number: `Narrate`
transliterates its English reading into Cyrillic, `Native` spells it in Latin
words inside a prose-Latin span.

#### Scenario: Auto follows the engine capability

- GIVEN the TeraTTS profile and `english_reading: "auto"`
- WHEN the pipeline processes "Kubernetes нужен кластер"
- THEN the text stays Latin and carries a prose-Latin span
- AND the same setting under the Silero profile transliterates it instead

#### Scenario: Native request falls back on an incapable engine

- GIVEN the Silero profile and `english_reading: "native"`
- WHEN the pipeline processes "Kubernetes нужен кластер"
- THEN the output is the same transliterated text `auto` produces, and no
  prose-Latin span is emitted

#### Scenario: Narrated transliterates on a capable engine

- GIVEN the TeraTTS profile and `english_reading: "narrated"`
- WHEN the pipeline processes "Kubernetes нужен кластер"
- THEN "Kubernetes" is transliterated into Russian and no prose-Latin span is
  emitted, while the word timestamps map back to the source text

#### Scenario: Unknown setting value canonicalizes to auto

- GIVEN a `config.json` with `english_reading: "bilingual"` or a patch carrying
  that value
- WHEN the configuration is loaded or the patch applied
- THEN the stored value is `"auto"`

#### Scenario: Setting change reaches the next synthesis

- GIVEN a running app with `english_reading: "auto"` on a Latin-capable engine
- WHEN the setting is changed to `"narrated"` and a new synthesis starts
- THEN that synthesis transliterates English prose without an app restart

#### Scenario: Number rendering follows the route

- GIVEN the TeraTTS profile, `english_reading: "auto"` and the input "In 2024 we
  shipped 3 releases."
- WHEN the pipeline processes it
- THEN the text is "In twenty twenty-four we shipped three releases.", each
  reading carries a prose-Latin span with language `en`, and the same text under
  the Silero profile reads the numbers in transliterated English

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

### Requirement: Numeric noun case agreement

The numeric span layer SHALL decline the noun that follows a number in the case
the preposition selects: after "к"/"ко" the dative, after "о"/"об"/"при" the
prepositional, for units, currencies, percentages, scale words and fraction
tails, declining both the numeral and the noun ("к 5 кг" → "к пяти килограммам",
"о 2 кг" → "о двух килограммах", "при 10 %" → "при десяти процентах", "к $5" →
"к пяти долларам", "к 1 млн" → "к миллиону", "к 2 тыс." → "к двум тысячам"). An
indeclinable noun SHALL keep its form while the numeral declines ("к €5" → "к
пяти евро"), and a fixed unit SHALL stay fixed with the numeral declined ("к 1
°C" → "к одному градусу Цельсия").

An adjectival fraction SHALL keep a following unit in the singular genitive
("около 1/2 кг" → "около одной второй килограмма"), while a simple fraction
SHALL decline the unit with the surrounding case ("к 1,5 кг" → "к полутора
килограммам").

#### Scenario: Dative after к and ко

- GIVEN the inputs "к 5 кг", "к $5" and "к 1 млн"
- WHEN the pipeline processes them with the reference profile
- THEN both the numeral and the noun take the dative: "к пяти килограммам", "к
  пяти долларам" and "к миллиону"

#### Scenario: Prepositional after о, об and при

- GIVEN the inputs "о 2 кг", "об 1 кг" and "при 10 %"
- WHEN the pipeline processes them with the reference profile
- THEN both the numeral and the noun take the prepositional: "о двух
  килограммах", "об одном килограмме" and "при десяти процентах"

#### Scenario: Indeclinable and fixed nouns

- GIVEN the inputs "к €5" and "к 1 °C"
- WHEN the pipeline processes them with the reference profile
- THEN the numeral declines and the noun stays fixed: "к пяти евро" and "к
  одному градусу Цельсия"

#### Scenario: Fraction and unit case

- GIVEN the inputs "около 1/2 кг" and "к 1,5 кг"
- WHEN the pipeline processes them with the reference profile
- THEN an adjectival fraction keeps the singular genitive ("около одной второй
  килограмма") and a simple fraction declines the unit ("к полутора
  килограммам")

