## MODIFIED Requirements

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
leave the digits in place and annotate them with a prose-Latin span (see Span
annotation output), so the English block is not split.

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

#### Scenario: English-context number on the native route

- GIVEN the TeraTTS profile and the input "In 2024 we shipped 3 releases."
- WHEN the pipeline processes it
- THEN the text stays `In 2024 we shipped 3 releases.`, each number carries a
  prose-Latin span with language `en`, and no Russian numeral appears

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
transliterates its English reading into Cyrillic, `Native` leaves the digits for
the engine inside a prose-Latin span.

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
- THEN the digits stay in the text and carry prose-Latin spans, and the same
  text under the Silero profile reads the numbers in transliterated English

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

An English-context number left to the engine on the `Native` route SHALL carry a
prose-Latin span with the profile's native language over its digits; the
`Narrate` route SHALL produce no span for it.

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
- THEN each number carries a prose-Latin span with language `en`, the spans do
  not overlap the surrounding prose spans, and the digit ranges map back to the
  source text

