## 1. Lane pool in the engine

- [x] 1.1 Add the lane-pool type in `teratts/src/synth.rs`: `Sessions` stays one
      lane; expose a constructor that opens `lanes` sets in parallel threads
      (`std::thread::scope`), returning the pool and a lane accessor.
- [x] 1.2 Add the thread-budget split helper (`min(4, total)` lanes,
      `max(1, total / lanes)` threads per lane) as a pure function with unit
      tests for 12, 4, 3 and 1 threads.
- [x] 1.3 Change `Engine` to hold the pool; `Engine::load` uses the default
      split, `Engine::load_with_lanes(lanes, threads)` is explicit, and
      `Engine::load_with_threads(dir, threads)` stays one lane with `threads`
      intra-op threads. Route `estimate` through lane 0.

## 2. Plan and ordered execution

- [x] 2.1 Introduce `PlannedChunk` (text, absolute codepoint start, language,
      per-segment `SynthOptions`, paragraph-break flag, segment index) and a
      planner that splits every segment under the 20-second policy using the
      duration predictor, carrying the existing per-segment options.
- [x] 2.2 Implement the ordered executor: workers pull plan indices from an
      `AtomicUsize`, synthesize on their own lane (chunk prepare, four sessions,
      optional word timestamps with zero-based origins) and send
      `(index, result)` over `mpsc`; the consumer emits through a `BTreeMap`
      reorder buffer and stops on the shared cancellation flag.
- [x] 2.3 Rewrite `synthesize_segments` (and `synthesize`) on top of the
      planner + executor: append samples, insert chunk and segment pauses, shift
      timestamps by the chunk's audio offset and absolute codepoint start.
- [x] 2.4 Rewrite `synthesize_stream` on the same executor: emit each chunk's
      part and the pause parts in order, keeping the existing part/summary
      contract, and keep timestamp computation off the streaming path.
- [x] 2.5 Remove the now-unused sequential helpers (`synthesize_samples`'s chunk
      loop) and keep a single model-call helper shared by the worker path.

## 3. Tests

- [x] 3.1 Unit tests for the budget split (`lane_split`) and the pause
      classification (`chunk_pause`); the plan's offsets and ordering are pinned
      by the bundle-gated parity test.
- [x] 3.2 Bundle-gated test: one lane × 1 thread versus four lanes × 1 thread
      produce identical samples, pause lengths and timestamps.
- [x] 3.3 Keep `teratts/tests/stream.rs` green on the default pool (buffered vs
      streamed parity with pauses and part offsets).
- [x] 3.4 Bundle-gated cancellation test: a cancelled multi-chunk synthesis
      returns `teratts_cancelled` and writes no WAV.

## 4. Docs, changelog and measurements

- [x] 4.1 Update `teratts/README.md` performance table with the lane numbers
      and the memory cost; note the lane pool in the crate description.
- [x] 4.2 Add a `[Unreleased]` CHANGELOG entry (user-visible TeraTTS speed-up).
- [x] 4.3 Re-run the benchmark harness on the reference article and record
      wall time, load time and peak RSS for 1 vs 4 lanes.

## 5. Validation

- [x] 5.1 `nix develop -c cargo test --manifest-path teratts/Cargo.toml`
      (bundle-gated suites included, `TERATTS_BUNDLE` + `ORT_DYLIB_PATH` set).
- [x] 5.2 `nix develop -c cargo fmt --check` and
      `nix develop -c cargo clippy --manifest-path teratts/Cargo.toml -- -D warnings`.
- [x] 5.3 `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml` (the
      app adapter and synthesis fixtures still compile and pass).
- [x] 5.4 `nix develop -c pnpm dlx @fission-ai/openspec@1.6.0 validate --specs --strict`
      (or the working `npx` equivalent) for the delta spec.
- [ ] 5.5 Archive the change and commit.
