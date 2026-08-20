# Delta: auto-update

## ADDED Requirements

### Requirement: Player is shut down before update installation

Before calling `Update.downloadAndInstall()`, the app SHALL destroy the
mpv subprocess (the `shutdown_player_for_update` command): mark the
player destroyed so in-flight commands short-circuit, then best-effort
destroy the mpv instance — a missing instance (already destroyed when
the main window closed) MUST NOT abort the update.

Rationale: the updater-launched installer force-kills the app, so
`RunEvent::Exit` never fires; without this, the orphaned mpv.exe locks
the install dir (#211).

#### Scenario: Update with a live mpv

- GIVEN the app with a running mpv subprocess
- WHEN the user confirms an update
- THEN mpv is destroyed before the installer starts AND the install
  proceeds without a "file in use" error

#### Scenario: Update with mpv already destroyed

- GIVEN the app whose mpv instance was already destroyed (e.g. the main
  window was closed to tray and the plugin tore mpv down)
- WHEN the user confirms an update
- THEN the destroy attempt fails silently in the log and the update
  proceeds normally
