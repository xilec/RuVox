# Delta: windows-installer

## ADDED Requirements

### Requirement: Installer kills the app's orphaned mpv

The NSIS installer SHALL, before copying or removing files
(`NSIS_HOOK_PREINSTALL` / `NSIS_HOOK_PREUNINSTALL`), terminate any
`mpv.exe` process whose executable path is under the install directory.
Processes outside the install directory (e.g. a standalone mpv player)
MUST NOT be touched.

Rationale: the app spawns mpv via tauri-plugin-mpv; when the installer
force-kills the app, the exit-time mpv cleanup never runs and the orphan
locks `mpv\mpv.exe`, failing the install (#211).

#### Scenario: Update install with a running mpv

- GIVEN RuVox is installed and its mpv subprocess is running
- WHEN the installer (auto-update or manual reinstall) runs
- THEN the in-dir mpv.exe is terminated before file copying AND the
  install completes without a "file in use" error

#### Scenario: Standalone mpv is left alone

- GIVEN a mpv.exe process running from a directory outside the install
  dir
- WHEN the installer runs
- THEN that process is still running afterwards
