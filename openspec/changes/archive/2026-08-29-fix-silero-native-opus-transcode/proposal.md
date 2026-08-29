# Proposal: fix-silero-native-opus-transcode

## Why

Entries synthesized by the silero-native engine stay as `{uuid}.wav` instead of
being transcoded to Ogg Opus: the crate writes its intermediate WAV as 16-bit
int PCM (deliberately matching upstream `save_wav`, and pinned by the parity
tests), while `encode_wav_to_opus` only accepts mono 32-bit-float WAV — so the
transcode always fails and the WAV is kept as the playback fallback. Real
history data confirms it (`audio_path` ends in `.wav`, snapshot
`audio_codec: "WAV"`). The same mismatch silently breaks the launch-time
WAV→Opus migration for those entries. (Issue #254.)

## What Changes

1. **`encode_wav_to_opus` accepts mono 16-bit int PCM** (`src-tauri/src/audio/`):
   int16 samples are converted to f32 (`s / 32768.0`) on the fly, keeping the
   streaming pipeline. All other integer widths stay rejected. Float32 input
   behavior is unchanged.
2. **Spec delta (`storage`)**: the encoding-pipeline requirement stops saying
   "Integer-PCM WAVs are rejected — engines SHALL write float"; engines now
   write either 32-bit float or 16-bit int mono WAVs, and the transcode step
   converts int16 to float internally. New regression scenario: a silero-native
   (16-bit int) WAV transcodes to `.opus`.
3. **Regression tests**: the transcode happy path is pinned for int16 input at
   the encoder level (24 kHz — silero-native's default output shape) and at the
   migration level (a legacy `.wav` entry now migrates to `.opus`).
4. No change to the `silero-native` crate: its int16 output format stays
   (upstream-parity and the parity tests depend on it).

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `storage`: the "Audio File Storage" requirement's encoding-pipeline clause —
  accept mono 16-bit int PCM in the transcode instead of rejecting all
  integer-PCM WAVs; new scenario pinning the silero-native transcode.

## Impact

- `src-tauri/src/audio/mod.rs` — format gate + int16→f32 conversion in
  `encode_wav_to_opus`; tests updated (one negative test flips to acceptance).
- `src-tauri/src/storage/` — migration test with an int16 source WAV; test util
  gains an int16 sine writer.
- No new dependencies; no frontend changes; existing `.wav` entries self-heal
  via the launch-time migration once the transcode accepts int16.
