# Tasks: silero-native-dur-hat-timestamps

## 1. Symbol char provenance in the frontend

- [x] 1.1 Change `build_sequence` (`silero-native/src/frontend/text.rs`) to also emit the `char` each sequence id was produced from (sos/eos included; dropped symbols simply do not appear); propagate through `Engine::prepare` (`silero-native/src/engine.rs`)
- [x] 1.2 Unit tests for the provenance: multi-word sentence, dropped unknown symbols (`^`), sos/eos chars, alignment `chars.len() == ids.len()` (no bundle needed)

## 2. Extract dur_hat and render word frames

- [x] 2.1 Read the `dur_hat` output in `Engine::synthesize` (`silero-native/src/engine.rs`, next to the existing `take_f32` calls); fail with a typed error if the output is missing
- [x] 2.2 Convert `dur_hat` to integer per-symbol frame counts (`trunc(dur + 0.5)`, clamps already in-graph), zip with the emitted chars into `Vec<SymbolDuration>`, and return them in `EngineOutput`; validate `dur_hat.len() == sequence.len()`

## 3. Timestamp computation

- [x] 3.1 Add `timestamps_from_durations` to `silero-native/src/timestamps.rs`: letter-level alignment of the chunk text's words to the symbol stream (skip non-letters, `ё` ≡ `е`, partial match on mid-word mismatch, zero-length fallback for unmatched words) → `WordTimestamp` (frames × 0.0125 s + chunk offset, ms rounding, monotonic non-overlapping, `end` clamped to chunk `duration_sec`)
- [x] 3.2 Unit tests: rounding, sos offset, monotonicity, clamping, zero-frame words (`start == end`), ms rounding
- [x] 3.3 Switch `SileroNative::synthesize` (`silero-native/src/lib.rs`) from `estimate_timestamps_chunked` to the new path; delete the char-proportional estimator and its now-dead tests from the crate

## 4. Fixtures and parity test

- [x] 4.1 Extend `silero-native/tests/tools/gen_parity_fixtures.py` to capture reference `dur_hat` (and the sequence it aligns to) per phrase into `parity.json`; regenerate fixtures via `tests/tools/regenerate_fixtures.sh`
- [x] 4.2 Add a bundle-gated test (`silero-native/tests/`, next to `parity.rs`): Rust `dur_hat` matches the reference within tolerance; `Σ frames == waveform length / 600`; word onsets from Rust vs. reference agree within 50 ms
- [x] 4.3 Verify the sample-rate invariance scenario: same phrase at 48000/24000/8000 Hz yields equal word times within ms rounding (bundle-gated)

## 5. Validation

- [x] 5.1 `nix develop -c cargo test --manifest-path silero-native/Cargo.toml` green (unit tier; bundle-gated tier with `SILERO_NATIVE_BUNDLE` set if a bundle is available)
- [x] 5.2 `nix develop -c just lint` green
- [ ] 5.3 Manual pass: run the app with the Silero Native engine, narrate a text with numbers and punctuation, confirm word highlighting tracks audible narration (checklist for the user)
