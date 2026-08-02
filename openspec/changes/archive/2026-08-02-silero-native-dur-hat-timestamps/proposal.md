# Proposal: silero-native-dur-hat-timestamps

## Why

The Silero Native engine currently estimates word-level timestamps heuristically — chunk audio duration is distributed proportionally to word character counts (port of `ttsd/ttsd/timestamps.py`). This drifts noticeably on numbers, pauses, and punctuation, so word highlighting in the player lags or leads the actual narration. The exported `tts_main.onnx` already produces `dur_hat` (4th output): exact per-symbol frame durations from the duration predictor (frame hop = 600 samples @ 48 kHz = 12.5 ms). v1 shipped with the heuristic to keep the port surface minimal (issue #145); this change is the follow-up.

## What Changes

- Read the `dur_hat` output of `tts_main.onnx` in the Silero Native engine (currently discarded at `silero-native/src/engine.rs`).
- Build a symbol-index → word-index mapping inside the crate's frontend layer (word boundaries are known before `build_sequence`; sos/eos and accentor markers must be accounted for).
- Replace the char-proportional estimation in `silero-native/src/timestamps.rs` with durations derived from `cumsum(dur_hat) × 0.0125 s`, aggregated to word ranges, with the final word `end` clamped to the actual synthesized `duration_sec` (PQMF downsampling rounds to whole frames).
- Keep the outward contract unchanged: same `WordTimestamp` shape (start/end in seconds, millisecond rounding, `original_pos` in original-text codepoints), same sorted/non-overlapping invariant — the app, storage, and frontend highlighting layers are untouched.
- Add unit tests for the dur_hat → word-times algorithm and a bundle-gated parity test comparing Rust-computed cumulative durations against reference `dur_hat` values captured in the golden parity fixtures.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `silero-native-engine`: the "Word Timestamps (v1)" requirement (char-proportional estimation, deferred `dur_hat` work) is replaced by dur_hat-based precise timestamps.

## Non-goals

- Piper engine: untouched (it has its own timestamp source / current behavior stays).
- Python `ttsd` engine: keeps its existing char-proportional estimation; backporting `dur_hat` to it is explicitly out of scope (decided separately per issue #145).
- No changes to the frontend highlighting, storage format, Tauri commands, or the `ttsd-protocol` / `word-highlight` / `position-mapping` specs — the timestamp contract is preserved.
- No audible-behavior verification tooling (energy-based onset detection); accuracy is validated against model-reference `dur_hat` in fixtures.

## Impact

- **Code:** `silero-native/src/engine.rs` (extract `dur_hat`), `silero-native/src/engine.rs`/`frontend/` (symbol→word mapping), `silero-native/src/timestamps.rs` (algorithm replacement), `silero-native/src/lib.rs` (plumb mapping + durations into timestamp computation), `silero-native/tests/tools/gen_parity_fixtures.py` (capture reference `dur_hat` into fixtures).
- **Specs:** delta on `openspec/specs/silero-native-engine/`.
- **APIs/contracts:** unchanged (`WordTimestamp`, `OkSynthesize`, storage JSON, Tauri commands).
- **Dependencies:** none new.
