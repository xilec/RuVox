# Tasks: add-silero-native-engine

Reference: `docs/plans/silero-native-port.md` (decisions, tracker),
`tmp/onnx-spike/REPORT.md` (feasibility reference — rewrite, do not port).

## 1. Nix / ort validation (blocker check — do first)

- [x] 1.1 Create a minimal throwaway crate under `tmp/` that loads an ONNX model via `pykeio/ort` and runs inference inside `nix develop`
- [x] 1.2 Verify the production flake build (`.#ruvox`) links/runs onnxruntime; decide linkage strategy (system lib vs `load-dynamic`)
- [x] 1.3 Add `onnxruntime` to `nix/devshell.nix` and the flake build with a comment explaining why it is load-bearing; record the chosen strategy in `silero-native/docs/architecture.md` stub

## 2. Subproject scaffolding

- [x] 2.1 Create `silero-native/` crate (`Cargo.toml`, `src/lib.rs`, `src/error.rs` with thiserror domain errors) — no Tauri dependencies
- [x] 2.2 Add `ort` dependency; wire `silero-native` as a path dependency of `src-tauri`
- [x] 2.3 Wire crate tests + fmt/clippy into `justfile` recipes and `.github/workflows/ci.yml`; keep the slim-build gate green

## 3. Model exporter (`silero-native/export/`)

- [x] 3.1 Create uv project with pinned deps (torch CPU, onnx, onnxruntime); pin the upstream model version from `models.yml` (no `latest`)
- [x] 3.2 Rewrite the spike export as a single clean `export.py`: JIT-graph surgery (MHA/repeat_interleave symbolics, `aten::format` removal) for `tts_main.onnx` with dynamic axes
- [x] 3.3 Export `istft.onnx` and `pqmf.onnx` (24k/8k path)
- [x] 3.4 Export `homosolver.onnx` (post `unpack_q_model`) and `accentor_tensor.onnx`; extract `ngrams`/`exceptions` dictionaries
- [x] 3.5 Emit `manifest.json` (model version, per-file sha256, opset, source `.pt` hash, export date)
- [x] 3.6 Built-in self-check: waveform parity ≤ 1e-3 on a fixed phrase set vs the Python reference; non-zero exit on failure
- [x] 3.7 GitHub Actions workflow: run exporter, upload bundle to a `silero-models-v5.x` release
- [x] 3.8 `export/README.md`: how to re-export on a new upstream release

## 4. Rust engine core

- [x] 4.1 Bundle loader: manifest parsing + sha256 verification of every file (typed errors, no silent loads)
- [x] 4.2 Text frontend port: `prepare_text_input` (lowercase, symbol filtering, dash normalization), symbol→id mapping; golden fixtures dumped from the unpacked upstream package
- [x] 4.3 Accentor: Rust ngram/exceptions lookup + `accentor_tensor.onnx` session; explicit `+` stress priority
- [x] 4.4 HomoSolver: BERT tokenizer port (`custom_tokenizers/bert_tokenizer.py` → Rust) + `homosolver.onnx` session
- [x] 4.5 Synthesis: `tts_main.onnx` + `istft.onnx` (+ `pqmf.onnx` for 24k/8k); speakers (id 0–4); default sample rate 24000; zero-duration clamping
- [x] 4.6 Public API: `SileroNative::load(bundle_dir)` / `synthesize(text, speaker, sample_rate) -> SynthesisResult` (wav + char-proportional timestamps + duration, `OkSynthesize`-compatible shape); panic containment at the boundary
- [x] 4.7 Edge-case handling per spec: empty input `bad_input`, unknown speaker/rate rejected before inference, unsupported markup stripped/rejected

## 5. Testing

- [x] 5.1 Golden parity suite (~20–30 phrases: tech text, numbers, homographs, ё, punctuation): frontend output symbol-for-symbol, waveform within threshold, stress identical to `apply_tts`
- [x] 5.2 Unit tests: ngram lookup, BERT tokenizer, zero durations, empty input, single-vowel words
- [x] 5.3 Integration test: full engine against the downloaded bundle (cached in CI); plumbing paths covered with a small synthetic ONNX fixture
- [x] 5.4 Benchmark reproducing spike numbers (~10x vs `apply_tts`); results recorded in `silero-native/docs/`

## 6. App integration

- [ ] 6.1 `src-tauri/src/tts/`: `SileroNative` backend next to `Ttsd`, engine enum gains `silero_native`; synthesis via `spawn_blocking`
- [ ] 6.2 Bundle downloader: GitHub Releases fetch into data dir, manifest sha256 verification, idempotent skip, partial-file quarantine, typed errors
- [ ] 6.3 IPC: `download_silero_native_bundle` command, `bundle_download_*` events, `AvailableEngines.silero_native`, `UIConfig.engine` gains `"silero_native"`; `src/lib/tauri.ts` wrappers
- [ ] 6.4 Settings UI: third engine option «Silero (нативный)» with availability gating, download button + live progress (Russian strings)

## 7. Docs and licensing

- [ ] 7.1 `silero-native/README.md`: overview, architecture diagram, build/test, model download/re-export
- [ ] 7.2 `silero-native/docs/architecture.md`: pipeline map, bundle/manifest format, debugging section (intermediate tensor dumps, Python reference comparison, known edge cases)
- [ ] 7.3 Licensing: `NOTICE` (CC BY-NC-SA 4.0, Silero Team, format conversion per §2(a)(4)), README attribution section, mention in user docs
- [ ] 7.4 Update `AGENTS.md` (layout, test commands) and `docs/`; update `docs/plans/silero-native-port.md` tracker

## 8. Manual verification (pre-PR)

- [ ] 8.1 Manual pass: switch between all three engines; download the bundle from Settings with visible progress; synthesize with the native engine at 24000 and 48000; listen to ref-vs-native parity samples; verify fallback behavior without the bundle
