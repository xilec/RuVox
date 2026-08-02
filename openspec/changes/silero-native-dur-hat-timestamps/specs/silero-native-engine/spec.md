# Delta spec: silero-native-engine

## RENAMED Requirements

- FROM: `### Requirement: Word Timestamps (v1)`
- TO: `### Requirement: Word Timestamps`

## MODIFIED Requirements

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
