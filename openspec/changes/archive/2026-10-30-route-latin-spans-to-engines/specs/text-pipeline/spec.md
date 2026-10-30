# Delta: text-pipeline

## MODIFIED Requirements

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

## ADDED Requirements

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
