# Tasks: opus-resample-off-list-rates

- [x] In `src-tauri/src/audio/mod.rs`, replace the sample-rate pre-check in
  `encode_wav_to_opus` with a "nearest native rate" decision: native rates pass
  through, off-list rates are resampled via a new `resample_linear` helper to
  the rate chosen by `nearest_supported_rate`; record the resampled rate in
  `OpusHead`.
- [x] Factor frame writing into a `write_frames` helper that buffers the last
  frame to mark `EndStream` without knowing the output length up front (so it
  works for both streaming native input and buffered resampled input).
- [x] Update `audio` unit tests: drop the "22050 is rejected" case, add
  `resamples_off_list_rate_to_nearest_native` (22050 → 24000) and
  `resamples_44100_to_48000` (44100 → 48000) checks on `OpusHead` rate.
- [x] Add `migrate_22050_wav_audio_to_opus` to `storage::service` tests (legacy
  22050 Hz WAV migrates to `.opus`, source removed).
- [x] `just test` (Rust `audio` + `storage::service` suites) green.
- [x] `just lint` (clippy + rustfmt) green.
- [x] Archive the change (sync delta specs into `openspec/specs/`).
