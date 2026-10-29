# tts-contract

Data-only contract for text preparation in RuVox: what each TTS engine can
read (`TextProfile`) and how normalized text becomes bounded synthesis calls
(`split_with_limit`).

## Why a separate crate

Two independent cargo workspaces need the same data:

- the application (`src-tauri`) runs the normalization pipeline and decides
  what to rewrite per engine;
- the in-process Silero engine (`silero-native`) splits long text before
  inference and applies its own charset filtering.

`src-tauri` already depends on `silero-native`, so only the engine crate could
host shared code — and then Piper and every later engine would depend on the
Silero crate. `tts-contract` sits below both and depends on nothing.

Nothing here knows about a specific engine: mapping an engine identity
(`EngineKind`, which lives in the application) to a profile is the
application's job (`src-tauri/src/tts/profiles.rs`).

## What is where

```
tts-contract/
└── src/
    ├── profile.rs   # TextProfile, Route, Reading, Markup, Digits, Units,
    │                # Stress, Charset, Chunk, Limit, Pauses + invariants
    └── chunk.rs     # split_with_limit — the one splitter for in-process engines
```

## The profile

| field | question it answers |
|---|---|
| `languages` | which language codes the engine reads natively; every voice of the engine inherits the list |
| `route` | per fragment kind (`latin_prose`, `identifier`): does the pipeline narrate it in Russian, or leave it for the engine |
| `digits` | who expands numbers, dates, times, ranges and versions |
| `units` | who expands percentages, sizes and currency — separate from `digits` because a unit reading is language data |
| `stress` | `None` / `Auto` (explicit marks win per word) / `Manual` (only explicit marks) |
| `charset` | `Any`, or `ModelAlphabet` — the engine silently drops codepoints outside its model's table, so the adapter must filter |
| `chunk` | the per-call limit (`Chars(n)` or `Secs(n)`) and the pauses at a paragraph break and at a language-segment boundary |
| `markup` | how the adapter wraps a `Native` span: `None`, `RuEnTags` (`<ru>…</ru>`) or `LangTag` (`<lang="en">…</lang>`) |

Invariants live in `TextProfile::validate`: a profile that routes any Latin
fragment `Native` must list a language other than Russian, otherwise there is
no language to read the fragment in.

## Usage

```rust
use tts_contract::split_with_limit;

// (chunk_text, codepoint offset in the source) — the contract the timestamp
// estimators consume; whitespace between chunks is not synthesized.
let chunks = split_with_limit(normalized_text, 900);
```

```rust
// Only the adapter renders markup, and only around spans it routes natively;
// the pipeline never emits tags and CharMapping never sees them.
let wrapped = profile.markup.wrap("en", "Kubernetes");
```

## Build and test

All commands run inside the dev shell:

```bash
nix develop -c cargo test --manifest-path tts-contract/Cargo.toml
nix develop -c cargo clippy --manifest-path tts-contract/Cargo.toml --all-targets --no-deps -- -D warnings
```

The behavior these types serve is specified in
[`openspec/specs/text-pipeline/`](../openspec/specs/text-pipeline/spec.md) and
[`position-mapping`](../openspec/specs/position-mapping/spec.md); `text-chunking`
is specified in the in-flight
[change](../openspec/changes/normalize-per-engine-blocks/specs/text-chunking/spec.md)
and moves into `openspec/specs/` when it is archived.

## License

GPL-3.0-only, same as the repository.
