# Proposal: opus-resample-off-list-rates

Fixes #206 (targeted at v0.3.0).

## Why

`crate::audio::encode_wav_to_opus` only accepts the five Opus-native sample
rates (8 / 12 / 16 / 24 / 48 kHz, RFC 6716 §2) and rejects anything else with
`unsupported wav format: expected sample rate in [...] Hz, got 22050`. Piper
voices emit at their own rate — `ruslan` outputs 22050 Hz — so every Piper
entry hits the rejection path, the WAV-to-Opus transcode is skipped, and the
entry keeps the much larger `.wav` instead of an `.opus`. The startup
migration sweep (`storage::service::migrate_wav_audio_to_opus`) logs the same
error for legacy 22050 Hz WAVs and leaves them as `.wav` forever.

## What Changes

Resample any off-list WAV rate to the **nearest** Opus-native rate before
encoding, instead of rejecting it:

- Native rates (8/12/16/24/48 kHz) pass through untouched — Silero `ttsd`'s
  48 kHz path stays streaming and unchanged.
- Off-list rates are linear-resampled to the closest native rate: 22050 →
  24000, 44100 → 48000, 32000 → 24000, 11025 → 12000, etc.
- `OpusHead` records the rate the encoder actually used (the native,
  resampled rate), never the original off-list rate.

The resampler is a small linear-interpolation pass (one buffer, only needed
for off-list inputs — Piper clips are seconds long, so buffering is cheap).
The native path still streams frame-by-frame from the WAV reader, so memory
stays constant for the common case.

## Non-goals

- Changing Piper's own output rate, or forcing Piper to emit a native rate —
  that is the TTS engine's choice; we adapt on the Rust side.
- Picking an arbitrary fixed target rate (e.g. always 48 kHz) — we resample to
  the *nearest* native rate to minimize rate change and keep 44.1 kHz content
  at 48 kHz rather than downscaling it.
- Downmixing stereo or converting integer PCM — those inputs are still
  rejected up front; only the off-list *sample rate* is now handled.
- Touching `UIConfig.sample_rate` semantics — it still selects the Silero
  output rate; Piper voices ignore it and we resample whatever they produce.
