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

## MODIFIED Requirements (shared types)

### Requirement: Shared IPC Types

`UIConfig` gains one field:

```typescript
interface UIConfig {
  // …existing fields…
  language: string; // UI language: "ru" / "en"; default "ru"
}
```
