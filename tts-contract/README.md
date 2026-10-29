# tts-contract

Data-only contract for text preparation: what a TTS engine can read
(`TextProfile`) and how normalized text becomes bounded synthesis calls
(`split_with_limit`).

## Why a separate crate

The application (`src-tauri`) and the in-process Silero engine
(`silero-native`) are independent cargo workspaces, and the application already
depends on the engine crate — so neither can host code the other needs. This
crate sits below both and depends on nothing. It knows no engine identity:
mapping an engine to a profile is the application's job
(`src-tauri/src/tts/profiles.rs`).

## Checks

Inside the dev shell (`nix develop`):

```bash
cargo test --manifest-path tts-contract/Cargo.toml
cargo clippy --manifest-path tts-contract/Cargo.toml --all-targets --no-deps -- -D warnings
```

## Where the details live

- Field semantics and invariants: the doc comments in `src/profile.rs` and
  `src/chunk.rs` (the types are the contract).
- Behavior: `openspec/specs/text-pipeline/` and
  `openspec/specs/position-mapping/`; `text-chunking` is specified in the
  in-flight change under `openspec/changes/` and joins `openspec/specs/` when
  it is archived.
- Decisions (why data-only, why three markup variants, why the sidecar keeps
  its own splitter): the design record in `openspec/changes/`, archived after
  the change lands.

## License

GPL-3.0-only, same as the repository.
