# storage Delta

## MODIFIED Requirements

### Requirement: Cache Directory Layout

The system SHALL store persistent data under two per-user roots: a **data root**
holding `history.json` and `audio/`, and a **config root** holding `config.json`:

```
<data_root>/
├── history.json                         # Versioned list of TextEntry records
└── audio/
    ├── {uuid}.opus                      # Ogg-Opus audio (32 kbps VOIP, mono)
    └── {uuid}.timestamps.json           # Word-level timestamps for the entry

<config_root>/
└── config.json                          # Application configuration (UIConfig)
```

The roots are platform-dependent:

- **Windows:** both roots coincide with `dirs::data_local_dir()/<bundle identifier>` (`%LOCALAPPDATA%\com.ruvox.app`). They MUST NOT coincide with the NSIS install dir (`%LOCALAPPDATA%\<productName>`) and MUST match the directory the NSIS uninstaller removes via its "Delete the application data" checkbox.
- **Other platforms:** the data root is `dirs::data_local_dir()/ruvox` (e.g. `~/.local/share/ruvox/`) and the config root is `dirs::config_dir()/ruvox` (e.g. `~/.config/ruvox/`).

The storage service SHALL create each root and the `audio/` subdirectory on initialization if they do not exist.

#### Scenario: First launch creates both directory trees
- GIVEN neither the data root nor the config root exists
- WHEN the storage service is initialized
- THEN the data root with its `audio/` subdirectory and the config root exist on disk

#### Scenario: Default roots location
- GIVEN no custom directories are configured
- WHEN the storage service is constructed with defaults
- THEN the data root is `%LOCALAPPDATA%\com.ruvox.app` on Windows and `~/.local/share/ruvox/` on Linux, and the config root is the same directory as the data root on Windows and `~/.config/ruvox/` on Linux

## ADDED Requirements

### Requirement: Legacy Cache Layout Migration

On startup the system SHALL migrate the legacy single-root layout from
`dirs::cache_dir()/ruvox` (Linux) into the two-root layout when the legacy directory
exists. Migration SHALL be per-item over `audio/`, `config.json`, and
`history.json`: an item moves only when its destination does not already exist,
making the migration idempotent and tolerant of partially completed earlier runs.
Items SHALL move in the order audio, then config, then history, so entry validation
on load never observes a moved `history.json` against not-yet-moved audio. Each move
SHALL prefer `rename` and fall back to copy-then-delete-source across filesystems.
After migration the system SHALL remove the legacy directory when it is empty.
Per-item failures SHALL be logged and SHALL NOT prevent startup.

#### Scenario: Legacy layout migrates on first launch
- GIVEN `~/.cache/ruvox/` containing `history.json`, `config.json`, and `audio/`, and no new-layout files
- WHEN the storage service is initialized with default roots
- THEN all three items exist under the new roots, are gone from the legacy directory, and the legacy directory is removed

#### Scenario: Migration is idempotent
- GIVEN the new layout is fully populated and the legacy directory is absent
- WHEN the storage service initializes repeatedly
- THEN no files move and no errors are logged

#### Scenario: Partial migration completes on next launch
- GIVEN a previous run moved `history.json` but left `audio/` and `config.json` in the legacy directory
- WHEN the storage service initializes
- THEN the remaining items move to their new roots and previously migrated items stay untouched

#### Scenario: Migration failure does not prevent startup
- GIVEN a legacy item that cannot be moved (e.g. permission denied)
- WHEN the storage service initializes
- THEN the failure is logged, the unmoved item stays in the legacy directory, and the app starts normally with whatever reached the new roots

### Requirement: Corrupted Config Recovery

If `config.json` cannot be parsed as JSON, the storage service SHALL rename it to
`config.json.bak` and return the default configuration. If `config.json` cannot be
read at all, the service SHALL log a warning and return the default configuration.

#### Scenario: Corrupted config falls back to defaults with backup
- GIVEN a `config.json` containing invalid JSON
- WHEN the configuration is loaded
- THEN the default configuration is returned, the original file is preserved as `config.json.bak`, and no `config.json` remains at its original path until the next save

#### Scenario: Unreadable config returns defaults
- GIVEN a `config.json` that fails to read at the IO level
- WHEN the configuration is loaded
- THEN the default configuration is returned and a warning is logged
