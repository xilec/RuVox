# windows-installer Specification

## Purpose

Defines the Windows distribution artifact: an NSIS installer for Windows
10 22H2+ / 11 (x86_64) that delivers WebView2 when missing and carries
all runtime resources the app needs.

## Requirements

### Requirement: NSIS installer for supported Windows versions

The project SHALL produce a single NSIS (`.exe`) installer targeting
Windows 10 22H2+ and Windows 11 on x86_64, built with `cargo tauri build`
on a Windows runner. No MSI package is produced. The installer SHALL
install per-user without requiring administrator rights.

#### Scenario: Install on a clean Windows 10 22H2 machine

- GIVEN a Windows 10 22H2 x86_64 machine without RuVox
- WHEN the user runs the NSIS installer with default options
- THEN RuVox installs per-user and launches successfully

### Requirement: WebView2 bootstrap

The installer SHALL embed the WebView2 bootstrapper
(`webviewInstallMode = embedBootstrapper`): when the WebView2 runtime is
absent, the installer downloads and installs it during setup. On machines
that already have WebView2 the installer SHALL NOT reinstall it.

#### Scenario: Machine without WebView2

- GIVEN a Windows machine without the WebView2 runtime
- WHEN the installer runs (with network access)
- THEN WebView2 is installed as part of setup and the app starts

#### Scenario: Machine with WebView2 present

- GIVEN a Windows machine that already has the WebView2 runtime
- WHEN the installer runs
- THEN setup completes without reinstalling WebView2

### Requirement: Bundled runtime resources

The installed app directory SHALL contain: `mpv/` (mpv.exe, its DLLs, and
the mpv LICENSE file), `onnxruntime.dll` next to the app executable, and
`espeak-ng-data/`. Third-party binaries SHALL be downloaded at build time
from pinned URLs with sha256 verification. The ttsd Python sidecar SHALL
NOT be included.

#### Scenario: Playback without preinstalled mpv

- GIVEN a Windows machine without mpv in PATH
- WHEN the user plays a synthesized entry
- THEN the bundled mpv plays the audio

#### Scenario: Piper Russian phonemization

- GIVEN a fresh install on Windows
- WHEN the user synthesizes Russian text with a Piper voice
- THEN phonemization uses the bundled espeak-ng-data (correct Russian
  stress)

#### Scenario: No ttsd in the installation

- GIVEN an installed RuVox on Windows
- WHEN the installation directory is inspected
- THEN no Python/`uv`/ttsd components are present, and the Silero (ttsd)
  engine reports unavailable with a Russian reason

### Requirement: Uninstall data cleanup

The installed app SHALL keep its user data (config, history, audio
cache, downloaded voices) outside the installation directory. When the
user checks "Delete the application data" in the NSIS uninstaller, the
app's data root (`%LOCALAPPDATA%\<bundle identifier>`) SHALL be removed.
When the checkbox is not checked, the data root SHALL be preserved.

#### Scenario: Uninstall with data deletion

- GIVEN an installed RuVox on Windows with synthesized audio and
  downloaded voices
- WHEN the user uninstalls with "Delete the application data" checked
- THEN the install dir AND `%LOCALAPPDATA%\com.ruvox.app` are removed

#### Scenario: Uninstall keeping data

- GIVEN an installed RuVox on Windows with user data
- WHEN the user uninstalls without "Delete the application data"
- THEN the install dir is removed but the data root survives

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
