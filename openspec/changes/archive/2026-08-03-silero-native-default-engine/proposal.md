# Proposal: silero-native-default-engine

## Why

Silero Native (in-process Silero v5 via ONNX) is now the most capable engine in the app: no Python sidecar, precise `dur_hat`-based word timestamps, and fast warmup. Piper remains the zero-dependency fallback, but new installs should get the better default experience out of the box.

## What Changes

- Config defaults in `src-tauri/src/storage/schema.rs`:
  - `engine`: `"piper"` → `"silero_native"`
  - `speaker`: `"xenia"` → `"aidar"`
  - `sample_rate`: `48000` → `24000` (the native engine's own default)
- Settings dialog defaults and the engine option label (`Piper (по умолчанию, без Python)` — Piper is no longer the default) updated to match.
- Behavior when the Silero Native bundle is not downloaded is unchanged: startup silently serves Piper for the run (config value preserved), and the Settings dialog offers the bundle download.
- Comments/docs referencing the old defaults (`src-tauri/src/lib.rs`, `src/lib/tauri.ts`, `ai/rules/conventions.md`) are updated.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `storage`: the "Config File Schema" requirement — default values for `engine`, `speaker`, and `sample_rate` change.
- `ipc-commands`: the "Shared IPC Types" requirement — `UIConfig` default annotations and the fresh-install defaults scenario change.
- `ui`: the "Settings dialog" requirement — saving while the form is coerced to the fallback engine no longer persists that fallback (new exposure once the default engine can be unavailable).

## Non-goals

- No forced migration: existing `config.json` files already carry explicit `engine`/`speaker`/`sample_rate` values and are untouched — only absent keys adopt the new defaults.
- Bundle auto-download on first run: out of scope; the existing Settings download action remains the way to get the bundle.
- Piper stays the fallback engine; its voice default (`ruslan`) is unchanged.

## Impact

- **Code:** `src-tauri/src/storage/schema.rs` (defaults + tests), `src/dialogs/Settings.tsx` (initial form values, engine option label), `src-tauri/src/lib.rs` and `src/lib/tauri.ts` (doc comments), `ai/rules/conventions.md` (engine-roles line).
- **Specs:** deltas on `openspec/specs/storage/` and `openspec/specs/ipc-commands/`.
- **APIs/contracts:** unchanged field names and types; only default values change.
- **Dependencies:** none new.
