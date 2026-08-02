# Silero Native Engine Specification

## Purpose

Specifies the in-process Silero TTS v5 engine (`silero-native/` crate) running on
ONNX Runtime from a pre-exported model bundle: bundle format and verification,
text frontend parity with the upstream Python frontend, the synthesis output
contract shared with the `ttsd` engine, supported speakers and sample rates,
word timestamps, and edge-case handling.

## Requirements

### Requirement: Model Bundle Format and Verification

The engine SHALL load from a model bundle directory containing:
`tts_main.onnx`, `istft.onnx`, `pqmf.onnx`, `homosolver.onnx`,
`accentor_tensor.onnx`, the accentor dictionaries (`ngrams`, `exceptions`),
and `manifest.json`. The manifest SHALL record the upstream model version,
per-file sha256, ONNX opset, the source `.pt` hash, and the export date.
Before loading, the engine SHALL verify every file against the manifest
sha256; a mismatch MUST fail loading with a typed error, never silently
load a corrupt file.

#### Scenario: valid bundle loads

- GIVEN a bundle directory whose files match the manifest checksums
- WHEN the engine loads the bundle
- THEN all ONNX sessions initialize and the engine reports ready

#### Scenario: corrupted file is rejected

- GIVEN a bundle where `tts_main.onnx` does not match its manifest sha256
- WHEN the engine loads the bundle
- THEN loading fails with an error naming the mismatched file

### Requirement: Text Frontend Parity

The engine's text frontend SHALL reproduce the upstream v5 Russian frontend:
lowercasing, symbol filtering against the model alphabet, dash
normalization, stress placement via the ngram accentor (dictionaries +
`accentor_tensor.onnx`), homograph disambiguation via the HomoSolver BERT,
and ё handling. Explicit `+` stress markers in the input SHALL take
priority over automatic placement. SSML and `[[...]]` phonetic inserts are
NOT supported; unsupported markup MUST be stripped or rejected with a typed
error rather than mispronounced.

#### Scenario: automatic stress is applied

- GIVEN the input text "замок" without stress markers
- WHEN the frontend processes it in a homograph context ("открыть замок")
- THEN the frontend output matches the upstream Python frontend output
  symbol-for-symbol (verified by golden fixtures)

#### Scenario: explicit stress marker wins

- GIVEN the input text "з+амок"
- WHEN the frontend processes it
- THEN the explicit stress is preserved and automatic placement is not
  applied to that word

### Requirement: Synthesis Output Contract

`SileroNative::synthesize(text, speaker, sample_rate)` SHALL return the WAV
audio bytes, word-level timestamps, and total duration, using the same
output contract as the Python `ttsd` engine (`OkSynthesize`) so that
callers in `src-tauri/src/tts/` are engine-agnostic. Synthesis SHALL run
off the async runtime (blocking thread) and MUST NOT propagate a panic
across the engine boundary.

#### Scenario: output shape matches the ttsd contract

- GIVEN a loaded engine
- WHEN `synthesize` completes for a valid input
- THEN the result carries WAV bytes at the requested sample rate,
  timestamps in the `WordTimestamp` shape, and `duration_sec` consistent
  with the audio length

#### Scenario: engine panic is contained

- GIVEN an internal failure inside ONNX Runtime during synthesis
- WHEN the panic/unwind reaches the engine boundary
- THEN `synthesize` returns a typed `synthesis_failed` error and the engine
  remains usable for subsequent calls

### Requirement: Speakers and Sample Rates

The engine SHALL support the five upstream v5 speakers (`aidar`, `baya`,
`kseniya`, `xenia`, `eugene`) and the sample rates 8000, 24000, and 48000
(24000 and 8000 via the PQMF path). The engine's default sample rate SHALL
be 24000. An unknown speaker or unsupported sample rate MUST fail with a
typed `bad_input` error before synthesis starts.

#### Scenario: default sample rate

- GIVEN a config without an explicit native-engine sample rate
- WHEN the engine is used
- THEN synthesis runs at 24000 Hz

#### Scenario: unknown speaker rejected

- GIVEN a loaded engine
- WHEN `synthesize` is called with speaker "unknown"
- THEN the call fails with `bad_input` naming the speaker

### Requirement: Word Timestamps

Word-level timestamps SHALL be derived from the model's own duration
predictor output (`dur_hat`, the 4th output of `tts_main.onnx`): per-symbol
durations converted to integer frame counts exactly as the exported graph
renders them (round-half-up, sos/eos clamps already applied in-graph), at
12.5 ms per frame. Symbol ranges SHALL be aggregated into word ranges using
the word boundaries known to the frontend, and converted to seconds. Because
the frame timeline is pre-ISTFT at 48 kHz and PQMF downsampling is
time-preserving, timestamps in seconds SHALL be identical across the 8000,
24000, and 48000 output sample rates; the last word's `end` SHALL be clamped
to the actual synthesized `duration_sec`. The output shape is unchanged:
`WordTimestamp` with `start`/`end` in seconds (millisecond rounding),
monotonically non-decreasing and non-overlapping, mapped back to
original-text codepoint positions via the existing `char_mapping` mechanism.

#### Scenario: word boundaries follow model durations

- GIVEN a synthesized chunk with a `dur_hat` vector
- WHEN timestamps are computed
- THEN each word's `start`/`end` equal the cumulative per-symbol integer
  frame counts of its symbols (times 12.5 ms), not a character-proportional
  share of the chunk duration

#### Scenario: timestamps cover the spoken text

- GIVEN a synthesized entry with a char mapping
- WHEN timestamps are computed
- THEN every word of the spoken text has a timestamp with monotonically
  non-decreasing `start`/`end`, and the last `end` does not exceed
  `duration_sec`

#### Scenario: frame counts match the rendered waveform

- GIVEN a synthesized chunk
- WHEN per-symbol integer frame counts are summed
- THEN the total equals the chunk's rendered waveform length in frames
  (audio samples / 600), and the final word `end` is clamped to
  `duration_sec` when PQMF downsampling rounds the sample count

#### Scenario: timestamps are sample-rate invariant

- GIVEN the same text synthesized at 48000, 24000, and 8000 Hz
- WHEN timestamps are compared
- THEN word `start`/`end` values in seconds are equal across sample rates
  within millisecond rounding

#### Scenario: accuracy against the model reference

- GIVEN the golden parity phrase set with reference `dur_hat` captured in
  the fixtures
- WHEN word onsets derived by the engine are compared to onsets derived
  from the reference `dur_hat`
- THEN they agree within 50 ms for every phrase

#### Scenario: zero-duration words stay ordered

- GIVEN an input containing symbols the model assigns zero frames to
- WHEN timestamps are computed
- THEN such words get `start == end` at the current timeline position and
  the overall sequence remains sorted and non-overlapping

### Requirement: Edge Cases

The frontend and synthesis SHALL handle: empty input (typed `bad_input`,
no inference), single-vowel words, inputs whose predicted durations contain
zeros (durations clamped inside the exported graph before
`repeat_interleave`), and inputs longer than one chunk (the engine splits
text into ≤900-character chunks on sentence boundaries, mirroring
`ttsd/ttsd/chunking.py`, synthesizes chunk by chunk, and concatenates the
audio with monotonically shifted timestamps). Because the exported
`tts_main.onnx` bakes a 5000-frame positional table, a chunk that still
overflows it MUST be split recursively until it fits.

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
