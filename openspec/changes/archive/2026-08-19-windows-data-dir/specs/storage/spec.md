# Delta: storage

## MODIFIED Requirements

### Requirement: Cache Directory Layout

The system SHALL store all persistent data under a per-user cache root,
with the following layout:

```
<cache_root>/
├── history.json                         # Versioned list of TextEntry records
├── config.json                          # Application configuration (UIConfig)
└── audio/
    ├── {uuid}.opus                      # Ogg-Opus audio (32 kbps VOIP, mono)
```

The cache root is platform-dependent:

- **Windows:** `dirs::data_local_dir()/<bundle identifier>`
  (`%LOCALAPPDATA%\com.ruvox.app`). It MUST NOT coincide with the NSIS
  install dir (`%LOCALAPPDATA%\<productName>`) and MUST match the
  directory the NSIS uninstaller removes via its "Delete the application
  data" checkbox.
- **Other platforms:** `dirs::cache_dir()/ruvox` (e.g. `~/.cache/ruvox/`).

The storage service SHALL create the cache root and the `audio/`
subdirectory on initialization if they do not exist.

#### Scenario: First launch creates the directory tree
- GIVEN the cache directory does not exist
- WHEN the storage service is initialized
- THEN the cache root and its `audio/` subdirectory exist on disk

#### Scenario: Default cache root location
- GIVEN no custom cache directory is configured
- WHEN the storage service is constructed with defaults
- THEN the cache root is the platform per-user location described above
  (`%LOCALAPPDATA%\com.ruvox.app` on Windows, `~/.cache/ruvox/` on Linux)
