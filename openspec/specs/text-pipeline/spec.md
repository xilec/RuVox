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
synthesis. `TTSPipeline::process_with_char_mapping(input)` is the primary
entry point and SHALL return the normalized text together with a
`CharMapping`; `TTSPipeline::process(input)` SHALL return the same normalized
text with the mapping discarded. The pipeline SHALL be shared as
`Arc<Mutex<TTSPipeline>>` in `AppState` and invoked from the
`add_text_entry` / `add_clipboard_entry` commands (normalization before
synthesis) and from `preview_normalize` (normalization preview dialog). When
normalization yields an empty string, the system SHALL reject synthesis with
the user-visible error "нормализация вернула пустой текст".

#### Scenario: Normalization before synthesis

- GIVEN the user adds the text "Вызови getUserData() через API"
- WHEN `add_text_entry` runs the normalization phase
- THEN the pipeline produces "Вызови гет юзер дата() через эй пи ай"
  (parentheses pass through verbatim; only the identifier and the English
  word are normalized) and synthesis proceeds with the normalized text

#### Scenario: Empty input

- GIVEN an empty input string
- WHEN `process_with_char_mapping` is called
- THEN the system returns an empty string and an empty `CharMapping` without
  running any phase

#### Scenario: Whitespace-only input rejected

- GIVEN input that contains only whitespace
- WHEN the pipeline processes it
- THEN the normalized result is empty and the synthesis command fails with
  "нормализация вернула пустой текст"

#### Scenario: process and process_with_char_mapping are consistent

- GIVEN any input text
- WHEN both `process` and `process_with_char_mapping` are called on it
- THEN the text returned by `process` is identical to the text returned by
  `process_with_char_mapping`

### Requirement: Fixed phase order

The system SHALL execute normalization phases in a strictly fixed order
(`src-tauri/src/pipeline/mod.rs::process_with_char_mapping`): BOM removal;
fenced code blocks; quote normalization; dash normalization; whitespace
normalization; inline code; Markdown structure; URLs/emails/IPs/paths; sizes;
dates and times; versions; ranges; percentages; operators; special symbols;
code identifiers; English words; numbers; whitespace post-processing. The
order is load-bearing: URLs MUST be consumed before the number phase so an IP
is not torn into four numbers, versions MUST precede bare numbers, code
identifiers MUST be split before English words, abbreviations MUST be
resolved before transliteration, and multi-character operators MUST be
processed longest-first.

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

#### Scenario: camelCase split before English processing

- GIVEN the input "getUserData"
- WHEN the pipeline processes it
- THEN the identifier is split by the code-identifier phase into "гет юзер
  дата" instead of being transliterated as one opaque word

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
exact marker string "Тут мермэйд диаграмма" and MUST NOT be read aloud. The
pipeline SHALL be constructed with `CodeBlockHandler` in `Full` mode: block
contents are tokenized and read aloud with identifiers, operators, brackets,
and integer literals normalized to spoken Russian. `CodeBlockHandler` also
supports a `Brief` mode in which a block is replaced with "далее следует
пример кода на <язык>" (language looked up in `LANGUAGE_NAMES`) or "далее
следует блок кода" when no language tag is present. An inline directive
`<!-- ruvox-code: full -->` or `<!-- ruvox-code: brief -->` SHALL override
the effective mode for the blocks that follow it in the same document, and
the directive itself SHALL be removed from the output.

#### Scenario: Mermaid block replaced with marker

- GIVEN the input "```mermaid\ngraph TD\nA-->B\n```"
- WHEN the pipeline processes it
- THEN the entire block is replaced with "Тут мермэйд диаграмма"

#### Scenario: Code block read in full mode

- GIVEN the pipeline in its default configuration and the input
  "```python\nprint('hi')\n```"
- WHEN the pipeline processes it
- THEN the block content is read as "принт открывающая скобка хи
  закрывающая скобка"

#### Scenario: Mode-switch directive

- GIVEN the text "<!-- ruvox-code: brief -->\n```python\nprint('hi')\n```"
- WHEN the pipeline processes it
- THEN the block is replaced with "далее следует пример кода на пайтон" and
  the directive does not appear in the output

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

#### Scenario: HTTPS URL

- GIVEN the input "https://github.com/user/repo"
- WHEN the pipeline processes it
- THEN the output contains "эйч ти ти пи эс двоеточие слэш слэш гитхаб точка
  ком слэш юзер слэш репо"

#### Scenario: Email address

- GIVEN the input "user@example.com"
- WHEN the pipeline processes it
- THEN the output contains "собака" between the local part and the domain,
  and the domain is read with "точка ком"

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
with "точка" between the integer and fractional digits.

#### Scenario: Simple range

- GIVEN the input "10-20"
- WHEN the pipeline processes it
- THEN the range is read as "от десяти до двадцати"

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
replaced first ("си плюс плюс", "си шарп", "эф шарп"). For remaining words
the resolution order SHALL be: `IT_TERMS` dictionary ("api" → "эй пи ай",
"github" → "гитхаб"); all-uppercase words of length ≥ 2 via
`AbbreviationNormalizer` (special cases like "ios" → "ай оу эс", `AS_WORD`
entries like "json" → "джейсон", otherwise letter-by-letter via
`LETTER_MAP`); `AS_WORD` dictionary for mixed-case entries; and finally
digraph-first transliteration (`sh` → "ш", `tion` → "шн", longest match
first). Custom terms registered via
`EnglishNormalizer::add_custom_terms` SHALL override `IT_TERMS`. Words
resolved by transliteration SHALL be recorded in the unknown-words map, which
SHALL be cleared at the start of every `process_with_char_mapping` call.

#### Scenario: Uppercase abbreviation spelled out

- GIVEN the input "через API"
- WHEN the pipeline processes it
- THEN the abbreviation is read letter by letter as "эй пи ай" and not
  transliterated as a word

#### Scenario: IT term from dictionary

- GIVEN the input "на github"
- WHEN the pipeline processes it
- THEN the word is read as "гитхаб"

#### Scenario: Unknown word transliterated

- GIVEN an English word absent from all dictionaries, e.g. "workflow"
- WHEN the pipeline processes it
- THEN the word is transliterated to Cyrillic via the digraph rules and
  recorded in the unknown-words map

### Requirement: Numbers

The system SHALL read standalone integers as Russian cardinal number words
("123" → "сто двадцать три"), including thousands, millions, and billions
with correct declension and gender agreement ("тысяча" feminine). The number
phase SHALL skip digits that are adjacent to a dot, another digit, or a
Latin/Cyrillic letter, because those regions are owned by the earlier URL,
size, date, version, range, and code-identifier phases. Non-integer input and
integers that fail to parse SHALL be left unchanged.

#### Scenario: Plain number

- GIVEN the input "Версия 3"
- WHEN the pipeline processes it
- THEN the number is read as "три"

#### Scenario: Number adjacent to a letter is skipped

- GIVEN a token like "v1" left over from earlier phases
- WHEN the number phase runs
- THEN the digit next to the letter is not expanded by the number phase

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
URLs/emails/IPs/paths, Markdown constructs, code blocks, mermaid, symbols,
operators, whitespace handling, and mixed paragraphs.

#### Scenario: Golden suite passes

- GIVEN the current pipeline implementation
- WHEN `cargo test --manifest-path src-tauri/Cargo.toml --test golden` runs
- THEN every fixture's pipeline output matches its `.expected.txt` and its
  `CharMapping` matches `.char_map.json`
