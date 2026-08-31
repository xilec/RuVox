## 1. Shared chunker

- [ ] 1.1 Create `src-tauri/src/tts/chunking.rs`: `split_with_limit(text, limit) -> Vec<(String, usize)>` ported from the ttsd / silero-native split logic (sentence punct → clause punct → whitespace run → hard split, `split >= window/2` filter, char-codepoint offsets), plus tests ported from `silero-native/src/chunking.rs` including `assert_covers_source`, a "does not split inside a word when whitespace is available" test, and a single-token-longer-than-limit hard-split test. Verify: `cargo test --manifest-path src-tauri/Cargo.toml chunking` green.

## 2. Chunked synthesis in PiperEngine

- [ ] 2.1 Extract a testable chunk-loop helper: given text, limit, cancel flag and a `FnMut(&str) -> Result<(Vec<f32>, u32), TtsError>` synthesizer closure, return concatenated samples + sample rate, checking the cancel flag before each chunk and failing fast on chunk error. Verify: unit tests with a fake closure — chunk order and concatenation, cancel aborts with `piper_cancelled` before the next chunk, chunk error propagates and discards prior samples.
- [ ] 2.2 Rewire `PiperEngine::synthesize` to call the helper inside the existing `spawn_blocking` while holding the `Mutex<Piper>` for the whole loop; add `PIPER_MAX_CHUNK_CHARS` (initial ~600) and an `Arc<AtomicBool>` cancel flag on the engine; reset the flag at synthesize start. Verify: `cargo test --manifest-path src-tauri/Cargo.toml piper` green; short text still synthesizes as a single chunk through the same path.
- [ ] 2.3 Override `TtsEngine::kill_current` for `PiperEngine` to set the cancel flag. Verify: unit test — flag observed by an in-flight helper loop; existing supervisor/switcher cancel tests still pass.

## 3. Chunked word timestamps

- [ ] 3.1 Add `estimate_timestamps_chunked(text, chunk_durations, char_mapping)` to `src-tauri/src/tts/piper/timestamps.rs` (port of ttsd's): per-chunk proportional distribution shifted by accumulated chunk durations, full-normalized-text offsets, `map_via_spans` mapping. Verify: unit tests — monotonicity across chunk boundaries, coverage of all words, char-mapping mapping, no-word chunk still advances the audio offset.
- [ ] 3.2 Switch `PiperEngine::synthesize` to the chunked estimator, feeding `(norm_start, norm_end, duration)` per chunk. Verify: `cargo test --manifest-path src-tauri/Cargo.toml` fully green.

## 4. Remove the Piper input gate

- [ ] 4.1 Delete `MAX_INPUT_CHARS`, the rejection message helpers and the three gate call sites (ingestion, `preview_normalize`, synthesis-time re-check) from `src-tauri/src/commands/mod.rs`; update the gate's tests to pin acceptance instead of rejection. Verify: `cargo test --manifest-path src-tauri/Cargo.toml commands` green; `grep -rn "MAX_INPUT_CHARS\|100_000" src-tauri/src` finds no gate remnants.

## 5. Chunk-limit measurement

- [ ] 5.1 Add an env-gated `#[ignore]` test (pattern of `SILERO_NATIVE_BUNDLE` gating) that runs `Piper::create` on 300/600/900/1200-codepoint inputs with the real installed voice and prints peak RSS (`VmHWM` from `/proc/self/status`) and wall time per size. Verify: runs locally when `RUVOX_PIPER_LIMIT_PROBE=1` and a voice is installed; documented how to run.
- [ ] 5.2 Set `PIPER_MAX_CHUNK_CHARS` from the measurement (keep a comment citing the numbers). Verify: constant documented; if the safe limit is below 150 codepoints, stop and revisit the gate removal per design D6 risk note.

## 6. Gates, changelog, manual pass

- [ ] 6.1 Run the full gates: `nix develop -c just lint && nix develop -c just test`. Verify: both green.
- [ ] 6.2 Add a 1–2-line `[Unreleased]` CHANGELOG note (user-visible: Piper now narrates long texts without freezing; long-input gate removed). Verify: note present in the task branch.
- [ ] 6.3 Manual pass checklist for the user: synthesize a ~22 KB text (the issue's reproduction) with Piper, watch RSS stay bounded, listen to chunk boundaries for prosody breaks, and confirm cancellation stops synthesis promptly. Verify: user confirms the checklist.
- [ ] 6.4 Run `ruvox-reviewer` over the branch diff vs merge base; fold accepted findings into the branch. Verify: review reported with findings addressed or deferred as issues.
- [ ] 6.5 Archive the change with the repo-pinned CLI version: `nix develop -c pnpm dlx @fission-ai/openspec@1.6.0 archive chunk-piper-synthesis` (newer CLI rejects the scenario renames in the MODIFIED deltas; see design.md → Migration Plan). Verify: specs synced, change moved to `archive/`, pinned validate green.
