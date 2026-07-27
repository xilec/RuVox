# Delta spec: silero-native-engine

New capability: in-process Silero TTS v5 engine running on ONNX Runtime from
a pre-exported model bundle.

## ADDED Requirements

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

### Requirement: Word Timestamps (v1)

Word-level timestamps SHALL be produced by the char-proportional estimation
algorithm (word duration proportional to its character count within the
chunk), matching the behavior of `ttsd/timestamps.py`, mapped back to
original-text positions via the existing `char_mapping` mechanism. Precise
`dur_hat`-based timestamps are deferred (issue #145).

#### Scenario: timestamps cover the spoken text

- GIVEN a synthesized entry with a char mapping
- WHEN timestamps are computed
- THEN every word of the spoken text has a timestamp with monotonically
  increasing `start`/`end` within the chunk duration

### Requirement: Edge Cases

The frontend and synthesis SHALL handle: empty input (typed `bad_input`,
no inference), single-vowel words, inputs whose predicted durations contain
zeros (durations clamped to the upstream model's minimum before
`repeat_interleave`), and inputs longer than one chunk (chunking is the
caller's responsibility; the engine processes one chunk per call).

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
