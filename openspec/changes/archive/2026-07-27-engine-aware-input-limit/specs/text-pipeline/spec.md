# Delta: text-pipeline

## MODIFIED Requirements

### Requirement: Input length limit

The pipeline is not required to accept unbounded input: text ingestion
surfaces SHALL reject input longer than 100 000 codepoints before
normalization starts (see the `ipc-commands` capability for the rejection
surface) — but only when the active TTS engine is Piper, whose one-shot
unchunked inference the limit protects. When the active engine is Silero,
which synthesizes in bounded chunks, input of any length SHALL be accepted.
Inputs at or below the limit SHALL be normalized in full without truncation.

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
