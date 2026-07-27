# Delta: text-pipeline

## ADDED Requirements

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

The pipeline is not required to accept unbounded input: text ingestion
surfaces SHALL reject input longer than 100 000 codepoints before
normalization starts (see the `ipc-commands` capability for the rejection
surface). Inputs at or below the limit SHALL be normalized in full without
truncation.

#### Scenario: Input at the limit is fully normalized

- GIVEN an input of exactly 100 000 codepoints
- WHEN the pipeline processes it
- THEN the whole input is normalized with no content dropped
