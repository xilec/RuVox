# Delta: multi-language-synthesis

## MODIFIED Requirements

### Requirement: Native routing is per engine profile

The pipeline SHALL route a fragment to the engine only when the active
engine's text profile leaves that fragment kind to it: identifiers and fenced
code blocks when the profile marks them `Native`, a Latin run in prose when the
profile declares `reads_latin_natively` and the resolved English reading is
`Native` (see `text-pipeline`). The TeraTTS profile SHALL declare the capability
and route code identifiers and fenced code blocks `Native`; the Piper profile
SHALL route fenced code blocks `Native` while narrating identifiers, and SHALL
declare no native Latin reading, so its prose is narrated whatever the setting
asks. Both Silero profiles — and the `ttsd` sidecar that shares their model
alphabet — SHALL keep narrating every Latin fragment and declare no native
Latin reading. Numbers and units SHALL stay expanded, and URLs and emails SHALL
stay narrated, for every profile including TeraTTS.

A `Native` fragment SHALL NOT be rewritten by a narrating phase, and a
narrated fragment SHALL NOT be handed to the English frontend. A `Native` code
block SHALL reach the adapter with its internal whitespace collapsed to single
spaces and SHALL carry a code-block span with the engine's language, so the
adapter can select the language frontend without guessing from the text.

The code-block routes SHALL apply only in the read narration mode. In brief
mode every profile replaces the block with the announcement and produces no
code-block span.

#### Scenario: TeraTTS reads prose and identifiers natively

- GIVEN the TeraTTS profile and the English reading setting `auto` or `native`
- WHEN the pipeline processes `Kubernetes и getUserData()`
- THEN both Latin fragments stay verbatim and carry English spans

#### Scenario: TeraTTS narrates prose on request but keeps identifiers native

- GIVEN the TeraTTS profile and the English reading setting `narrated`
- WHEN the pipeline processes `Kubernetes и getUserData()`
- THEN `Kubernetes` is transliterated with no English span, while
  `getUserData` stays verbatim with an English identifier span

#### Scenario: TeraTTS reads a code block natively

- GIVEN the TeraTTS profile and the read mode
- WHEN the pipeline processes the fenced block
  `let x = 42; if x == 0 { return; }`
- THEN the code text stays verbatim with an English code-block span, and
  neither the digit nor the operator is read in Russian

#### Scenario: Piper reads code natively but narrates prose

- GIVEN the Piper profile and the read mode
- WHEN the pipeline processes the same text, with the English reading setting
  `native`
- THEN the code block stays verbatim with an English code-block span, while
  `Kubernetes` and `getUserData` are transliterated and carry no English span:
  the engine has no native Latin reading, so the request falls back to
  narration

#### Scenario: Russian-only engines keep narrating

- GIVEN the Silero or Silero Native profile and any English reading setting
- WHEN the pipeline processes the same text
- THEN every Latin fragment and the code block are narrated in Russian and no
  English span is produced

#### Scenario: Structural fragments stay narrated

- GIVEN the TeraTTS profile and the read mode
- WHEN the pipeline processes a URL and an email
- THEN both are narrated in Russian and carry no English span

#### Scenario: Brief mode leaves code to the pipeline

- GIVEN the TeraTTS or the Piper profile and the brief mode
- WHEN the pipeline processes a fenced code block
- THEN the block is replaced with the announcement and no code-block span is
  produced
