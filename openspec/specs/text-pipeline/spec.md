# Text Pipeline Specification

## Purpose

Covers the normalization pipeline (`TTSPipeline`, implemented in Rust under
`src-tauri/src/pipeline/`) that converts technical text — Markdown, code,
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
(`src-tauri/src/pipeline/mod.rs`): BOM removal; fenced code blocks; quote
normalization; dash normalization; whitespace normalization; inline code;
Markdown structure; URLs/emails/IPs/paths; sizes; dates and times; percentage
ranges; percentages; ranges; versions; leading-dot decimals; operators; special
symbols; code identifiers; user-dictionary pre-pass; English words; numbers;
whitespace post-processing; span resolution. The order is load-bearing: URLs
MUST be consumed before the number phase so an IP is not torn into four
numbers, percentage ranges MUST be consumed before percentages so the trailing
"%" is not read separately from its range, percentages MUST be consumed before
versions so a decimal percentage is not torn apart by the version phase,
versions MUST precede bare numbers, leading-dot decimals MUST follow versions so
"1.5" keeps its version reading and MUST precede bare numbers so the fraction
digits are consumed before the number phase runs, code identifiers MUST be split
before English words, the user-dictionary pre-pass MUST run after code-identifier
splitting (so dictionary entries apply to split parts of identifiers, never to
whole identifiers) and before English-word resolution (so user entries win over
every built-in table), abbreviations MUST be resolved before transliteration,
and multi-character operators MUST be processed longest-first.

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
exact marker string "Тут мермэйд диаграмма" and MUST NOT be read aloud. All
other fenced blocks SHALL be narrated according to the pipeline's code block
narration mode, which the application sets from the `code_block_mode` config
value (`"brief"` default, `"read"` optional; the wiring lives in the
`ipc-commands` capability):

- **Brief** (`"brief"`): the block is replaced with "далее следует пример
  кода на <язык>" (language looked up in `LANGUAGE_NAMES`) or "далее следует
  блок кода" when no language tag is present.
- **Full** (`"read"`): block contents are tokenized and read aloud with
  identifiers, operators, brackets, and integer literals normalized to spoken
  Russian.

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

- GIVEN the pipeline configured in Full mode and the input
  "```python\nprint('hi')\n```"
- WHEN the pipeline processes it
- THEN the block content is read as "принт открывающая скобка хи
  закрывающая скобка"

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

The system SHALL read size and duration values (`KB`, `MB`, `GB`, `TB`, `ms`,
`sec`, `min`, `hr`, `px`, `em`, `rem`, `vh`, `vw`, and their Cyrillic
counterparts кб/мб/гб/тб) as number words plus a correctly declined unit
("мегабайт/мегабайта/мегабайт"), applying feminine forms ("одна", "две") for
feminine units. The system SHALL read semantic versions (`v2.3.1`,
`1.2.0-beta`) as number words joined by "точка", with pre-release suffixes
`alpha`, `beta`, `rc`, `dev`, `stable`, `release` read via the suffix table
("beta" → "бета") and any suffix number read as a number word.

#### Scenario: Size with declension

- GIVEN the input "Файл весит 100MB"
- WHEN the pipeline processes it
- THEN the size is read as "сто мегабайт"

#### Scenario: Duration

- GIVEN the input "таймаут 50ms"
- WHEN the pipeline processes it
- THEN the duration is read as "пятьдесят миллисекунд"

#### Scenario: Pre-release version

- GIVEN the input "1.2.0-beta"
- WHEN the pipeline processes it
- THEN the version is read as "один точка два точка ноль бета"

### Requirement: Dates and times

The system SHALL read ISO dates (`YYYY-MM-DD`) and European dates
(`DD.MM.YYYY` with a four-digit year) as "<день-ordinal> <месяц в родительном
падеже> <год-ordinal> года" (e.g. "2024-12-31" → "тридцать первое декабря две
тысячи двадцать четвёртого года"). Dates with out-of-range month or day
values SHALL be left unchanged so their digits remain available for the
number phase. The system SHALL read clock times `HH:MM` and `HH:MM:SS` with
two-digit minute components as number words; times with out-of-range
components (e.g. "25:00") SHALL be left unchanged and fall through to the
number phase. Date and time processing MUST run after URLs/paths and before
versions, ranges, and bare numbers.

#### Scenario: ISO date

- GIVEN the input "релиз 2024-12-31"
- WHEN the pipeline processes it
- THEN the date is read as "тридцать первое декабря две тысячи двадцать
  четвёртого года"

#### Scenario: Invalid time falls through to numbers

- GIVEN the input "В 25:00 встреча"
- WHEN the pipeline processes it
- THEN the time phase is a no-op and the digits are read by the number phase
  as "двадцать пять:ноль"

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

### Requirement: Operators and special symbols

The system SHALL replace the multi-character operators `===`, `!==`, `->`,
`=>`, `>=`, `<=`, `!=`, `==`, `&&`, `||` with their spoken forms ("===" →
"строго равно", "->" → "стрелка"), processing them longest-first via
`TRACKED_OPERATOR_KEYS`; a single `=` MUST NOT be replaced at this phase so
mathematical formulas stay intact. The system SHALL replace Greek letters
(`GREEK_LETTERS`), math symbols (`MATH_SYMBOLS`), and arrow symbols
(`ARROW_SYMBOLS`) with their Russian names, surrounded by spaces. A tilde
directly before a number SHALL be read as "около" ("~46" → "около 46").

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

The system SHALL read standalone integers as Russian cardinal number words
("123" → "сто двадцать три"), including thousands, millions, and billions
with correct declension and gender agreement ("тысяча" feminine). The number
phase SHALL skip digits that are adjacent to another digit or a
Latin/Cyrillic letter, or that sit next to a dot acting as a digit
separator (a decimal/version separator: a dot directly adjacent to digits
on the side facing the number), because those regions are owned by the
earlier URL, size, date, version, range, and code-identifier phases. A
number immediately before a terminal dot (a period followed by whitespace,
end of text, or a non-digit character) SHALL be read normally — sentence
punctuation is not a separator. Non-integer input and integers that fail to
parse SHALL be left unchanged. Number replacements SHALL be applied
positionally by byte range, so a match that is a substring of another match
(e.g. "1" inside "10") MUST NOT corrupt the longer number.

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

#### Scenario: Number before a sentence-ending dot

- GIVEN the input "Встреча в 5."
- WHEN the pipeline processes it
- THEN the number is read as "пять" and the output is "Встреча в пять."

#### Scenario: Dot between digits remains a separator

- GIVEN a leftover fragment like "3.14" that earlier phases did not consume
- WHEN the number phase runs
- THEN neither "3" nor "14" is expanded by the number phase

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
profile containing: the language codes the engine accepts natively; the
fragment route (see the next requirement); whether the engine reads digits
natively; whether it reads units natively; its stress support (`None`, `Auto`,
`Manual`); the codepoints it accepts (`Any` or an explicit alphabet); its
chunking limit and pause policy (see `text-chunking`); and the markup syntax it
wants around native spans (`None`, `RuEnTags`, `LangTag`).

The profile SHALL contain no behavior — no callbacks, no trait objects — and
SHALL be reachable two ways: from an engine instance through
`TtsEngine::text_profile()`, and from the static per-engine registry keyed by
`EngineKind` that the pipeline and tests use without an engine in hand. The
registry SHALL declare a profile for every `EngineKind` variant, and the
definition SHALL live outside the application crate so that an engine crate can
share it.

Invariant: a profile that routes any Latin fragment `Native` SHALL list at
least one language other than Russian; declaring native Latin reading without a
language to read it in is a specification error, not a runtime state.

Invariant: `units: Expanded` SHALL require `digits: Expanded` — a unit reading
spells its number out ("пятьдесят процентов"), so the pipeline cannot expand a
unit while the engine reads the number.

Invariant: `identifier: Native` SHALL require `latin_prose: Native` — an engine
that reads identifiers natively reads Latin by definition, and the opposite
combination would hand the same engine a Latin token in one place and its
Russian reading in another.

A profile that violates an invariant SHALL be rejected by `TextProfile::validate`
rather than handled at runtime.

#### Scenario: Every engine kind has a profile

- GIVEN the registry
- WHEN it is asked for each `EngineKind` variant
- THEN a profile is returned for every variant, and the profiles differ only
  where the engines genuinely differ

#### Scenario: Native routing requires a non-Russian language

- GIVEN a profile with `latin_prose: Native` and `languages: ["ru"]`
- WHEN the profile invariants are checked
- THEN the check fails

#### Scenario: Incoherent profile combinations are rejected

- GIVEN a profile with `digits: Native` and `units: Expanded`
- WHEN the profile invariants are checked
- THEN the check fails, because a unit reading needs the number in words
- AND a profile with `identifier: Native` and `latin_prose: Narrate` fails too

#### Scenario: Profile is data, not behavior

- GIVEN a profile value
- WHEN it is copied, serialized for a generation snapshot, or compared
- THEN no closure, engine handle or I/O is involved

### Requirement: Fragment route

The profile SHALL decide, per fragment kind, who reads the fragment:
`Narrate` turns the fragment into Russian text inside the pipeline (the
compensation a Russian-only engine needs); `Native` leaves the fragment intact
and lets the adapter read it in its own language.

This change SHALL route two kinds: a Latin run in prose and a code identifier
(including identifiers split out of identifiers, per the existing code-identifier
requirement). Abbreviations, URLs/emails and fenced code blocks SHALL NOT carry
their own route until evidence for one exists; the route struct SHALL grow one
field per kind when that evidence arrives.

The Latin prose block is one unit of compensation for a Russian-only reader:
transliteration, the built-in term dictionary, abbreviation readings and
single-letter naming SHALL all be gated by the prose route together, because a
single route cannot honestly claim that an engine reads "Kubernetes" natively
but needs "API" spelled out in Cyrillic.

URLs, emails and fenced code blocks SHALL keep their structural narration
regardless of the prose route for this change: their reading is punctuation
words ("точка", "слэш") and block markers, not a transliteration of a prose
word. Reading them natively needs a route of their own (#311).

A fragment routed `Native` SHALL be annotated (see the span requirement) and
SHALL NOT be rewritten by any narrating phase. A fragment routed `Narrate`
SHALL NOT be annotated as native, and SHALL NOT be presented to the adapter as
something to read.

#### Scenario: Identifier routed Native survives verbatim

- GIVEN a profile with the identifier route set to `Native`
- WHEN the pipeline processes "Вызови getUserData() через API"
- THEN "getUserData" stays Latin in the output, carries an identifier span with
  its language, and no Russian reading of it is emitted

#### Scenario: Prose Latin run routed Native survives verbatim

- GIVEN a profile with the prose route set to `Native` and English among its
  languages
- WHEN the pipeline processes "Kubernetes нужен кластер"
- THEN "Kubernetes" stays Latin and is annotated as a native prose run

#### Scenario: Narrating profile transliterates as today

- GIVEN the reference (Silero) profile
- WHEN the pipeline processes "Вызови getUserData() через API"
- THEN the output is "Вызови гет юзер дата() через эй пи ай" and no native span
  is produced

#### Scenario: Unrouted kinds keep their structural narration

- GIVEN a profile that routes prose and identifiers `Native`
- WHEN the pipeline processes a text containing a URL and a fenced code block
- THEN those fragments are narrated in Russian exactly as today

#### Scenario: Abbreviations follow the prose route

- GIVEN a profile that routes prose `Native`
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
- **Gated blocks** (run only when the profile asks): the Latin prose block
  (transliteration, term dictionary, abbreviations, single-letter naming) and
  the Russian narration of an identifier; digit expansion — numbers, dates,
  times, ranges, versions, leading-dot decimals and tilde-approximation — for
  `digits: Expanded`; unit expansion — sizes, percentages and percentage
  ranges — for `units: Expanded`; chunking (see `text-chunking`).
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
identifier, prose Latin run, dictionary override), its language when the
fragment is native, and its range (see `position-mapping` for the coordinate
space and the invariants).

A Latin fragment SHALL be annotated only when the active profile routes its
kind `Native`; a narrating profile SHALL produce no native span for it. The
span's language SHALL be the non-Russian language the profile declares (for
TeraTTS, `en`). A dictionary-override span SHALL carry no language and SHALL
stay Russian regardless of the prose route.

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

- GIVEN the TeraTTS profile
- WHEN the pipeline processes `Kubernetes и getUserData()`
- THEN `Kubernetes` carries a prose-Latin span with language `en` and
  `getUserData` carries a code-identifier span with language `en`

#### Scenario: Narrating profile emits no native spans

- GIVEN the Silero profile
- WHEN the pipeline processes the same text
- THEN every Latin fragment is transliterated and no span carries a language

#### Scenario: No markup leaks into the text

- GIVEN any profile and any fixture
- WHEN the pipeline processes it
- THEN the normalized text contains no tag inserted by the pipeline, and every
  span range falls inside the text

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

