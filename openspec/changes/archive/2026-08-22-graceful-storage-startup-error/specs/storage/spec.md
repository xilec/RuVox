# storage Delta

## ADDED Requirements

### Requirement: Graceful Startup Failure

When the storage service cannot be opened at startup (per-user directories
unresolvable, permissions denied, I/O error), the application SHALL NOT panic or
abort. It SHALL log a structured error entry with the underlying cause to the
application log, show the user a native error dialog with an actionable
Russian-language message (including the log directory location), and exit the
process cleanly with a non-zero exit code.

#### Scenario: Storage open failure is graceful
- GIVEN the storage service cannot be opened during startup (e.g. the per-user data directory cannot be created)
- WHEN the application starts
- THEN no panic message is produced, an error entry with the cause is written to the application log, a native error dialog in Russian names the problem and points to the log directory, and after it is dismissed the process exits with a non-zero exit code
