# Delta: text-pipeline

## ADDED Requirements

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
