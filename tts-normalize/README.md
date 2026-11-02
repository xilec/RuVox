# tts-normalize

The text layer shared by RuVox and any other consumer of the same narration
behaviour: the normalization pipeline (numbers, units, dates, URLs, code,
English reading, stress), the user dictionary with its TOML file format, the
per-engine `TextProfile` tables and the `EngineKind` they are keyed by.

Scope: text in, text out. Synthesis, timings, bundle loading and audio belong to
the engine crates. The only filesystem access is the dictionary store, so a
consumer reads exactly the file the app writes.

## Checks

Inside the dev shell (`nix develop`):

```bash
cargo test --manifest-path tts-normalize/Cargo.toml
cargo clippy --manifest-path tts-normalize/Cargo.toml --all-targets --no-deps -- -D warnings
```

## Using it from another project

The repository root is one cargo workspace, so the crate resolves as a git
dependency:

```toml
tts-normalize = { git = "https://github.com/xilec/RuVox", rev = "…" }
```

## Where the details live

- Behavior: `openspec/specs/text-pipeline/` and
  `openspec/specs/position-mapping/`.
- Per-engine text decisions: `src/profiles.rs` — what each engine reads itself,
  and what the pipeline therefore rewrites.
- Dictionary file format and recovery rules: the doc comment in
  `src/dictionary/store.rs`.
- Golden corpus: `src-tauri/tests/fixtures/pipeline/` (run by the app crate,
  which re-exports this crate's pipeline).

## License

GPL-3.0-only, same as the repository.
