# auto-update Delta

## MODIFIED Requirements

### Requirement: Update availability check

The application SHALL check for updates against GitHub releases using
tauri-plugin-updater on installs the updater can serve: Windows installs,
and Linux installs running from an AppImage. When an update is found, the
user SHALL be notified in the UI (Russian-language notification) and can
trigger the install; the startup check failure (e.g. offline) SHALL be
silent — no error dialogs for transient network problems. On installs the
updater cannot serve (Linux .deb/nix packages), the update check SHALL be
disabled entirely: no startup check, no update section in Settings, and no
error surfaced to the user.

#### Scenario: Update available

- GIVEN a newer version is published on GitHub releases
- WHEN the app checks for updates on an install the updater serves
  (Windows, or Linux running from an AppImage)
- THEN the user is notified in Russian and can start the update

#### Scenario: Offline check

- GIVEN the machine has no network access
- WHEN the app checks for updates at startup
- THEN the check fails silently and the app continues normally

#### Scenario: Linux install without self-update support

- GIVEN the app is installed from a .deb package or via nix on Linux
- WHEN the app starts or the user opens Settings
- THEN no update check runs, no update section is shown, and no error is
  reported

### Requirement: Signed update installation

Updates SHALL be installed from the updater artifacts attached to the
GitHub release — the NSIS installer on Windows, the AppImage on Linux —
and the updater SHALL verify the artifact signature against the public key
embedded in the app before installing. A mismatched or missing signature
MUST abort the update. The published updater manifest SHALL carry an entry
for every platform it advertises, and each entry's signature MUST match the
exact distributed artifact file.

#### Scenario: Valid signature

- GIVEN an updater artifact signed with the project's updater key (the
  NSIS installer on Windows, the AppImage on Linux)
- WHEN the user confirms the update
- THEN the update downloads, verifies, installs, and the app restarts on
  the new version

#### Scenario: Valid signature on Linux AppImage

- GIVEN an AppImage updater artifact signed with the project's updater key
- WHEN the user confirms the update in an AppImage install
- THEN the update downloads, verifies, replaces the running AppImage, and
  the app relaunches on the new version

#### Scenario: Invalid signature

- GIVEN an update artifact whose signature does not match the embedded
  public key
- WHEN the update is attempted
- THEN the installation aborts and the running app is unchanged
