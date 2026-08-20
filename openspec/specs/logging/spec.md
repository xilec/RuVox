# logging Specification

## Purpose

Covers diagnostic logging of the RuVox backend: the log file location,
rotation, level control, and how `tracing` records reach the log
(`tauri-plugin-log` with its `tracing` feature).

## Requirements

### Requirement: Diagnostic log file

The app SHALL write diagnostic logs to a per-user log directory via
`tauri-plugin-log`. Our code logs via `tracing`; since no tracing
subscriber is installed, the `tracing` crate's `log` feature SHALL be
enabled so tracing macros fall back to emitting `log` records, which the
plugin's global logger captures. The log directory is the Tauri per-app
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

### Requirement: Update checks are logged

Every update check (startup and manual) SHALL leave a record in the log
file via the `tauri-plugin-log` JS API: the outcome ("up to date" /
"update available: <version>") at info level, or the failure reason at
error level. Startup-check failures SHALL be logged even though they are
silent in the UI.

#### Scenario: Manual check writes a log entry

- GIVEN the app on Windows
- WHEN the user presses "Проверить обновления" in Settings
- THEN the log file contains an `update check (manual)` entry with the
  outcome, regardless of success or failure

#### Scenario: Failed startup check is logged but silent

- GIVEN the app starts without network access to the update endpoint
- WHEN the startup update check runs
- THEN no UI notification is shown AND the log file contains an
  `update check (startup) failed` entry with the error message

### Requirement: Log directory is reachable from Settings

The app SHALL expose the log directory path to the frontend (`get_log_dir`),
and Settings SHALL offer an "Открыть папку" button that reveals that directory
in the OS file manager so the user can grab logs for a support request.

#### Scenario: Settings reveals the log folder

- GIVEN the app running on any supported OS
- WHEN the user presses "Открыть папку" in the Logs section of Settings
- THEN the OS file manager opens the log directory
