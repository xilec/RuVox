## Context

`PiperEngine::synthesize` (`src-tauri/src/tts/piper/engine.rs`) runs the whole normalized text
through a single `piper_rs::Piper::create` call — the only API piper-rs 0.2.0 exposes
(monolithic phonemization + inference; no sentence-level or streaming surface, and the project
is nearly dormant: the latest release 0.2.0 from 2026-05 is also the tip of `main`). VITS
encoder activation memory grows quadratically, so long texts freeze machines (proposal.md —
Why). Both sibling engines already chunk: ttsd (`ttsd/ttsd/chunking.py`) and silero-native
(`silero-native/src/chunking.rs`), and the official piper1-gpl synthesizes one audio chunk per
sentence. Piper's `timestamps.rs` already anticipated a chunked variant in its module docs.
The `Mutex<Piper>` is currently taken for the single `create` call inside one `spawn_blocking`;
`TtsEngine::kill_current` is a no-op for Piper, so a "cancelled" inference keeps running in the
background holding the model lock until the whole text finishes.

## Goals / Non-Goals

**Goals:**
- Bound Piper inference memory to a per-chunk constant regardless of input length.
- Keep the `TtsEngine::synthesize` signature, output contract (one WAV + word timestamps +
  duration) and frontend behavior unchanged.
- Make cancellation actually stop Piper work (between chunks) instead of leaving a zombie
  inference holding the model mutex.
- Remove the now-redundant Piper-only 100k input gate (spec deltas for `ipc-commands`,
  `text-pipeline`).

**Non-Goals:**
- Streaming/progressive playback of chunk audio (chunks concatenate into one WAV; progress
  events are a separate future change — the frontend has no synthesis-progress listener today).
- Changes to the ttsd or silero-native engines; refactoring their chunkers into a shared crate.
- Upstreaming a sentence-level API to piper-rs.
- Re-measuring or tuning the normalization pipeline (already near-linear since
  fix-pipeline-quadratic).

## Decisions

### D1: Standalone chunker in `src-tauri/src/tts/chunking.rs` (not reusing `silero_native::chunking`)

Port the ttsd split logic (sentence punct → clause punct → whitespace run → hard split at the
limit, `split >= window/2` filter) into a new `tts`-level module. `src-tauri/src/tts/` is
already the shared-helper layer for engines (`map_via_spans`, `CharMappingEntry` live there).

- Alternative considered: make `split_with_limit` pub in silero-native and call it. Rejected:
  silero-native is a standalone engine crate whose chunking module is documented in Silero
  terms (`MAX_CHUNK_SIZE` "Silero limit", `sanitize_for_silero`); pointing the Piper engine at
  a sibling engine crate's internals is hidden coupling — a little duplication beats it
  (code-quality.md → Duplication). A shared mini-crate is over-engineering at two Rust
  consumers; revisit if a third appears.

### D2: Chunk limit chosen by measurement, starting point ~600 codepoints

`PIPER_MAX_CHUNK_CHARS` in `src-tauri/src/tts/piper/engine.rs`. Memory is not the binding
constraint at these sizes (issue data: 7.6 GB tensor at 22 KB input ⇒ ~1–3 MB tensors at a few
hundred chars); the real constraints are VITS attention stability/prosody on long chunks and
chunk latency. A small env-gated `#[ignore]` measurement test (pattern: `SILERO_NATIVE_BUNDLE`
gating) runs `create` on 300/600/900/1200-char inputs with the real voice and prints peak RSS
(`VmHWM` from `/proc/self/status`); the constant is set from its results. Not run in CI (needs
the downloaded voice).

### D3: Chunk loop inside one `spawn_blocking`, holding the model mutex for the whole loop

The existing `piper.lock()` guard extends over the loop instead of a single call. Extract the
loop into a testable helper taking a `FnMut(&str) -> Result<(Vec<f32>, u32), TtsError>`
synthesizer closure, so chunking/timestamps/cancel logic is unit-testable without a real Piper
model. Samples accumulate in a `Vec<f32>` and the WAV is written once after the loop, so a
failed or cancelled synthesis never leaves a partial audio file. `sample_rate` comes from the
first chunk (constant per voice).

### D4: Cancellation via `Arc<AtomicBool>` checked between chunks

`PiperEngine` holds a cancel flag; `kill_current` (overridden, replacing the default no-op)
sets it; the chunk loop checks it before each chunk and aborts with a typed
`piper_cancelled` error; the flag resets at the start of each `synthesize`. This plugs into the
existing supervisor → switcher → `kill_current` wiring unchanged. Worst-case races (one extra
chunk after a cancel, a stale flag caught by the next synthesize's reset) are benign and
documented at the flag. Alternative: tokio `CancellationToken` — rejected as unnecessary here;
the check points are synchronous and single-consumer.

### D5: Chunked timestamps — direct port of `estimate_timestamps_chunked` from ttsd

New `estimate_timestamps_chunked(text, chunk_durations, char_mapping)` in
`src-tauri/src/tts/piper/timestamps.rs`, where `chunk_durations: Vec<(usize, usize, f64)>` is
`(norm_start, norm_end, duration_sec)` per chunk. Words inside a chunk are distributed
proportionally to their codepoint length; each chunk's contribution is shifted by the
accumulated duration of preceding chunks. `original_pos` semantics and `map_via_spans` usage
are unchanged from the single-chunk variant.

### D6: Remove the Piper input gate entirely

Delete `MAX_INPUT_CHARS`, the rejection helpers and the three call sites in
`src-tauri/src/commands/mod.rs` (ingestion, preview, synthesis-time re-check). After chunking,
residual long-input risk is linear (CPU time; bounded memory; sample buffers ~0.5 GB f32 at
100k chars — the same Silero already accepts unguarded). The obsolete gate comment is removed
with the code.

## Risks / Trade-offs

- [Prosody discontinuity at chunk boundaries — each chunk synthesizes with fresh noise/latents]
  → the splitter prefers sentence boundaries, where a pause is natural; this is exactly how
  official Piper and both Silero engines behave. Manual pass includes listening to chunk
  boundaries on a real voice.
- [Hard split can cut inside a word] → only reachable when a limit-sized window has no
  whitespace at all (a single token longer than the limit, e.g. a long URL); pinned by a unit
  test, and unchanged from the behavior the Silero engines already ship.
- [Cancellation races] → a chunk in flight when `kill_current` fires finishes; the loop then
  stops before the next chunk. A stale flag from a previous cancel is cleared at the start of
  the next `synthesize`. Worst case is one extra chunk of wasted work, never corruption.
- [Measured safe chunk limit turns out very small (<150)] → would mean revisiting the gate
  removal in a follow-up; unlikely, since tensor sizes scale with the square of chunk length
  and the 600-char start point is already two orders of magnitude below the freeze threshold.

## Migration Plan

No data migration. The gate removal changes IPC behavior in the same release as the chunking
that justifies it — specs (`ipc-commands`, `text-pipeline`) sync on archive. Rollback is a
plain revert of the task branch.

Tooling note: validate and archive for this change MUST use the repo-pinned CLI
(`pnpm dlx @fission-ai/openspec@1.6.0`, per conventions.md → Testing gates). Newer CLI
versions (≥1.7) reject MODIFIED deltas that rename scenarios, which this change does
legitimately — the gate scenarios ("oversized text is rejected…") become acceptance scenarios,
the same rename pattern the archived engine-aware-input-limit change shipped.

## Open Questions

None — the chunk limit is settled by a measurement task inside this change, not by a deferred
decision.
