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
