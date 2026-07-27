# Design: engine-aware-input-limit

## Context

`validate_input_length` (`src-tauri/src/commands/mod.rs`) rejects input longer
than `MAX_INPUT_CHARS` (100 000 codepoints) in `ingest_text` and
`preview_normalize`. It exists solely as a guard for Piper's unchunked one-shot
inference; Silero (`ttsd`) chunks input into ≤900-char pieces
(`ttsd/ttsd/chunking.py`, `MAX_CHUNK_SIZE = 900`) and synthesizes sequentially,
so memory stays bounded regardless of input length. Normalization itself is
near-linear since `fix-pipeline-quadratic` (locked by a 1 MB / 10 s perf test),
so no pipeline-side reason remains to cap Silero input.

## Decision

### Gate the check on the active engine kind

`AppState.tts` is the `EngineSwitcher` (as `Arc<dyn TtsEngine>`); its `kind()`
is a synchronous read of an `AtomicU8` maintained across engine swaps. Both call
sites have `state` in scope, so `validate_input_length` takes the kind:

```rust
fn validate_input_length(text: &str, engine: EngineKind) -> CmdResult<()> {
    if engine == EngineKind::Piper && text.chars().count() > MAX_INPUT_CHARS {
        return Err(CommandError::Internal {
            message: "текст слишком длинный для движка Piper (максимум \
                      100 000 символов); сократите текст или переключитесь \
                      на Silero в настройках"
                .to_string(),
        });
    }
    Ok(())
}
```

The kind is read at ingestion time. If the user switches engines afterwards,
already-ingested entries are unaffected at ingestion — but review found the
guard could then be bypassed: an oversized entry accepted under Silero would
hit Piper's unchunked run if its synthesis (queued or via `regenerate_entry`)
executes after a switch to Piper. So the guard is **also re-checked at
synthesis time**: the shared background task (`spawn_synthesis`) fails the
entry with the same message when the engine active at synthesis start is
Piper and the text is oversized. The check is centralized in
`oversized_input_message(text, kind)`, used by both `validate_input_length`
(ingestion/preview) and the synthesis task.

### Message wording

Russian (user-facing), names the engine and the two ways out (shorten / switch
to Silero). The space-grouped literal "100 000" stays in sync with
`MAX_INPUT_CHARS` as before.

## Alternatives considered

- **Per-engine configurable limits** — over-engineering; Silero needs no limit
  at all, and Piper's value is a stopgap until chunking lands (#155).
- **Drop the limit entirely** — Piper still OOMs on long input; the cap keeps
  the failure fast and explicit instead of a system hang.
- **Check at synthesis time instead of ingestion** — worse UX: the entry would
  persist and then fail in the background; rejecting at ingestion keeps the
  error synchronous and side-effect free.

## Testing

- `build_test_app()` gains a `build_test_app_with_kind(kind)` variant that
  installs the `EngineSwitcher` with the given initial kind (the switcher's
  atomic kind is what `state.tts.kind()` reads; the stub engine's own
  hardcoded `kind()` is irrelevant here). `build_test_app()` delegates with
  `EngineKind::Piper`, so existing oversized-input tests keep passing
  unchanged.
- New tests: oversized input accepted with `EngineKind::Silero` for both
  `ingest_text` and `preview_normalize` (entry is created / normalized output
  returned); input at the limit still accepted for Piper.
- Gates: `just test`, `just lint`,
  `openspec validate --specs --strict` after spec sync.
