# src-tauri

Rust backend of the RuVox desktop app and the Tauri 2 shell that hosts the
React frontend: text normalization, the TTS engines, storage, audio and the
command surface the UI talks to.

## Checks

Inside the dev shell (`nix develop`):

```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --no-deps -- -D warnings
```

## Where the details live

- Behavior: `openspec/specs/` (`text-pipeline`, `position-mapping`,
  `word-highlight`, `storage`, `ttsd-protocol`, `piper-engine`,
  `silero-native-engine`).
- Layout, toolchain, commands: `AGENTS.md` and `docs/development.md`.
- Per-engine text decisions: `src/tts/profiles.rs`; the contract types come
  from the `tts-contract` crate.

## License

GPL-3.0-only, same as the repository.
