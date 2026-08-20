# Proposal: piper-float-wav

Fixes #206 (follow-up to `opus-resample-off-list-rates`).

## Why

`opus-resample-off-list-rates` fixed the off-list *sample rate* rejection, but
a real Piper run still keeps the `.wav`: `PiperEngine` quantizes its
internally-synthesized f32 samples to 16-bit Int PCM on write
(`write_wav_i16`), and `crate::audio::encode_wav_to_opus` rejects any non-float
WAV up front with `unsupported wav format: expected 32-bit float PCM, got Int
16-bit` — before the rate check even runs. So every Piper entry is still kept
as a large `.wav` instead of transcoding to `.opus`.

## What Changes

`PiperEngine` writes its synthesized audio as a mono **32-bit-float** WAV
(`write_wav_f32`) instead of 16-bit Int PCM:

- Piper synthesizes f32 samples internally, so writing float skips a lossy i16
  quantization step (strictly better source quality for the Opus encode).
- The float WAV is exactly what `encode_wav_to_opus` accepts, so the clip
  transcodes to `.opus`; the off-list rate (e.g. 22050 Hz for `ruslan`) is
  resampled to the nearest Opus-native rate by the already-merged
  `opus-resample-off-list-rates` change.

## Non-goals

- Converting integer PCM to float inside the encoder — the encoder keeps
  rejecting int PCM; Piper is fixed at the source instead. Legacy `.wav` files
  left by older builds are not auto-migrated (they are int PCM); the user
  cleans them up manually.
- Changing Piper's output sample rate — voices still emit at their own rate;
  the Rust side resamples as before.
