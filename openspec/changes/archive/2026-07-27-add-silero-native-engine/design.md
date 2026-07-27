# Design: add-silero-native-engine

## Context

Silero TTS v5 ships as a `torch.package` (`v5_x_ru.pt`) and runs through the
Python `ttsd` sidecar (PyTorch + Python runtime, opt-in flake flag
`withSilero`). The spike (`tmp/onnx-spike/REPORT.md`) established:

- The v5 pipeline is non-autoregressive (FastPitch-like):
  `text → accentor (HomoSolver BERT + AccentorNgram) → symbol ids →
  dur/pitch predictors → encoder → LengthRegulator (repeat_interleave) →
  HourGlass decoder → ConvNeXt vocoder → ISTFT (48k) / PQMF (24k, 8k)`.
- Every neural component exports to ONNX with verified parity
  (`tts_main.onnx` 81 MB, `istft.onnx` 23 MB, `homosolver.onnx` 117 MB,
  `accentor_tensor.onnx` 8 MB); the string part of the accentor
  (ngram + exceptions dicts) ports to Rust.
- Export needs programmatic JIT-graph surgery (custom symbolics for
  `_native_multi_head_attention` and `repeat_interleave`, `aten::format`
  fast-path removal); weights and model code are never patched.
- CPU speedup ~10x vs Python `apply_tts`.

## Goals / Non-Goals

**Goals:**

- In-process Silero v5 engine in Rust on ONNX Runtime, quality-identical to
  the Python path (parity-gated).
- Reproducible, pinned, self-checking exporter + bundle published to GitHub
  Releases; app downloads and verifies the bundle.
- Third engine option; Piper default and `ttsd` fallback untouched.

**Non-Goals:** see proposal (no SSML/phonetic inserts, no `dur_hat`
timestamps in v1, no ttsd deprecation, no default change).

## Decisions

### D1. ONNX Runtime via `ort`, not libtorch / candle

`ort` (pykeio/ort v2) runs the exported graphs as-is with the observed 10x
speedup. Alternatives rejected: libtorch (`tch-rs`) keeps a PyTorch-sized
runtime for zero quality gain; candle/burn would require reimplementing the
architecture and weight conversion — weeks of work and parity risk.
Linkage strategy (system onnxruntime vs `load-dynamic`) is decided by the
Phase-1 nix spike and documented in `nix/devshell.nix` comments — this is
the one open technical risk.

### D2. Standalone `silero-native/` crate, path dependency

No root cargo workspace (would perturb `src-tauri` builds and flake
packaging); `src-tauri` depends on it via `path = "../silero-native"`.
The crate is pure Rust with no Tauri dependencies, mirroring how
`src-tauri/src/pipeline/` is kept Tauri-free — testable in isolation.
Layout: `src/` (engine), `export/` (uv project, build-time only), `tests/`
(golden parity), `docs/` (architecture, debugging), `README.md`, `NOTICE`.

### D3. Pre-exported bundle on GitHub Releases, not in-app export

Export requires torch + Python; shipping that toolchain to users defeats the
purpose. The exporter is a maintainer tool: pinned model version from
`models.yml` (no `latest` — also fixes ttsd's unpinned-model risk on our
side), `manifest.json` with per-file sha256 + source `.pt` hash, and a
mandatory parity self-check (waveform max abs diff ≤ 1e-3 on a fixed phrase
set, non-zero exit otherwise). A GitHub Actions workflow runs the exporter
and uploads the bundle to a `silero-models-v5.x` release. In-app export and
installer bundling were rejected (UX and size respectively).

### D4. Full accentor parity (ngram + HomoSolver BERT)

The user chose full v5 parity: ngram accentor (Rust dict lookup +
`accentor_tensor.onnx`) plus the HomoSolver BERT (`homosolver.onnx`) for
homographs. Alternative rejected: ngram-only (v4-level quality) — saves
~117 MB and 2–3 days but regresses homograph pronunciation vs the engine
users already have. The BERT tokenizer
(`custom_tokenizers/bert_tokenizer.py`) is ported to Rust; it is the most
drift-prone piece and gets dedicated golden tests.

### D5. Bundle management mirrors the Piper voice download

Download into the app data dir, sha256 verification against the manifest,
progress events, offline/error states — the same UX pattern as
`download_piper_voice`, reusing its event throttling and UI affordances.
Alternative rejected: bundle in the installer (+230 MB for everyone,
including Piper-only users).

### D6. API shape mirrors the ttsd protocol semantics

`SileroNative::load(bundle_dir)` once; `synthesize(text, speaker,
sample_rate) -> SynthesisResult { wav, timestamps, duration_sec }` with the
same output contract as `OkSynthesize`, so `src-tauri/src/tts/` treats all
engines uniformly. Synthesis runs via `spawn_blocking` with panic isolation
(`catch_unwind`) instead of ttsd's process supervision. Timestamps v1 keep
the char-proportional algorithm (ported from `ttsd/timestamps.py` semantics)
to stay output-compatible; `dur_hat`-based timestamps are issue #145.

### D7. All three sample rates, default 24000

PQMF export is in scope (24k/8k path), per user decision; the native engine
defaults to 24000 (halves decode+ISTFT cost vs 48k at indistinguishable
listening quality for speech).

## Risks / Trade-offs

- [onnxruntime linkage in the nix build fails or bloats the bundle] → Phase-1
  spike before any other work; fallback is `load-dynamic` with the runtime
  shipped as a data-dir artifact; worst case the change is abandonable
  before large investment.
- [BERT tokenizer port drifts from the Python original] → golden parity
  tests pin frontend output symbol-for-symbol against reference dumps from
  the unpacked package.
- [Zero-duration edge case in `repeat_interleave` (known from the spike)]
  → dedicated unit tests; clamp policy defined in the spec.
- [Upstream releases a new model version that breaks the exporter] →
  version pinning + exporter self-check; re-export is a maintainer action,
  never automatic.
- [License misattribution] → `NOTICE` file, README section, docs mention;
  bundle is a technical format conversion under CC BY-NC-SA 4.0 §2(a)(4),
  non-commercial use only.
- [CI time/size for golden tests needing the ~230 MB bundle] → tests that
  need the bundle use a cached download; a small synthetic ONNX fixture
  covers CI paths that only exercise plumbing.

## Migration Plan

Additive only: new crate, new engine enum variant, new settings option.
Rollback = revert the PR; users on Piper or Python Silero are unaffected.
The model bundle is external (Releases), so rollback leaves a harmless
orphan artifact in the data dir.

## Open Questions

- `ort` linkage strategy (system vs `load-dynamic`) — resolved by the
  Phase-1 nix spike.
- Exact bundle version tag scheme once upstream versions are enumerated
  (`v5_ru` vs `v5_2_ru`…): pin decided at exporter implementation time and
  recorded in the exporter README.
