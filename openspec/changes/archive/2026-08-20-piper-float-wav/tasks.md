# Tasks: piper-float-wav

- [x] In `src-tauri/src/tts/piper/engine.rs`, replace `write_wav_i16` with
  `write_wav_f32` (mono, 32-bit float, `SampleFormat::Float`; clamp samples to
  -1.0..1.0, no i16 quantization) and update the `synthesize` call site.
- [x] Add a `write_wav_f32_produces_float_mono_wav` unit test: written WAV is
  mono / float / 32-bit / correct rate, and samples round-trip without
  quantization.
- [x] `cargo test --lib` green; `cargo clippy --lib` and `cargo fmt --check`
  clean.
- [x] Manual verification: run the dev build, synthesize with Piper, confirm a
  `.opus` appears in the audio cache and no "keeping wav" warning is logged.
- [x] Archive the change (sync delta specs into `openspec/specs/`).
