# Delta: playback

## ADDED Requirements

### Requirement: Per-OS mpv executable resolution

The player SHALL resolve the mpv executable per platform. On Windows it
SHALL prefer the bundled `mpv/mpv.exe` inside the app's resource directory
and fall back to a PATH lookup when the bundled executable is absent; on
Linux it SHALL keep resolving `mpv` from PATH. A missing executable SHALL
surface as a typed player error, not a panic.

#### Scenario: Windows with bundled mpv

- GIVEN the app runs on Windows and `mpv/mpv.exe` exists in the resource
  directory
- WHEN the player initializes
- THEN the bundled executable is spawned

#### Scenario: Windows without bundled mpv

- GIVEN the app runs on Windows and no bundled `mpv/mpv.exe` exists
- WHEN the player initializes
- THEN `mpv` from PATH is used if present, otherwise a typed error is
  returned

#### Scenario: Linux resolution unchanged

- GIVEN the app runs on Linux
- WHEN the player initializes
- THEN `mpv` is resolved from PATH exactly as before
