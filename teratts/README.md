# teratts

In-process TeraTTSv2 engine: a 99M-parameter Russian/English flow-matching
ONNX model running on the ONNX Runtime the app already vendors, with no Python
or PyTorch at synthesis time.

The crate owns the model bundle format and verification, the four ONNX
sessions, the Russian text frontend (language tags, RUAccent dictionary
stress, codepoint gate), sentence chunking under the model's 25-second call
limit, approximate word timestamps, and the 44.1 kHz WAV writer. The
application adapter (`src-tauri/src/tts/teratts/`) maps it to `TtsEngine`.

Check it with:

```bash
cargo test --manifest-path teratts/Cargo.toml
```

Bundle-gated tests need a bundle directory and are skipped otherwise:

```bash
TERATTS_BUNDLE=/path/to/bundle cargo test --manifest-path teratts/Cargo.toml
```

Behavior is specified in `openspec/specs/teratts-engine/`; the bundle exporter
and the update procedure live in `export/`.
