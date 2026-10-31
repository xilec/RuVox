# Delta: multi-language-synthesis

## MODIFIED Requirements

### Requirement: Native routing is per engine profile

The pipeline SHALL route a fragment to the engine only when the active
engine's text profile marks that fragment kind `Native`. The TeraTTS profile
SHALL route Latin prose, code identifiers and fenced code blocks `Native`; the
Piper profile SHALL route fenced code blocks `Native` while narrating prose
and identifiers; both Silero profiles — and the `ttsd` sidecar that shares
their model alphabet — SHALL keep narrating all three. Numbers and units SHALL
stay expanded, and URLs and emails SHALL stay narrated, for every profile
including TeraTTS.

A `Native` fragment SHALL NOT be rewritten by a narrating phase, and a
narrated fragment SHALL NOT be handed to the English frontend. A `Native` code
block SHALL reach the adapter with its internal whitespace collapsed to single
spaces and SHALL carry a code-block span with the engine's language, so the
adapter can select the language frontend without guessing from the text.

The code-block routes SHALL apply only in the read narration mode. In brief
mode every profile replaces the block with the announcement and produces no
code-block span.

#### Scenario: TeraTTS reads prose, identifiers and code natively

- GIVEN the TeraTTS profile and the read mode
- WHEN the pipeline processes `Kubernetes и getUserData()` and a fenced block
  `let x = 42; if x == 0 { return; }`
- THEN the Latin fragments and the code text stay verbatim and carry English
  spans, and neither the digit nor the operator is read in Russian

#### Scenario: Piper reads code natively but narrates prose

- GIVEN the Piper profile and the read mode
- WHEN the pipeline processes the same text
- THEN the code block stays verbatim with an English code-block span, while
  `Kubernetes` and `getUserData` are transliterated and carry no English span

#### Scenario: Russian-only engines keep narrating

- GIVEN the Silero or Silero Native profile
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
