# silero-native

In-process **Silero TTS v5** engine for RuVox, running on ONNX Runtime —
a native (Rust) replacement for the Python `ttsd` sidecar path. It is the
default TTS engine in the app («Silero (нативный)»), alongside Piper (the
zero-dependency fallback) and Silero via `ttsd` (Python, opt-in).

The engine loads a pre-exported ONNX bundle (~230 MB) that the app downloads on
demand; locally, tests and examples find a bundle through
`SILERO_NATIVE_BUNDLE`.

## Checks

Inside the dev shell (`nix develop`):

```bash
cargo test --manifest-path silero-native/Cargo.toml                    # unit tier
SILERO_NATIVE_BUNDLE=tmp/bundle-v5 cargo test --manifest-path silero-native/Cargo.toml  # parity tier
cargo clippy --manifest-path silero-native/Cargo.toml --all-targets --no-deps -- -D warnings
```

## Where the details live

- Binary layout, pipeline map, bundle format, debugging:
  `silero-native/docs/architecture.md`.
- Performance: `silero-native/docs/benchmarks.md`.
- Re-exporting a new upstream model: `silero-native/export/README.md`.
- Behavior: `openspec/specs/silero-native-engine/`.

## License

The crate and exporter code are GPL-3.0-only (repo license). The model bundle is
a technical format conversion of the upstream Silero TTS model, which is
**CC BY-NC-SA 4.0 © Silero Team** — non-commercial use only, with attribution.
See [NOTICE](NOTICE).
