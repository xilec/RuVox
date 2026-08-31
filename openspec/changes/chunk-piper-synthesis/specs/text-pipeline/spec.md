## MODIFIED Requirements

### Requirement: Input length limit

The pipeline SHALL NOT impose a length-based rejection: input of any length
SHALL be accepted and normalized in full without truncation, regardless of the
active TTS engine. Inference-side memory safety is owned by the TTS engines,
which synthesize long text in bounded chunks (see the `silero-native-engine`
and `piper-engine` capabilities); normalization itself scales near-linearly
with input length.

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

#### Scenario: Oversized input is normalized when Piper is active

- GIVEN the active TTS engine is Piper and an input longer than 100 000
  codepoints
- WHEN the pipeline processes it
- THEN the whole input is normalized with no content dropped and no
  length-based rejection occurs (Piper synthesis runs in bounded chunks)
