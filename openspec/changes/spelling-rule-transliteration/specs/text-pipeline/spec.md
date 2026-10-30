# Delta: text-pipeline

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
NOT be curated as its reading ("function" reads "функшн", not "функция"). An
entry SHALL remain only when removing it changes a reading on another
resolution path: it keeps an all-caps token from letter-by-letter spelling
("TEST" reads "тест", "DATA" reads "дата", "STRING" reads "стринг"), it shadows
a wrong inflection composition ("living" reads "ливинг", not "лайвинг"), or it
is an irregular reading the rules cannot express ("kubernetes", "nginx"). When
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
win ("features" → "фичес", not "фичеес"). The "-ies" suffix
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
"жн", `-ture` → "че", `-cial`/`-tial` → "шл", `-sure` → "жер", `-gine` →
"джин", `-ule` → "уль", unstressed `-age` → "эдж", and the common
`-ive`/`-ate`/`-ade` endings); a silent final `e` that lengthens the nearest
preceding vowel ("file" → "файл", "page" → "пейдж", "type" → "тайп"); vowel
digraphs (`ai/ay` → "эй", `ea/ee/ie` → "и", `oa/ow` → "оу", `oo` → "у", `ou` →
"ау", `ue` → "ю", `ui` → "уи", `au/aw` → "о", `ew` → "ью", `oi/oy` → "ой");
soft `c`/`g` before `e`/`i`/`y`; voiced `s` between vowels and in `-sion`/
`-sure` while "system" stays unvoiced; `th` as "с" before a back vowel
("method" → "месод") and "т" otherwise ("thin" → "тин"); `ck` → "к", `qu` →
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
- THEN the output reads `юзер`, `вержн`, `функшн`, `фиче`, and `спешл`

#### Scenario: Rule readings replace redundant dictionary entries

- GIVEN the word `package` and the composition guard `feature`
- WHEN the pipeline normalizes them
- THEN the output reads `пэкэдж` and `фиче`, not the former semantic
  translations `пакет` and `фича`; `package` has no exact entry and `feature`
  keeps one only so `features` still composes

#### Scenario: Guarded all-caps tokens still use the dictionary

- GIVEN the tokens `TEST`, `DATA`, and `APP`
- WHEN the pipeline normalizes them
- THEN they read `тест`, `дата`, and `апп` from `IT_TERMS` instead of being
  spelled letter by letter
