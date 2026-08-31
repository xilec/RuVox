## Why

Piper synthesis passes the entire normalized text to `piper-rs`'s `Piper::create` in a single
unchunked ONNX inference. The VITS text encoder is transformer-based, so activation memory grows
quadratically with input length: a 22 KB normalized text requests a single 7.6 GB tensor
(`Failed to allocate memory for requested buffer of size 7631070976`), pushing the machine into
swap thrash and a hard freeze (issue #155, two observed incidents requiring a hard reboot). The
existing 100 000-codepoint ingestion gate does not protect against this — 10 500 characters are
enough to trigger multi-GB allocations. Both sibling engines (ttsd Silero and silero-native)
already synthesize long text in bounded chunks; the official Piper implementation
(piper1-gpl) also synthesizes one audio chunk per sentence.

## What Changes

- Piper synthesis chunks the normalized text into bounded chunks (≤ `PIPER_MAX_CHUNK_CHARS`
  codepoints) before inference, preferring sentence boundaries, then clause punctuation, then any
  whitespace, with a hard split as the last resort. Per-chunk audio is concatenated into one WAV.
- Word timestamps gain a chunked variant: each chunk's words are distributed across that chunk's
  duration and shifted by the accumulated duration of preceding chunks.
- Cancellation between chunks: `kill_current` now reaches the Piper engine — the chunk loop
  checks a cancel flag before each chunk and aborts with a typed error, so a cancelled synthesis
  no longer keeps burning CPU for the whole text while holding the model mutex.
- **BREAKING** (IPC behavior): the Piper-only 100 000-codepoint input gate is removed —
  `add_text_entry` / `add_clipboard_entry` / `preview_normalize` and the synthesis-time guard no
  longer reject long input when Piper is active, matching Silero's behavior. Chunked synthesis
  makes the gate redundant; the residual risk of long input is linear (CPU time, bounded memory).
- The per-chunk limit constant is chosen by measurement (peak memory of a single `Piper::create`
  call vs chunk length), not guessed.

## Capabilities

### New Capabilities

- `piper-engine`: the in-process Piper TTS engine behavior — bounded chunking of the
  normalized text before synthesis, per-chunk inference with audio concatenation, chunked
  word-timestamp estimation, and cancellation between chunks.

### Modified Capabilities

- `ipc-commands`: the Piper-only 100 000-codepoint rejection in `add_text_entry` /
  `add_clipboard_entry`, `preview_normalize`, and the synthesis-time input length guard
  requirement are removed; long input is accepted with any active engine.
- `text-pipeline`: the "Input length limit" requirement changes from a Piper-conditional
  rejection to no length-based rejection at all (any input length is normalized in full).

## Impact

- `src-tauri/src/tts/piper/engine.rs` — chunk loop replacing the single `Piper::create` call;
  cancel flag checked between chunks; `kill_current` override.
- `src-tauri/src/tts/piper/timestamps.rs` — new `estimate_timestamps_chunked` (port of
  `ttsd/ttsd/timestamps.py::estimate_timestamps_chunked`).
- `src-tauri/src/tts/chunking.rs` (new) — standalone chunker, port of the ttsd / silero-native
  split logic with a Piper-specific limit; deliberately a separate implementation, not a
  dependency on the `silero-native` crate's internals.
- `src-tauri/src/commands/mod.rs` — `MAX_INPUT_CHARS` Piper gate and its rejection helpers removed.
- Specs: `openspec/specs/piper-engine/` (new), `openspec/specs/ipc-commands/` and
  `openspec/specs/text-pipeline/` (deltas).
- No frontend changes: `synthesize` keeps its signature and output contract (one WAV,
  word timestamps, duration).
