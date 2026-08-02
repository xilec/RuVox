# silero-native

In-process **Silero TTS v5** engine for RuVox, running on ONNX Runtime —
a native (Rust) replacement for the Python `ttsd` sidecar path. It is the
third TTS engine in the app («Silero (нативный)»), alongside Piper
(default) and Silero via `ttsd` (Python, kept as fallback).

## What is where

```
silero-native/
├── src/                 # the engine crate
│   ├── bundle.rs        # manifest parsing + sha256 verification + ONNX sessions
│   ├── frontend/        # text frontend ported from the upstream torch.package
│   │   ├── text.rs      #   prepare_text_input, symbol→id sequence
│   │   ├── accentor.rs  #   ngram stress/ё placement (+ accentor_tensor.onnx)
│   │   ├── homosolver.rs#   homograph disambiguation (+ homosolver.onnx)
│   │   └── bert.rs      #   BasicTokenizer + WordPiece port
│   ├── engine.rs        # tts_main → istft → pqmf synthesis pipeline
│   ├── timestamps.rs    # dur_hat-based word timestamps
│   └── error.rs         # thiserror domain errors
├── export/              # maintainer tool: .pt → ONNX bundle (uv project)
├── tests/               # unit (no model) + bundle-gated (parity) tests
├── examples/            # synthesize.rs, bench.rs (manual/bench drivers)
├── docs/
│   ├── architecture.md  # pipeline map, bundle format, debugging guide
│   └── benchmarks.md    # synthesis speed measurements
└── NOTICE               # upstream model license attribution (CC BY-NC-SA 4.0)
```

## Model bundle

The engine loads a pre-exported ONNX bundle (~230 MB): `tts_main.onnx`,
`istft.onnx`, `pqmf_24k.onnx`, `pqmf_8k.onnx`, `homosolver.onnx`,
`accentor_tensor.onnx`, accentor dictionaries, tokenizer vocab,
`frontend.json`, `manifest.json` (per-file sha256 + provenance).

- The app downloads it on demand from the `silero-models-v5_ru` GitHub
  Release into `<data_local_dir>/ruvox/voices/silero-native/` (idempotent,
  checksum-verified).
- Locally, point tests/examples at any bundle via `SILERO_NATIVE_BUNDLE`
  (default lookup: `tmp/bundle-v5`).
- Re-exporting on a new upstream release: see `export/README.md`.

## Build and test

All commands run inside the dev shell (`nix develop -c ...`); `ort` finds
onnxruntime via `ORT_DYLIB_PATH` (see `docs/architecture.md`).

```bash
cargo test --manifest-path silero-native/Cargo.toml    # unit tier; bundle tier skips without a bundle
SILERO_NATIVE_BUNDLE=tmp/bundle-v5 cargo test --manifest-path silero-native/Cargo.toml  # full tier
cargo clippy --manifest-path silero-native/Cargo.toml --no-deps -- -D warnings
cargo run --release --manifest-path silero-native/Cargo.toml --example synthesize  # writes tmp/native-*.wav
```

## Usage (library)

```rust
let engine = silero_native::SileroNative::load(bundle_dir)?;
let result = engine.synthesize("Привет, мир!", "xenia", 24000)?;
// result.wav — 16-bit PCM WAV; result.timestamps; result.duration_sec
```

## License

The crate and exporter code are GPL-3.0-only (repo license). The model
bundle is a technical format conversion of the upstream Silero TTS model,
which is **CC BY-NC-SA 4.0 © Silero Team** — non-commercial use only, with
attribution. See [NOTICE](NOTICE).
