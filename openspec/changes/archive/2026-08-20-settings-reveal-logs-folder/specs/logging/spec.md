# Delta: logging

New capability. No existing spec is modified.

## ADDED Requirements

### Requirement: Log directory is reachable from Settings

The app SHALL expose the log directory path to the frontend (`get_log_dir`),
and Settings SHALL offer an "Открыть папку" button that reveals that directory
in the OS file manager so the user can grab logs for a support request.

#### Scenario: Settings reveals the log folder

- GIVEN the app running on any supported OS
- WHEN the user presses "Открыть папку" in the Logs section of Settings
- THEN the OS file manager opens the log directory
