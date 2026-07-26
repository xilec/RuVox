# Tasks: Await warmup before retry

## Implementation

- [x] `src-tauri/src/tts/supervisor.rs`: introduce `WarmupState`
  (`WarmingUp | Ready | Failed`) and make the slot
  `RwLock<Option<LiveHandle>>` with `LiveHandle { proc, ready:
  watch::Receiver<WarmupState> }`; initial spawn installs state `Ready`.
- [x] `ensure_respawned`: after a successful spawn, create the readiness
  channel (`WarmingUp`), spawn the warmup task (unchanged events, state flip
  at the end), install the `LiveHandle`.
- [x] `with_retry`: after obtaining the current `LiveHandle`, await
  readiness while `WarmingUp` before running the operation.
- [x] Fix the stale `spawn_warmup` comment (ttsd does not auto-load the
  model on synthesize).

## Tests

- [x] New mock ttsd fixture that answers `model_not_loaded` until `warmup`
  is called; integration test: request issued right after a kill/crash
  succeeds without the client calling warmup explicitly.
- [x] Existing supervisor tests (suicide respawn, kill_current, fatal,
  second-chance) stay green.

## Validation

- [x] `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml` (incl.
  `--features test-helpers --test supervisor`) green.
- [x] `nix develop -c just lint` green.
- [x] `nix develop -c pnpm dlx @fission-ai/openspec@1.6.0 validate
  await-warmup-before-retry --strict` green.
