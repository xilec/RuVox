# Design: piper-float-wav

## Decision

Change Piper's WAV writer to emit 32-bit-float PCM (`write_wav_f32` in
`src-tauri/src/tts/piper/engine.rs`), replacing the i16 quantization.

## Rejected alternative: int→float conversion in the encoder

`encode_wav_to_opus` could accept 16-bit Int WAVs and convert samples to f32
before encoding. Rejected because:

- **Quality**: Piper synthesizes f32 internally. Writing i16 quantizes the
  signal, and converting back to float cannot recover that precision. Writing
  float directly keeps the full synthesized signal.
- **Simplicity**: the encoder keeps a single accepted input format (mono
  32-bit-float); no second decode path to maintain and test.
- **Cost is already paid**: the float WAV is transient — it is transcoded to
  `.opus` and removed on success, so the larger on-disk size never persists.

Trade-off accepted: legacy int-PCM `.wav` files written by older builds are
still rejected by the encoder and by the startup migration sweep; they are
cleaned up manually (single-user install base at this stage).
