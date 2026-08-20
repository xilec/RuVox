# Delta: logging

## ADDED Requirements

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
