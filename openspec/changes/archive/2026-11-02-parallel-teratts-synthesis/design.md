## Context

TeraTTS synthesizes one chunk per model call, and the chunker (20 s profile
limit, 240-character budget) turns a long article into dozens of chunks; the
application path additionally splits the normalized text into language segments
at every native Latin/code span, so a technical article becomes hundreds of
independent chunk calls. Today those calls run strictly one after another in
`synthesize_samples` / `synthesize_segments` / `synthesize_stream`, sharing four
`Mutex<Session>` handles.

Measurements on the reference machine (Ryzen 9 7900, 24 logical cores, ONNX
Runtime 1.27.1, release profile) on the 12 935-character article:

- one intra-op lane, 12 threads: 98–103 s for 408 segments (1040 s of audio);
- thread scaling inside one lane stops early: 4 → 55.6 s, 8 → 43.2 s, 12 →
  44.3 s, 24 → 83.4 s on the single-segment path;
- two lanes × 6 threads: 59 s; four lanes × 3 threads: 40–43 s (2.4x);
- peak RSS: 1 lane 1.3 GiB, 4 full engines (prototype) 4.6 GiB.

The graphs are chains of small ops, so intra-op parallelism saturates, while
chunks are independent — the same conclusion the Silero Native engine reached
for its own graph when it pinned eight intra-op threads.

## Goals / Non-Goals

**Goals:**

- Cut wall-clock time of the first synthesis of a long text by ~2–2.5x on a
  multi-core machine without changing the audio the engine produces.
- Keep the intra-op thread budget the same as today (half the logical cores) —
  more threads are measurably worse.
- Keep one executor for the buffered and the streamed paths so their parity
  invariant cannot drift.
- Bound memory growth: share everything that is read-only (accentizer, indexer,
  style tensors, bundle) and duplicate only the ONNX session sets.

**Non-Goals:**

- Streaming audio to the player while synthesis runs (separate issue).
- Caching chunks across regenerations.
- Changing chunk limits, per-segment options or pause policy.
- GPU execution providers, thread-count settings in the UI, or parallelizing
  across utterances/voices.

## Decisions

**Lane pool of session sets, not a shared session.** `ort::Session::run` takes
`&mut self` (the outputs borrow the session), so one session cannot serve two
threads; the crate wraps each session in a `Mutex`, which serializes everything.
The pool therefore holds `N` × `Sessions`. Rejected: one session per graph with
an unsafe unlock — the `ort` API does not expose a sound way to run it
concurrently; rejected: duplicate whole `Engine` instances (as in the
performance prototype) — each copy also re-parses the RUAccent dictionary
(~0.9 GiB), which is read-only and shareable.

**Thread budget split across lanes.** The existing `default_threads()` (half the
logical cores) becomes the total budget: `lanes = min(4, total)`,
`threads_per_lane = max(1, total / lanes)`. Rejected: four lanes × the old
twelve threads each — 48 intra-op threads oversubscribe badly (24 threads
already measured 83.4 s versus 43.2 s at 8).

**Plan, then execute, then assemble in text order.** Planning resolves segment
options, the 20-second chunk split and its duration validation once, producing
a flat `Vec<PlannedChunk>` with absolute text offsets and pause metadata.
Execution hands chunks to lanes; assembly walks the plan in order, appending
samples, inserting pauses and shifting timestamps. Rejected: parallelizing only
inside `synthesize_samples` — a single-language text is one segment, so it would
gain nothing; rejected: parallel segments *and* parallel chunks (two levels) —
two schedulers and two assembly paths for no additional throughput.

**Ordered emission through a reorder buffer.** Workers send `(index, result)`
over an `mpsc` channel; the consumer keeps a `BTreeMap` and emits the next index
as soon as it arrives, which keeps at most `lanes` audio buffers alive and lets
the streaming path stay incremental. Rejected: collect everything, then emit —
the streaming contract would silently become buffered; rejected: one channel per
chunk and `recv` in order — threads would block on send.

**Timestamps computed on the lane, shifted at assembly.** A chunk's word
timestamps depend only on its own text, audio and duration predictor, so the
worker computes them with zero-based audio and codepoint origins and the
assembler adds the chunk's audio offset and absolute codepoint start. Rejected:
compute timestamps in the consumer — that would serialize ~10–15 % of the work
(the predictor runs once per word) on the main thread; rejected: drop timestamps
on the parallel path — a behavior change for the highlighting feature.

**Parallel lane opening at load.** Four session sets are opened in parallel
threads; the bundle is verified once, as today. Rejected: lazy lane opening —
the first synthesis would pay ~15 s of extra load, exactly what this change is
trying to avoid; rejected: sequential opening — ~4x the load time.

**Cancellation is dispatch-level.** Workers check the existing `AtomicBool`
before taking the next chunk and stop; chunks already running finish and their
audio is dropped; the call returns `teratts_cancelled` and writes nothing, as
today. Rejected: aborting a running ONNX session — no API for it.

**API shape.** `Engine::load` uses the lane pool; `load_with_lanes(lanes,
threads_per_lane)` is the explicit constructor (tests, benchmark);
`load_with_threads(dir, threads)` keeps its current meaning — one lane with
`threads` intra-op threads — so the Python parity fixtures, which are sensitive
to intra-op reduction order, keep testing the same configuration.

## Risks / Trade-offs

- **Memory.** Each lane duplicates ~0.4 GiB of weights plus ONNX arenas;
  sharing the accentizer keeps the total well below the 4.6 GiB prototype, but
  4 lanes still add gigabytes on top of the current 1.3 GiB. Mitigation: the
  pool is capped by the thread budget (`min(4, total)`) and a user-visible
  setting can follow if low-RAM machines complain.
- **Load time.** Four session sets take longer to open than one; parallel
  opening keeps it close to the current single-set time, but it is still the
  slowest part of `Engine::load` and worth re-measuring.
- **Float determinism.** Audio depends on intra-op thread count, so parity
  between configurations is only guaranteed for equal per-lane thread counts;
  the invariance test pins them explicitly, and the fixtures keep running on the
  single-lane constructor.
- **Concurrency correctness.** Ordered assembly, the reorder buffer and the
  dispatch-level cancellation are the new failure surface; the tests cover
  ordering, buffered/stream parity and cancellation.
- **Overlap with other engines.** The app runs one engine at a time, so lanes do
  not contend with Piper/Silero synthesis; a future cross-engine queue would
  have to account for the extra threads.
