# Delta: tray

## ADDED Requirements

### Requirement: Tray tooltip shows the app version

The tray icon tooltip SHALL include the application version, formatted as
`RuVox v<version>` (e.g. `RuVox v0.3.1`), so bug reports can quote the version
directly from the tray. The version SHALL be read from the app package info
(`app.package_info().version`, the `version` field of `tauri.conf.json`) at tray
init.

#### Scenario: Tooltip names the version

- GIVEN the application is running
- WHEN the user hovers the tray icon
- THEN the tooltip reads `RuVox v<version>` matching the built `tauri.conf.json`
  version, not the bare `RuVox`
