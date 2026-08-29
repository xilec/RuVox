# Delta spec: storage

## MODIFIED Requirements

### Requirement: Audio File Storage

The system SHALL store synthesized audio per entry as `audio/{uuid}.opus`, where `{uuid}` is the entry's `EntryId`. The file SHALL be an Ogg-Opus stream:

| Property | Value |
|----------|-------|
| Container | Ogg |
| Codec | Opus (RFC 6716, RFC 7845) |
| Channels | 1 (mono) |
| Sample rate | One of 8 / 12 / 16 / 24 / 48 kHz — the rates libopus accepts natively (RFC 6716 §2). The TTS engine SHOULD write one of these; if it writes any other rate (e.g. a Piper voice at 22050 Hz, or 44100 Hz), the Rust side SHALL resample it to the nearest native rate before encoding. `OpusHead` SHALL record the native (resampled) rate the encoder actually used, not the original off-list rate |
| Bitrate | 32 000 bps (VOIP application) |
| Frame size | 20 ms |
| Pre-skip | Queried from `libopus`'s lookahead, scaled to 48 kHz output ticks |

The encoding pipeline is: the TTS engine (ttsd subprocess, Piper, or Silero Native) writes a mono WAV as either 32-bit-float PCM (ttsd, Piper) or 16-bit int PCM (Silero Native, matching upstream `save_wav`); the Rust side transcodes it to Opus and removes the source WAV. The transcode SHALL accept both formats, converting 16-bit int samples to float internally; any other sample format or bit width SHALL be rejected. If the WAV's sample rate is not one of the Opus-native rates, the Rust side SHALL resample it to the nearest native rate first. On encode failure the source `.wav` SHALL be left in place as a playback fallback. `save_audio` SHALL return the relative filename for `TextEntry.audio_path`.

#### Scenario: Saving audio returns the relative filename
- GIVEN an entry with id `550e8400-e29b-41d4-a716-446655440000`
- WHEN audio bytes are saved for the entry
- THEN the file `audio/550e8400-e29b-41d4-a716-446655440000.opus` exists and the returned filename is `550e8400-e29b-41d4-a716-446655440000.opus`

#### Scenario: Transcode failure keeps the WAV fallback
- GIVEN a synthesized `.wav` that fails Opus encoding
- WHEN the transcode step runs
- THEN the source `.wav` remains on disk so playback can still use it

#### Scenario: Piper clip is written as float WAV and transcodes to Opus
- GIVEN a synthesis produced by the Piper engine
- WHEN the clip is written to disk
- THEN the WAV is mono 32-bit-float PCM, so the transcode step accepts it, stores `.opus`, and removes the source `.wav`

#### Scenario: Silero Native 16-bit int WAV transcodes to Opus
- GIVEN a synthesis produced by the Silero Native engine (mono 16-bit int PCM at 24000 Hz)
- WHEN the Rust side transcodes it to Opus
- THEN the entry is stored as `.opus` (the source `.wav` is removed), `OpusHead` records 24000 Hz, and no "unsupported wav format" error is logged

#### Scenario: Off-list sample rate is resampled to the nearest native rate
- GIVEN a synthesized mono WAV at an off-list rate (e.g. 22050 Hz from a Piper voice)
- WHEN the Rust side transcodes it to Opus
- THEN the entry is stored as `.opus` (the source `.wav` is removed), `OpusHead` records 24000 Hz, and no "unsupported wav format" error is logged

#### Scenario: Native sample rate passes through without resampling
- GIVEN a synthesized mono WAV at a native rate (e.g. 48000 Hz)
- WHEN the Rust side transcodes it to Opus
- THEN the entry is stored as `.opus` and `OpusHead` records 48000 Hz, with no resampling step

#### Scenario: Other integer widths are still rejected
- GIVEN a synthesized mono WAV in a format outside the accepted set (e.g. 32-bit int PCM)
- WHEN the Rust side transcodes it to Opus
- THEN the transcode fails with an "unsupported wav format" error and the source `.wav` remains on disk
