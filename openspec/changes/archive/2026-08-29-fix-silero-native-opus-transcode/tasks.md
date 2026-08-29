# Tasks: fix-silero-native-opus-transcode

## 1. Encoder

- [x] 1.1 `src-tauri/src/audio/mod.rs`: `encode_wav_to_opus` accepts mono
      16-bit int PCM — exhaustive match on `(sample_format, bits_per_sample)`
      selecting the hound sample reader; int16 samples map to f32 via
      `s as f32 / 32768.0` on the fly (streaming preserved for native rates,
      buffered only for the resample path). Anything else keeps failing with
      `UnsupportedFormat`.

## 2. Encoder tests (`src-tauri/src/audio/mod.rs`)

- [x] 2.1 Flip `rejects_non_float_sample_format` into an acceptance
      regression for the silero-native output shape: mono 16-bit int PCM at
      24000 Hz encodes successfully, produces a >1000-byte Ogg stream whose
      `OpusHead` records 24000 Hz.
- [x] 2.2 Keep the negative gate: a new test pins that 32-bit int PCM is
      still rejected with `UnsupportedFormat` (stereo rejection already
      covered).

## 3. Migration

- [x] 3.1 `src-tauri/src/storage/test_util.rs`: add an int16 sine-WAV writer
      beside `write_sine_wav`.
- [x] 3.2 `src-tauri/src/storage/service.rs` tests: a legacy entry whose
      `.wav` is 16-bit int PCM migrates to `.opus` in
      `migrate_wav_audio_to_opus` (regression for the real-world `.wav`
      entries reported in #254).

## 4. Bundle-gated end-to-end regression

- [x] 4.1 `src-tauri/tests/` (new gated integration test, same
      skip-without-bundle contract as `silero-native/tests/common`): load the
      real bundle, synthesize a short phrase via `SileroNative`, run the
      result through `replace_wav_with_opus`, and assert the `.opus` output
      exists with a sane size and `OpusHead` rate 24000. Runs only where the
      bundle is present (`SILERO_NATIVE_BUNDLE` or `<repo>/tmp/bundle-v5`).

## 5. Gates

- [x] 5.1 `just test` (Rust + TS + Python) and `just lint` green.
- [x] 5.2 `CHANGELOG.md`: 1-2-line `[Unreleased]` note (silero-native audio
      now stored/kept as Opus; existing `.wav` entries migrate on launch).
