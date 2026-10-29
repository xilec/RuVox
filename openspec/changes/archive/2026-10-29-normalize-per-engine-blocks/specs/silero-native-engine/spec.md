# Delta: silero-native-engine

## MODIFIED Requirements

### Requirement: Edge Cases

The frontend and synthesis SHALL handle: empty input (typed `bad_input`,
no inference), single-vowel words, inputs whose predicted durations contain
zeros (durations clamped inside the exported graph before
`repeat_interleave`), and inputs longer than one chunk. Long-text splitting
SHALL use the shared splitter (see `text-chunking`) parameterized by the Silero
text profile — 900 codepoints, no inserted pauses — instead of an engine-local
copy; the engine synthesizes chunk by chunk and concatenates the audio with
monotonically shifted timestamps. Because the exported `tts_main.onnx` bakes a
5000-frame positional table, a chunk that still overflows it MUST be split
recursively until it fits. Newline collapsing before the frontend
(`sanitize_for_silero`) SHALL remain an engine-side step: it exists because the
model's char-level tokenizer does not handle control characters, not because
the pipeline decided anything about the text.

#### Scenario: empty input rejected

- GIVEN a loaded engine
- WHEN `synthesize` is called with an empty or whitespace-only string
- THEN the call fails with `bad_input` and no ONNX session runs

#### Scenario: zero durations clamped

- GIVEN an input for which the duration predictor outputs zeros for some
  symbols (e.g. punctuation-only spans)
- WHEN synthesis runs
- THEN the zero durations are clamped to the model minimum and the output
  waveform matches the upstream reference within the parity threshold

#### Scenario: long text is chunked

- GIVEN an input longer than 900 characters
- WHEN synthesis runs
- THEN the text is split on sentence boundaries, each chunk is synthesized
  separately, and the result is a single waveform with monotonically
  increasing word timestamps whose last `end` does not exceed the total
  duration

#### Scenario: multiline input does not glue words

- GIVEN an input containing newlines (e.g. multi-paragraph text)
- WHEN the frontend prepares it
- THEN newlines are replaced with spaces (mirroring ttsd's
  `sanitize_for_silero`) before symbol filtering, so adjacent words are
  never concatenated

#### Scenario: splitting is shared, not local

- GIVEN the crate's sources
- WHEN the chunk splitter is located
- THEN it comes from the shared crate parameterized by the Silero profile, and
  no local copy of the split algorithm remains
