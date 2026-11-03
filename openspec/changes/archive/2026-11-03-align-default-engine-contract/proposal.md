## Why

#359 made TeraTTSv2 the default engine and synced the `storage`,
`teratts-engine` and `ui` first-run prompt requirements, but two synced
requirements still declare Silero Native as the default. `ipc-commands`
"Shared IPC Types" keeps `engine: "silero_native" (default)` in the `UIConfig`
comment and asserts `engine: "silero_native"` in the `get_config` defaults
scenario, while the `ui` "Settings dialog" reset scenario does not say that
«Сброс» returns the engine to the storage default through the availability
coercion. Both disagree with the implementation: `UIConfig::default_engine()`
returns `"teratts"`, and `handleReset` runs the factory engine through
`computeEngineFormState`, the same coercion as the initial load. #359 is
already archived, so the correction is a small follow-up change with MODIFIED
deltas.

## What Changes

- `ipc-commands` "Shared IPC Types": change the `engine` field comment to
  `"teratts" (default)` and make the `UIConfig defaults include engine fields`
  scenario expect `engine: "teratts"`.
- `ui` "Settings dialog": extend the `Reset restores factory defaults`
  scenario to state that «Сброс» also returns the engine to the storage
  default `"teratts"`, passed through the same availability coercion as the
  initial load (Piper with the coercion alert when the TeraTTSv2 bundle is
  not installed).
- No code change: the Rust default and the Settings reset path already
  implement this behavior.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `ipc-commands`: the `UIConfig` default engine in the shared-type comment and
  in the `get_config` defaults scenario becomes `"teratts"`.
- `ui`: the Settings reset scenario records the engine returning to the
  storage default and its coercion.

## Impact

- Affected specs: `ipc-commands`, `ui`.
- Affected code: none — `src-tauri/src/storage/schema.rs`
  (`UIConfig::default_engine`) and `src/dialogs/Settings.tsx` (`handleReset`
  via `computeEngineFormState`) already match the corrected text.
- Docs: no README or CHANGELOG change; #359 already covered those.

## Non-goals

- Changing the default engine, the reset behavior or the coercion rules.
- Rewriting the archived #359 change or its implementation.
- Rewording the `storage`, `teratts-engine` or `ui` first-run prompt
  requirements that #359 already synced.
