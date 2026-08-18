# Delta: ipc-commands

## MODIFIED Requirements

### Requirement: Engine Availability Command

The system SHALL provide `get_available_engines()` returning per-engine
availability (`AvailableEngines`):

```typescript
interface EngineAvailability { available: boolean; reason: string | null }
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
unavailable, `reason` SHALL be a Russian-language user-facing string.

#### Scenario: probe on a system without ttsd
- GIVEN no `pyproject.toml` in the resolved ttsd directory
- WHEN `get_available_engines` is invoked
- THEN `silero.available` is `false` with a Russian `reason`, and `piper.available` is `true`

#### Scenario: probe when uv cannot be spawned
- GIVEN a `pyproject.toml` exists in the resolved ttsd directory but the
  `uv` binary is not installed (spawn fails)
- WHEN `get_available_engines` is invoked
- THEN `silero.available` is `false` with a Russian `reason`, the command
  succeeds, and `piper.available` is `true`

#### Scenario: native engine unavailable before bundle download
- GIVEN no model bundle in the app data dir
- WHEN `get_available_engines` is invoked
- THEN `silero_native.available` is `false` with a Russian `reason` explaining
  that the model bundle must be downloaded
