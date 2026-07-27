# Proposal: fix-pipeline-quadratic

## Why

Pasting a large text (e.g. raw HTML markup from a copied web page, hundreds
of KB) into the synthesis path hangs the application at 100% CPU and can
freeze the whole system (observed incident: full reboot required). Root
cause is quadratic complexity in `TrackedText`
(`src-tauri/src/pipeline/tracked_text.rs`): every replacement rebuilds the
whole string and re-clones the entire replacement history, so cost grows as
O(M·n + M²) with M replacements over an n-char document. Measured: 256 KB of
dense markup takes ~2 s, 1 MB takes ~28 s, and a saved web page of several
MB effectively never finishes — all while the global pipeline mutex is held
and the blocking task cannot be cancelled.

## What Changes

- Rework `TrackedText` replacement application so each pipeline phase costs
  O(n + M log M) instead of O(M·n + M²): apply a phase's replacements in one
  string rebuild (single `String::with_capacity` + splice in ascending
  offset order) and serve position-mapping queries from a sorted interval
  index (binary search) instead of per-query clones of the entry list.
- Add an input length limit on the text ingestion surface: `ingest_text` /
  `add_text_entry` / `preview_normalize` reject oversized input with a
  typed, user-presentable error instead of accepting unbounded text.
- Add a performance regression test (dense-markup input with a wall-time
  budget) so the quadratic cannot silently return.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `text-pipeline`: normalization MUST scale near-linearly with input size;
  oversized input MUST be rejected before normalization starts.
- `ipc-commands`: `ingest_text` / `add_text_entry` / `preview_normalize`
  MUST reject oversized input with a typed error surfaced to the UI.

## Non-goals

- Chunking text into sentences before Piper synthesis (separate change; the
  input limit is the guard against OOM in the ONNX run in the meantime).
- Cancellation checks between pipeline phases (deferred; near-linear phases
  make the uninterruptible window short).
- Frontend word-highlight scaling (`findSpanByOrigPos` per-tick
  `querySelectorAll`) — cosmetic slowdown, not a system hang.
- Timestamp span mapping (`map_via_spans`) worst case — unreachable once
  input size is bounded.

## Impact

- `src-tauri/src/pipeline/tracked_text.rs` — core rework of replacement
  application and position-map bookkeeping.
- `src-tauri/src/commands/` (text ingestion commands) — input length
  validation and typed error.
- `src/` — error toast text for oversized input (Russian UI string).
- `src-tauri/tests/` — performance regression test / golden fixture with a
  dense-markup input.
- No protocol or schema changes; existing golden fixtures must pass
  unchanged (behavior-preserving rework).
