# Tasks: Cancel must reach the swapped-out engine's ttsd

## Implementation

- [x] `src-tauri/src/tts/switcher.rs` — add
  `last_silero: RwLock<Option<Weak<dyn TtsEngine>>>` to `EngineSwitcher`;
  populate it in `new()` (when the initial engine is Silero) and in
  `apply_config` (on every Silero build); `kill_current_ttsd()` kills the
  current engine and then the weak-referenced previous one when still
  alive.
- [x] Unit test: initial fake Silero engine with a kill counter is swapped
  out via `apply_config("piper", …)`; `kill_current_ttsd()` reaches the
  swapped-out engine while its `Arc` is still held.

## Validation

- [x] `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml` green.
- [x] `nix develop -c just lint` green.
- [x] openspec validate fix-cancel-after-engine-switch --strict green.
