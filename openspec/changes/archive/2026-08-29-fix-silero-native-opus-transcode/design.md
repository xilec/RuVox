# Design: fix-silero-native-opus-transcode

## Context

`encode_wav_to_opus` (`src-tauri/src/audio/mod.rs`) gates its input on
`hound::SampleFormat::Float` + 32 bits per sample. The `silero-native` crate
writes its intermediate WAV as 16-bit int PCM — a deliberate upstream-parity
choice (`save_wav` in the reference implementation), pinned by
`silero-native/tests/parity.rs`, which reads the WAV back as `i16`. The
mismatch means every silero-native synthesis fails the transcode in
`finalize_audio_files` and keeps the `.wav` fallback, and the launch-time
`migrate_wav_audio_to_opus` sweep skips the same files (its error path just
counts and logs).

## Goals / Non-Goals

**Goals:**
- Silero-native output transcodes to `.opus` on synthesis (no `.wav` fallback).
- Existing `.wav` entries self-heal via the launch-time migration.
- Pin the transcode happy path for int16 input with tests that run everywhere
  (no model bundle required).

**Non-Goals:**
- Changing the `silero-native` crate's WAV format (upstream parity + parity
  tests depend on int16; float32 would also double the in-memory buffer).
- Lossy-format conversion, dithering, or any resampling changes.

## Decisions

- **Convert at the encoder, not the engine.** `encode_wav_to_opus` accepts
  mono 16-bit int PCM alongside mono float32 and scales samples by
  `s as f32 / 32768.0` on the fly, keeping the streaming pipeline (no extra
  buffering for native rates). Alternatives: (a) make silero-native write
  float32 — rejected: breaks the crate's upstream-parity contract and its
  parity tests for zero user-visible gain; (b) convert in
  `finalize_audio_files`/migration call sites — rejected: every future caller
  would repeat it; the encoder is the single choke point.
- **Scale by 32768, not 32767.** `i16::MIN → -1.0` maps the full range; the
  32767/32768 asymmetry on the top end is ~3e-5, far below audible/parity
  thresholds (silero-native itself encodes with `* 32767`, so the round trip
  differs by at most 1 LSB).
- **Keep one negative gate.** Formats other than float32/int16 (e.g. 8/24/32-bit
  int, stereo) still fail fast with `UnsupportedFormat` — engines are
  in-repo and known, so tolerance buys nothing.

## Risks / Trade-offs

- [Spec said "engines SHALL write float"] → The spec delta rewords the pipeline
  clause; engines still only ever write the two accepted formats.
- [Double-accepting formats weakens the gate] → Mitigated by exhaustively
  matching `(sample_format, bits_per_sample)`; anything else is rejected before
  decode, covered by negative tests.

## Migration Plan

None needed: behavior is strictly widened; existing `.wav` entries migrate on
the next launch through the unchanged migration sweep.

## Open Questions

None.
