# src-tauri

Rust backend of the RuVox desktop app and the Tauri 2 shell that hosts the
React frontend. Everything that touches text, audio, storage or the OS lives
here; the frontend reaches it only through the commands in `src/commands/`.

## What is where

```
src-tauri/
├── src/
│   ├── pipeline/     # text normalization: 20+ phases over TrackedText (CharMapping)
│   │   └── normalizers/  # code blocks, code identifiers, URLs, abbreviations,
│   │                     # English, numbers, symbols
│   ├── tts/          # engine-agnostic TTS layer
│   │   ├── engine.rs     # TtsEngine trait + EngineKind
│   │   ├── profiles.rs   # static TextProfile per engine (tts-contract)
│   │   ├── supervisor.rs # ttsd sidecar: spawn, NDJSON, auto-restart
│   │   ├── piper/        # Piper engine, voice catalogue, chunked synthesis
│   │   ├── silero_native/# in-process Silero engine adapter (silero-native crate)
│   │   ├── switcher.rs   # moves between engines at runtime
│   │   └── chunking.rs   # re-export shim for tts-contract's splitter
│   ├── storage/      # history JSON, audio files, cache eviction, schema
│   ├── player/       # tauri-plugin-mpv wrapper
│   ├── dictionary/   # user dictionary: model, validation, import/export
│   ├── audio/        # WAV/Opus handling
│   ├── commands/     # #[tauri::command] surface + orchestration (synthesis worker)
│   ├── tray/         # system tray
│   ├── state.rs      # AppState: config, pipeline, engines, storage
│   ├── paths.rs      # data/cache directories
│   └── import.rs     # file import
├── tests/            # integration tests: golden pipeline corpus, perf, probes
│   └── fixtures/     # golden inputs/expected output/char maps, mock ttsd scripts
└── deny.toml         # cargo-deny: advisories and license whitelist
```

## Text pipeline and profiles

`pipeline::TTSPipeline` is pure Rust with no Tauri dependencies. It owns the
normalizers and runs them in a fixed, load-bearing phase order (see
`pipeline/mod.rs` and the `text-pipeline` spec).

What it does to a fragment depends on the engine's `TextProfile`
(`tts/profiles.rs`, types from the `tts-contract` crate): a Russian-only
engine gets Latin transliterated, digits and units expanded; an engine that
reads them itself gets the fragment left alone and annotated. Engines consume
the profile for their own limits too — chunk size and pauses come from there,
not from constants in the engine modules.

## Behavior is specified elsewhere

`openspec/specs/` is the source of truth for observable behavior
(`text-pipeline`, `position-mapping`, `word-highlight`, `storage`,
`ttsd-protocol`, `piper-engine`, `silero-native-engine`, …). Code comments
explain only non-obvious why.

## Build and test

All commands run inside the dev shell (`nix develop`):

```bash
cargo test --manifest-path src-tauri/Cargo.toml        # unit + integration + golden corpus
cargo test --manifest-path src-tauri/Cargo.toml --test golden   # golden fixtures only
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --no-deps -- -D warnings
cargo run --manifest-path src-tauri/Cargo.toml          # the app (normally via `pnpm tauri dev`)
```

Golden fixtures live in `tests/fixtures/pipeline/`: every `*.input.txt` has an
expected `*.expected.txt` and `*.char_map.json`, and a pipeline change that
alters output must update them deliberately.

`tests/piper_chunk_limit_probe.rs` is a manual memory probe (ignored by
default, needs a downloaded voice) that justified Piper's chunk limit.

## License

GPL-3.0-only, same as the repository.
