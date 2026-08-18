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
