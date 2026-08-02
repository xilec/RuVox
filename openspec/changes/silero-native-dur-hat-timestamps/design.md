# Design: silero-native-dur-hat-timestamps

## Context

See proposal.md — Why. Current state relevant to the approach:

- `silero-native/src/engine.rs` runs `tts_main.onnx` and reads only the `mag`, `x`, `y` outputs (`take_f32`); the 4th output `dur_hat` — shape `(1, L)` f32, per-symbol predicted frame durations, frame hop = 600 samples @ 48 kHz = 12.5 ms — is discarded. `L` equals the input `sequence` length (sos `|` + sentence symbols + eos `~`).
- The exported graph bakes the upstream behavior: sos/eos duration clamps (`dur[0] > 5 → 5`, same for the last symbol) mutate the duration tensor in place, and the waveform length is `repeat_interleave((dur + 0.5).long())` frames — i.e. per-symbol **integer** frame counts, round-half-up. `dur_hat` is returned after the in-place clamps (`export/export.py:148-152`, `:360-365`).
- `Engine::prepare` (`engine.rs:123-143`) produces `Vec<i64>` sequence ids + `spoken_text`; word boundaries are not tracked. The transformations between input text and sequence (symbol filtering, whitespace collapsing in `clean_star_text`, accentor `+`/ё markers, dropped unknown symbols in `build_sequence`) change string length, so the word map cannot be reconstructed from `spoken_text` after the fact.
- Timestamps today: `silero-native/src/timestamps.rs` `estimate_timestamps_chunked` — char-proportional; called from `SileroNative::synthesize` (`lib.rs:201`) with per-chunk `(text, offset, duration_sec)`.
- The outward contract (`WordTimestamp { word, start, end, original_pos }`, seconds, ms rounding, sorted/non-overlapping, `original_pos` mapped via `char_mapping` spans in `src-tauri/src/tts/silero_native/engine.rs`) must not change.

## Goals / Non-Goals

**Goals:**

- Word start/end derived from the model's own duration predictor instead of character counts.
- Word onsets accurate to ~50 ms against the model reference on the golden parity phrase set.
- Works identically at 48k / 24k / 8k output sample rates.
- No changes outside the `silero-native` crate and its test tooling.

**Non-Goals:**

- Audible-onset (energy-based) verification tooling.
- `dur_hat` backport to `ttsd`, any Piper changes (see proposal Non-goals).
- Cross-chunk pause refinement beyond what the model produces.

## Decisions

### 1. Align by letters via per-symbol char provenance, not by word lists

`build_sequence` (`frontend/text.rs:149-172`) is the last point where the correspondence `sequence[i] ↔ sentence.char_at(i-1)` (offset by sos) is still direct. It is extended to also emit, alongside the ids, the `char` each id was produced from (sos/eos included; dropped symbols simply don't appear). Timestamps are then computed by walking the original chunk text's words letter-by-letter against this symbol stream (skipping non-letter symbols such as `+`, `^`, punctuation, spaces, sos/eos; `ё` ≡ `е` because the accentor may substitute them). A word's range runs from its first matched letter's frame start to its last matched letter's frame end; punctuation pauses between words become gaps, and mid-word mismatch (e.g. digits the frontend dropped) truncates the word at its last matched letter. Words that match nothing get `start == end` at the current cursor.

*Alternative considered:* per-symbol word indices (whitespace-split model words) aggregated into word frame ranges, then aligned word-list-to-word-list. Rejected — `extract_words_with_positions` splits original text on non-alphanumerics ("раз–два" → two words) while the model sees one whitespace token, so word-level alignment needs fuzzy matching anyway; letter-level alignment is exact for exactly these cases and keeps one obvious correspondence point (`build_sequence`).

*`+` stress markers carry audio.* The bundle-gated suite showed the accentor emits `+` immediately before the stressed vowel (including single-vowel words via `stress_single_vowel`) and the model renders real frames for it (e.g. `+я`: sos 5 frames, `+` 11 frames, `я` 9 frames). A `+` run directly ahead of a word's first letter therefore opens the word's range; markers elsewhere are covered by the range via the cumulative timeline.

### 2. Word times from integer frame counts, not raw float `dur_hat`

The graph materializes audio via `repeat_interleave(floor(dur + 0.5))` — the actual waveform has `T = Σ frames_i` frames. To make word boundaries land exactly on waveform positions we compute `frames_i = trunc(dur_hat[i] + 0.5)` (dur_hat ≥ 0, sos/eos clamps already applied inside the graph) and take `cumsum(frames) × 0.0125 s` as the timeline. Word `start` = cumsum up to and including sos + preceding symbols; word `end` = cumsum through the word's last symbol.

Consistency check: `Σ frames_i` must equal the `istft` output frame count (audio length / 600); the parity test asserts this, and the last word's `end` is clamped to the actual `duration_sec` as a belt-and-braces guard (PQMF downsampling can round the sample count).

*Alternative considered:* float `cumsum(dur_hat) × 0.0125`. Rejected — cumulative rounding drift vs. the actual waveform grows with chunk length; the integer form is exactly what the model renders.

### 3. Seconds-based timeline is sample-rate invariant

`dur_hat` frames live on the 48 kHz pre-ISTFT timeline. PQMF downsampling (48k → 24k/8k) is time-preserving, so word times in seconds need no per-sample-rate conversion; only the final `end` clamp to `duration_sec` differs. No special-casing per sample rate.

### 4. `EngineOutput` carries per-symbol frame durations; `timestamps.rs` stays the contract layer

`Engine::synthesize` returns, per chunk, `Vec<SymbolDuration { ch, frames }>` — the exact integer frame count per input symbol (sos/eos included), aligned with the emitted sequence chars. `timestamps.rs` gains a `timestamps_from_durations` function that aligns the original chunk text's words to this symbol stream (decision 1) and converts frame ranges + chunk offset + clamp to `duration_sec` into the existing `WordTimestamp` shape (ms rounding, monotonicity). `SileroNative::synthesize` in `lib.rs` switches from `estimate_timestamps_chunked` to the new function; the char-proportional estimator is deleted from the crate (the Python `ttsd` keeps its own copy — separate home, unchanged).

*Alternative considered:* keep the old estimator as a fallback when `dur_hat` is missing from the ONNX outputs. Rejected — the bundle is versioned and verified by manifest; a bundle without `dur_hat` is a load-time contract violation, not a runtime fallback case. Silent fallback would mask a broken bundle.

### 5. Accuracy validated against model-reference `dur_hat` in fixtures

`tests/tools/gen_parity_fixtures.py` (reference ONNX pipeline) already runs `tts_main` and discards `dur_hat` at `:129`; it is extended to store per-phrase `dur_hat` (plus the frontend `sequence` it corresponds to) into `parity.json`. A new bundle-gated test asserts, per golden phrase: Rust-extracted `dur_hat` matches the reference (tolerance), integer frame cumsum equals the fixture waveform length, and word onsets derived from both agree within 50 ms. Pure unit tests (no bundle) cover the frame→timestamp algorithm: rounding, sos/eos offset, monotonicity, clamping, ms rounding, empty/zero-duration symbols.

## Risks / Trade-offs

- [Exported `dur_hat` output turns out to be pre-clamp or pre-rounding in some bundle rebuild] → The parity test's `Σ frames == waveform frames` assertion fails loudly at fixture-regeneration / test time, forcing a re-check of the exporter contract.
- [Word map drifts from sequence when frontend rules change (new dropped symbols, accentor changes)] → The map is built at the single point of emission (`build_sequence`), so frontend changes that alter emission break compilation or unit tests, not silently misalign.
- [Zero-duration words (dropped/inaudible symbols)] → Such words get `start == end` at the current timeline position; they remain sorted and non-overlapping, satisfying the `word-highlight` consumer invariant.
- [Punctuation-produced pauses belong to no word] → Inter-word gaps simply appear as gaps between `end` and next `start`; the highlighting code already tolerates gaps (spec `word-highlight`).
- [Per-symbol rounding slightly shifts early words vs. float durations] → Accepted: it matches the rendered waveform exactly, which is what the listener hears.

## Migration Plan

No migration. The on-disk `{uuid}.timestamps.json` format is unchanged; entries synthesized before this change keep their old (estimated) timestamps, new entries get precise ones. Rollback = revert the change; no persistent-state cleanup needed.

## Open Questions

(none)
