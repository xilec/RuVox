# auto-update Specification

## Purpose

Defines in-app auto-updates: the app discovers new versions from GitHub
releases and installs them with signature verification, so Windows users
stay current without manual downloads.

## Requirements

### Requirement: Update availability check

The application SHALL check for updates against GitHub releases using
tauri-plugin-updater. When an update is found, the user SHALL be notified
in the UI (Russian-language notification) and can trigger the install;
the check failure (e.g. offline) SHALL be silent — no error dialogs for
transient network problems.

#### Scenario: Update available

- GIVEN a newer version is published on GitHub releases
- WHEN the app checks for updates
- THEN the user is notified in Russian and can start the update

#### Scenario: Offline check

- GIVEN the machine has no network access
- WHEN the app checks for updates
- THEN the check fails silently and the app continues normally

### Requirement: Signed update installation

Updates SHALL be installed from the NSIS-based updater artifact attached
to the GitHub release, and the updater SHALL verify the artifact
signature against the public key embedded in the app before installing.
A mismatched or missing signature MUST abort the update.

#### Scenario: Valid signature

- GIVEN an update artifact signed with the project's updater key
- WHEN the user confirms the update
- THEN the update downloads, verifies, installs, and the app restarts on
  the new version

#### Scenario: Invalid signature

- GIVEN an update artifact whose signature does not match the embedded
  public key
- WHEN the update is attempted
- THEN the installation aborts and the running app is unchanged

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
