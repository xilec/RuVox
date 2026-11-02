## Why

TeraTTS is the only engine that can read mixed Russian/English text, but it is
much slower than the default Silero Native engine on long texts: on a real
20-minute article (12 935 source characters) a release build spends ~95 s
synthesizing with TeraTTS against ~7.7 s with Silero Native. The gap is not
thread count — TeraTTS already runs ONNX Runtime with half the logical cores as
intra-op threads, and the graphs stop scaling there (4 threads 55.6 s, 8 threads
43.2 s, 12 threads 44.3 s, 24 threads 83.4 s on that article). The parallel
headroom is in the work units: every chunk is an independent model call, and the
application path produces hundreds of them (272 spans → 408 segments on that
article), all currently synthesized one after another.

## What Changes

- The engine opens several independent ONNX session sets ("lanes") at load and
  synthesizes chunks concurrently across them. Default: 4 lanes; the existing
  half-of-logical-cores budget becomes the **total** intra-op thread budget,
  split evenly across lanes.
- Chunks are planned first (splitting, duration validation, per-segment options)
  and then executed on the lanes, with results assembled strictly in text order,
  so the emitted audio, timestamps and pause placement do not depend on lane
  scheduling.
- The buffered (`synthesize`, `synthesize_segments`) and streamed
  (`synthesize_stream`) paths share one ordered executor instead of two
  sequential loops; streaming keeps emitting in order, up to `lanes` chunks
  ahead of the consumer.
- Cancellation stops dispatching new chunks; chunks already running finish and
  are discarded, and the call still returns `teratts_cancelled` with no output.
- `Engine::load_with_threads` keeps its current single-lane meaning for parity
  tests and the benchmark example; a new `load_with_lanes(lanes, threads_per_lane)`
  exposes the lane pool.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `teratts-engine`: adds a requirement for parallel chunk synthesis (lane pool,
  default lane count, ordering invariance, per-lane thread budget) and updates
  the cancellation requirement to describe dispatch across lanes.

## Impact

- `teratts/src/engine.rs` — planning/execution pipeline, lane pool, buffered and
  streaming paths, cancellation.
- `teratts/src/synth.rs` — `Sessions` stays one lane's session set; a pool owns
  the lanes.
- `teratts/examples/synth.rs` — keeps the single-lane benchmark mode and gains
  the lane count in its output.
- `teratts/tests/stream.rs`, new bundle-gated test for lane-order invariance.
- Memory: each lane holds its own copy of the four ONNX graphs (~0.4 GiB of
  weights plus arenas); the model load opens 4 session sets. The accentizer,
  indexer and style tensors stay shared, so the per-lane cost is the session set,
  not a second engine.
- `teratts/README.md` performance table and `CHANGELOG.md` (user-visible
  speed-up) are updated.

## Non-goals

- Streaming playback progress to the player while synthesizing (tracked as a
  separate issue; it touches the file-based publication path).
- Caching unchanged chunks across regenerations.
- Changing the 20-second chunk limit or the per-segment option/pause policy.
- GPU execution providers, or parallelizing across voices/bundles.
- Changing the text pipeline, the app adapter or the normalization layer.
