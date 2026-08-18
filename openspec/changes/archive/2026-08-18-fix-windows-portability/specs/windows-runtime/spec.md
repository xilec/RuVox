# Delta: windows-runtime

## Purpose

Defines how the backend adapts to Windows at runtime: startup environment,
per-OS subprocess cleanup semantics, and which engines and helper
subprocesses exist on Windows.

## ADDED Requirements

### Requirement: Windows startup environment

On Windows, before any worker threads or TTS engines are initialized, the
application SHALL point `PIPER_ESPEAKNG_DATA_DIRECTORY` at the bundled
`espeak-ng-data/` resource directory so that Piper phonemization loads full
Russian dictionaries. On other platforms the startup sequence SHALL NOT
modify process environment for this purpose (the Linux/nix wrapper already
sets it).

#### Scenario: Windows startup with bundled data

- GIVEN the app runs on Windows and the `espeak-ng-data/` resource exists
- WHEN the application starts
- THEN `PIPER_ESPEAKNG_DATA_DIRECTORY` points at the bundled directory
  before any Piper synthesis is attempted

#### Scenario: Non-Windows startup

- GIVEN the app runs on Linux
- WHEN the application starts
- THEN the startup code does not set `PIPER_ESPEAKNG_DATA_DIRECTORY`

### Requirement: Per-OS subprocess cleanup

Orphan mpv reaping based on `/tmp` socket names and `/proc` inspection
SHALL run on Unix platforms only. On Windows the application MUST NOT
attempt `/proc`-based reaping (tauri-plugin-mpv uses named pipes there),
and startup MUST NOT fail due to the absence of that cleanup.

#### Scenario: Windows startup without reaping

- GIVEN the app runs on Windows
- WHEN the application starts
- THEN no orphan-mpv reaping is attempted and startup proceeds normally

### Requirement: Engines and helper subprocesses on Windows

The Windows build SHALL ship Piper and Silero Native as the available TTS
engines. The ttsd (Python/`uv`) subprocess SHALL NOT be required or
bundled on Windows, and no Windows code path SHALL assume `uv` is
installed.

#### Scenario: Engine set on Windows

- GIVEN the app runs on a fresh Windows installation without `uv`
- WHEN the application starts and TTS is used
- THEN Piper and Silero Native work, and no attempt to spawn `uv` blocks
  or breaks startup
