# Delta: ipc-commands

## MODIFIED Requirements

### Requirement: Command Error Format

All fallible Tauri commands SHALL return errors as a typed JSON object
(`CommandError` in `src-tauri/src/commands/mod.rs`, serialized with
`#[serde(tag = "type", rename_all = "snake_case")]`), causing the frontend
`invoke()` promise to reject with that object:

```typescript
interface CommandError {
  type: "not_found" | "storage_error" | "synthesis_error"
      | "playback_error" | "config_error" | "internal";
  code: string;           // machine-readable error-site id, e.g. "image.fetch_failed"
  params?: string[];      // interpolation values for the localized message
  message?: string;       // optional raw detail (engine/HTTP strings); fallback when `code` is unknown to the frontend
}
```

The frontend SHALL translate known `code`s via its localization catalogs and
SHALL fall back to `message`, then to a generic per-`type` string, for unknown
codes. Backend error sites SHALL NOT embed user-facing prose.

#### Scenario: Command failure rejects with typed error
- **GIVEN** a command such as `get_entry` with a malformed `id`
- **WHEN** the command handler returns an error
- **THEN** the `invoke()` promise rejects with `{ "type": "not_found", "code": "<site-id>", "params": ["<id>"] }`

#### Scenario: Storage error mapping
- **GIVEN** a storage failure where the entry does not exist
- **WHEN** any command surfaces that `StorageError::NotFound`
- **THEN** the error is serialized with `type: "not_found"` and the entry id in `params`; all other storage failures serialize as `type: "storage_error"`

#### Scenario: Unknown code falls back
- **GIVEN** an older frontend or a new backend code missing from the catalogs
- **WHEN** an error with an unknown `code` but present `message` is shown
- **THEN** the UI displays the raw `message`, not a broken placeholder

## MODIFIED Requirements

### Requirement: Engine Availability Command

The system SHALL provide `get_available_engines()` returning per-engine
availability (`AvailableEngines`):

```typescript
interface LocalizedText {
  code: string;        // machine-readable reason id, e.g. "silero.uv_missing"
  params?: string[];   // positional interpolation values
  message?: string;    // optional raw diagnostic detail
}
interface EngineAvailability {
  available: boolean;
  reason: LocalizedText | null; // Some only when available == false
}
interface AvailableEngines {
  piper: EngineAvailability;
  silero: EngineAvailability;
  silero_native: EngineAvailability;
}
```

Piper (in-process) SHALL always report `available: true`. Silero SHALL report
availability based on a cheap probe: presence of `pyproject.toml` in the ttsd
directory and a successful `uv --version` exec; a `uv` binary that cannot be
spawned at all (not installed — the normal case on Windows, where ttsd is
not shipped) SHALL be treated as an unsuccessful probe, not an error.
Silero Native SHALL report availability based on presence and manifest
validity of the downloaded model bundle in the app data dir. When
unavailable, `reason` SHALL carry a machine-readable code (translated by the
frontend like command errors), not user-facing prose.

#### Scenario: probe on a system without ttsd
- **GIVEN** no `pyproject.toml` in the resolved ttsd directory
- **WHEN** `get_available_engines` is invoked
- **THEN** `silero.available` is `false` with `reason.code` `"silero.ttsd_missing"`, and `piper.available` is `true`

#### Scenario: probe when uv cannot be spawned
- **GIVEN** a `pyproject.toml` exists in the resolved ttsd directory but the
  `uv` binary is not installed (spawn fails)
- **WHEN** `get_available_engines` is invoked
- **THEN** `silero.available` is `false` with `reason.code` `"silero.uv_missing"`, the command
  succeeds, and `piper.available` is `true`

#### Scenario: native engine unavailable before bundle download
- **GIVEN** no model bundle in the app data dir
- **WHEN** `get_available_engines` is invoked
- **THEN** `silero_native.available` is `false` with `reason.code`
  `"native.bundle_missing"` indicating that the model bundle must be downloaded

## MODIFIED Requirements (shared types)

### Requirement: Shared IPC Types

`UIConfig` gains one field:

```typescript
interface UIConfig {
  // …existing fields…
  language: string; // UI language: "ru" / "en"; default "ru"
}
```
