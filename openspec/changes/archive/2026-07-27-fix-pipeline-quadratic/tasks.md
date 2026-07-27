# Tasks: fix-pipeline-quadratic

## 1. TrackedText batch apply

- [x] 1.1 Rework `TrackedText` replacement application so a phase's replacements are collected first and applied in one `String::with_capacity` + splice pass in ascending offset order, computing the phase's position-map entries during the same pass (`src-tauri/src/pipeline/tracked_text.rs`)
- [x] 1.2 Replace clone-per-query bookkeeping with a sorted interval index: keep `offset_entries` sorted by `orig_start`, merge each phase batch in one sorted insert, and serve `is_current_char_pos_inside_replacement` / `current_to_original` / `find_containing_replacement` by binary search without `.to_vec()` clones
- [x] 1.3 Run the full Rust suite incl. golden fixtures (`cargo test --manifest-path src-tauri/Cargo.toml`) — all existing fixtures and `char_map.json` comparisons must pass unchanged

## 2. Input length limit

- [x] 2.1 Add `MAX_INPUT_CHARS` (100_000 codepoints) in the ingestion layer and reject oversized input in `ingest_text` (covers `add_text_entry` and `add_clipboard_entry`) with `CommandError` type `internal` and the Russian message naming the limit, before normalization/persistence
- [x] 2.2 Apply the same rejection in `preview_normalize` before running the pipeline
- [x] 2.3 Add Rust tests: oversized input rejected for `ingest_text` and `preview_normalize` (typed error, no entry persisted); input at the limit accepted

## 3. Performance regression lock

- [x] 3.1 Add a Rust test in `src-tauri/tests/` that normalizes ~1 MB of dense replacement-heavy markup and asserts completion within 10 s wall time plus a consistent `CharMapping` (pre-fix this input takes ~30 s)
- [x] 3.2 Add a scaling check (sizes n and 2n of the same input class, time(2n) < 4x time(n)) — keep margins wide enough for loaded CI

## 4. Gates

- [x] 4.1 `just test` green (Rust incl. golden fixtures + new tests, TS, Python)
- [x] 4.2 `just lint` green (fmt, clippy -D warnings, deny, eslint, knip, tsc, ruff)
- [ ] 4.3 Manual: paste a large (>100k chars) HTML page source via Add — the app shows the limit error toast and stays responsive; a large-but-under-limit paste (e.g. ~80k chars of markup) normalizes in seconds without CPU saturation
