# Delta: ipc-commands

New capability. No existing spec is modified.

## ADDED Requirements

### Requirement: Log directory command

The system SHALL provide `get_log_dir()`, which returns the absolute
per-user log directory path (the same directory `tauri-plugin-log` writes
its rotated files into). The command SHALL create the directory if it does
not yet exist, so the frontend can reveal a real path even before any log
line is flushed. The frontend reveals this path in the OS file manager via
a "Открыть папку" button in Settings.

#### Scenario: log dir is created and returned

- GIVEN the app running on any supported OS
- WHEN `get_log_dir` is invoked
- THEN it returns the absolute per-app log directory path and that directory
  exists on disk

