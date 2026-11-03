## 1. Dependency swap

- [x] 1.1 Replace `opus = "0.3"` with `opus1 = "0.3"` in
      `src-tauri/Cargo.toml`, update the comment above it (why pure Rust, the
      #19 measurements), and refresh `Cargo.lock`.
- [x] 1.2 Remove `libopus` from `nix/devshell.nix` (build inputs and the
      pkg-config note) and from `flake.nix` (build inputs, runtime library
      path and any comment that names it).
- [x] 1.3 Drop `libopus-dev` from the non-Nix dependency list in
      `docs/install.md`.
- [x] 1.4 Give `opus1` its own `opt-level = 3` in the workspace release
      profile: the size-optimized build costs the pure-Rust codec ~40 % on
      the reference sample, while the C `libopus` it replaces was built at
      `-O2` by Nix.

## 2. Audio module

- [x] 2.1 In `src-tauri/src/audio/mod.rs`, switch imports and the error
      variant to `opus1`, construct the encoder with
      `OpusEncoder::new(SampleRate, Channels::Mono, Application::Voip)` and set
      the bitrate with `Bitrate::BitsPerSecond`.
- [x] 2.2 Replace `Encoder::get_lookahead()` with `OpusEncoder::lookahead()`,
      keeping the existing 48 kHz scaling; the 48 kHz value stays 312.
- [x] 2.3 Switch the decoder to `OpusDecoder::decode`, keeping the existing
      pre-skip/granule trimming and WAV writing unchanged.
- [x] 2.4 Keep the frame loop, `OpusHead`/`OpusTags` builders and the atomic
      `.part` publish untouched; only the encode call changes.

## 3. Validation

- [x] 3.1 `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml
      audio::` — encoder/decoder round-trip, rate matrix, off-list resampling.
- [x] 3.2 `cargo fmt --check` and `cargo clippy -D warnings` are clean (no
      frontend code touched, so the TS half of `just lint` does not apply).
- [x] 3.3 Encode the 22:49 reference WAV with the new path and check wall time,
      output size and `OpusHead` against the libopus baseline; decode it back
      and confirm the exported WAV matches the old decoder within Opus noise.
- [x] 3.4 Manual pass: save a long narration in the app, play it, and export
      the audio to WAV; confirm playback, seeking and the exported file.

## 4. Bookkeeping

- [x] 4.1 Add the `[Unreleased]` CHANGELOG entry (faster saving, no system
      `libopus`).
- [x] 4.2 Archive the change and sync the storage spec.
