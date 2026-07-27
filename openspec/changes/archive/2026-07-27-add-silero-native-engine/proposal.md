# Proposal: add-silero-native-engine

## Why

Silero TTS v5 (the best Russian voices we ship) currently requires the Python
`ttsd` sidecar: a heavy opt-in dependency (PyTorch, Python runtime) that
complicates packaging and startup. A feasibility spike (`tmp/onnx-spike/`,
report `tmp/onnx-spike/REPORT.md`) proved the full v5 pipeline exports to ONNX
with waveform parity 1.7e-4 and runs ~10x faster on CPU than Python
`apply_tts`. A native Rust + ONNX Runtime engine removes the Python dependency
for Silero users while keeping identical voice quality.

## What Changes

- New `silero-native/` Rust crate: in-process Silero v5 engine (text frontend,
  ngram accentor + HomoSolver BERT, FastPitch synthesis, ISTFT/PQMF) running
  on ONNX Runtime (`ort` crate) from a pre-exported model bundle.
- New `silero-native/export/` uv project: reproducible exporter that converts
  a pinned upstream `v5_x_ru.pt` into the ONNX bundle with a built-in parity
  self-check; a CI workflow publishes the bundle to GitHub Releases.
- The app gains a third TTS engine «Silero (нативный)» alongside Piper
  (default, unchanged) and Silero (Python, kept as fallback). The engine
  becomes available after a one-time bundle download (~230 MB) from GitHub
  Releases with sha256 verification and progress events.
- Sample rates 8000 / 24000 / 48000 supported; the native engine defaults to
  24000.
- Word-level timestamps stay on the existing char-proportional estimation in
  v1 (precise `dur_hat`-based timestamps are deferred to issue #145).

## Capabilities

### New Capabilities

- `silero-native-engine`: behavior of the in-process ONNX Silero engine —
  bundle layout and verification, engine loading, text frontend (accentor,
  homographs, ё), synthesis (speakers, sample rates), output format, error
  and edge-case handling.

### Modified Capabilities

- `ipc-commands`: the engine enum gains `"silero-native"`;
  `AvailableEngines` gains a third entry; new commands for bundle download
  (`download_silero_native_bundle`, status/cancel as needed) and download
  progress events.
- `ui`: the Settings form exposes the third engine option with its
  availability gating and bundle-download affordance.

## Impact

- **New code:** `silero-native/` crate (engine + exporter + tests + docs),
  `src-tauri/src/tts/` backend wiring, `src/dialogs/Settings.tsx`,
  `src/lib/tauri.ts` invoke wrappers.
- **Dependencies:** `ort` (ONNX Runtime) in Rust; `onnxruntime` in
  `nix/devshell.nix` and the flake build; export-time Python deps (torch,
  onnx, onnxruntime) confined to `silero-native/export/` — they never enter
  the app bundle.
- **CI:** crate tests + lint gates; bundle-export workflow; slim-build gate
  must stay green (the native engine is part of the slim build, the Python
  sidecar remains opt-in).
- **Licensing:** model bundle files are CC BY-NC-SA 4.0 (Silero Team);
  `NOTICE` + attribution in `silero-native/` and user docs.
- **Unaffected:** `ttsd` Python engine, Piper engine, normalization pipeline,
  playback/queue.

## Non-goals

- Replacing or deprecating the Python `ttsd` engine (stays as fallback).
- Changing the default engine (Piper stays default).
- Precise `dur_hat`-based word timestamps (issue #145).
- SSML support and `[[...]]` phonetic inserts in the native frontend.
- Porting the exporter itself to Rust (Python+torch remains a build-time-only
  tool).
