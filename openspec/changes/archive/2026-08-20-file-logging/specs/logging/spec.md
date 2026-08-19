# Delta: logging

New capability. No existing spec is modified.

## ADDED Requirements

### Requirement: Diagnostic log file

The app SHALL write diagnostic logs to a per-user log directory via
`tauri-plugin-log` (its `tracing` feature SHALL be enabled so existing
`tracing` records reach the log). The log directory is the Tauri per-app
log dir: `%LOCALAPPDATA%\com.ruvox.app\logs` on Windows,
`~/.local/share/com.ruvox.app/logs` on Linux, inside the app's
`Library/Logs` dir on macOS. This is a second data location, separate
from the storage root (see the storage spec). Debug builds SHALL also
log to stdout.

#### Scenario: Release build writes a log file

- GIVEN an installed release build of RuVox
- WHEN the app starts and synthesizes an entry
- THEN a log file in the log dir contains startup and engine-selection
  entries at info level or above

#### Scenario: Debug build logs to stdout

- GIVEN a debug build (`cargo tauri dev`)
- WHEN the app runs
- THEN log records are emitted to stdout as well as the log file

### Requirement: Log rotation and level

Old log files SHALL be rotated (`RotationStrategy::KeepSome`) so the log
dir does not grow without bound, and each log file SHALL be size-capped.
The minimum level SHALL default to `info` and SHALL be overridable via
the `RUST_LOG` environment variable.

#### Scenario: RUST_LOG overrides the default level

- GIVEN the app is started with `RUST_LOG=debug`
- WHEN the app runs
- THEN the log file contains debug-level records
