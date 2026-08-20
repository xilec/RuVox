# Design: opus-resample-off-list-rates

## Why resample instead of widening the whitelist

The issue suggested two options: add 22050 to the accepted-rate whitelist, or
resample to 24000. **Adding 22050 was rejected**: `libopus` only accepts the
five native rates as an *encoder input rate* (`opus_encoder_create` errors on
anything else, RFC 6716 §2). Encoding happens at the resampled native rate
regardless, so widening the whitelist would just move the failure from the
pre-check into `Encoder::new`. Resampling is the only correct fix.

## Why "nearest native rate", not a fixed target

Resampling to a single fixed rate (e.g. always 48 kHz) would upscale 22050 →
48000 (more than 2×) and downscale 44100 → 48000, both farther from the source
than necessary. Choosing the *nearest* native rate keeps the rate change
minimal (22050 → 24000 is a 1.09× stretch; 44100 → 48000 is 1.09×), so any
resampling artefact is negligible at 32 kbps VOIP. `nearest_supported_rate`
picks by absolute Hz distance.

## Resampler choice

Linear interpolation was chosen over a windowed/Sinc resampler: the output is
already low-bitrate speech (32 kbps VOIP), the off-list rates are within ~9%
of a native rate, and the clips are short. A full Sinc filter would add a
dependency and complexity for inaudible gain. The linear pass is ~15 lines and
needs no new crate. Endpoints past the last input sample are held constant so
a trailing frame stays well-formed.

## Frame-writing refactor

`encode_wav_to_opus` previously computed `total_frames` from `reader.duration()`
to mark the final Ogg page `EndStream`. With resampling the output length isn't
known up front, so frame writing moved into `write_frames`, which buffers the
most recently completed frame and flushes it as `NormalPacket` when the next
arrives (or as the terminal `EndStream` page at end of input). Behavior is
identical for the native path (verified by the per-rate round-trip tests).
