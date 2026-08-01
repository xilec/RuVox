# Design: fix-pipeline-quadratic

## Context

`TrackedText` (`src-tauri/src/pipeline/tracked_text.rs`) is the mutable text
+ position-map core every normalization phase uses. Its `sub` /
`replace_byte_range` pair was designed for short chat-style strings: each
replacement rebuilds the whole string (`format!("{}{}{}")`, O(n)) and
re-clones/rescans the entire replacement history (`.to_vec()` per query,
`insert(0, …)` with a full vector shift, O(M)). Across ~30 phases the cost is
O(M·n + M²): measured 137 ms at 64 KB of dense markup, ~2 s at 256 KB,
~28 s at 1 MB — while the global `Arc<Mutex<TTSPipeline>>` is held inside an
uncancellable `spawn_blocking`, so a large paste wedges every entry behind it
at 100% CPU (observed incident: full system freeze, reboot required).

There is also no length limit anywhere on the ingestion path
(`ingest_text`, `preview_normalize`), and Piper phonemizes/inferrs the whole
normalized text in one ONNX run, so unbounded input also risks an OOM in the
engine.

## Goals / Non-Goals

**Goals:**

- Each pipeline phase runs in O(n + M log M): one string rebuild per phase,
  position-map queries served without cloning the entry list.
- Behavior-preserving: all existing golden fixtures and unit tests pass
  unchanged; `TrackedText`'s public API and the phase code stay as-is.
- Oversized input is rejected at the ingestion surface with a clear
  user-visible (Russian) error before normalization starts.
- A regression test fails if the quadratic returns.

**Non-Goals:**

- Sentence-level chunking before Piper + mid-phase cancellation (separate
  change; the input limit bounds the ONNX memory blow-up until then).
- Frontend highlight scaling, `map_via_spans` worst case (unreachable once
  input is bounded).

## Decisions

### 1. One string rebuild per phase, not per replacement

`TrackedText::sub` (and internal callers) collects the phase's replacements
first, then applies them in ascending offset order with a single
`String::with_capacity` + splice pass. Position-map bookkeeping for the whole
batch is computed during that same pass.

Alternatives rejected:
- **Rope / piece-table (e.g. `ropey`)** — new dependency and a rewrite of the
  position mapping; the batch rebuild already reaches the target complexity.
- **Keeping per-replacement apply but optimizing constants** — leaves O(M²)
  history bookkeeping in place; the incident class survives.

### 2. Sorted interval index instead of clone-per-query bookkeeping

`offset_entries` remains the single source of truth, kept sorted by
`orig_start`; containment/lookup queries
(`is_current_char_pos_inside_replacement`, `current_to_original`,
`find_containing_replacement`) binary-search it by reference instead of
cloning via `get_sorted_entries().to_vec()` on every call. The per-phase
batch (decision 1) inserts its entries in one sorted merge — no
`insert(0, …)` shifts, no per-query re-sort.

### 3. Hard rejection over silent truncation for oversized input

`ingest_text` (covers `add_text_entry` and the tray's `add_clipboard_entry`)
and `preview_normalize` reject input longer than `MAX_INPUT_CHARS`
(100_000 codepoints) with the existing `internal` `CommandError` type and a
Russian message naming the limit, e.g. "текст слишком длинный (максимум
100 000 символов)". 100k codepoints covers long articles/documentation pages
while bounding the unchunked Piper run to a survivable memory footprint.

Alternatives rejected:
- **Truncation** — silently drops user content; the user must decide what to
  do with an over-limit text.
- **New `CommandError` variant `input_too_large`** — nicer typing, but the
  frontend renders all command errors through the same toast; `internal`
  with a clear message is consistent with the existing blank-input rejection
  and keeps the IPC contract unchanged.
- **No limit, rely on the perf fix alone** — near-linear normalization still
  feeds an unchunked ONNX run; the limit is the guard until chunking lands.

### 4. Regression lock: large-input wall-time test with a wide margin

A Rust test normalizes ~1 MB of dense, replacement-heavy markup and asserts
completion within a generous wall-time budget (10 s). Pre-fix this input
takes ~30 s (fails); post-fix it takes well under 1 s — a >10x margin on
both sides keeps the test non-flaky on slow CI while still catching a
quadratic regression. A pure complexity test (no timing) is not practical
without instrumenting allocation counts; the wide-margin timing assert is
the pragmatic lock, placed in `src-tauri/tests/` next to the golden suite.
Correctness of the large-input output itself stays pinned by the existing
golden fixtures (behavior-preserving rework).

## Risks / Trade-offs

- [Batch apply subtly changes position mapping for overlapping/adjacent
  replacements] → The rework is gated by the full golden suite incl.
  `char_map.json` comparisons; number-phase substring scenarios ("1" inside
  "10") already have fixtures.
- [Wall-time test flakes on a loaded CI machine] → >10x margin on both sides
  of the budget; release-mode-like workload kept at 1 MB.
- [100k limit rejects a legitimate use case later] → The constant lives in
  one place (`commands` ingestion layer) with the spec naming it; raising it
  after Piper chunking lands is a one-line change plus spec edit.
