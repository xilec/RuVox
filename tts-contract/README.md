# tts-contract

Data-only contract for text preparation: what a TTS engine can read
(`TextProfile`) and how normalized text becomes bounded synthesis calls
(`split_with_limit`). Depends on nothing and knows no engine identity — mapping
an engine to a profile is the application's job.

## Checks

Inside the dev shell (`nix develop`):

```bash
cargo test --manifest-path tts-contract/Cargo.toml
cargo clippy --manifest-path tts-contract/Cargo.toml --all-targets --no-deps -- -D warnings
```

## Where the details live

- Field semantics and invariants: the doc comments in `src/profile.rs` and
  `src/chunk.rs`.
- Behavior: `openspec/specs/text-pipeline/`, `openspec/specs/position-mapping/`
  and `openspec/specs/text-chunking/`.
- Decisions: `openspec/changes/` (archived after each change lands).

## License

GPL-3.0-only, same as the repository.
