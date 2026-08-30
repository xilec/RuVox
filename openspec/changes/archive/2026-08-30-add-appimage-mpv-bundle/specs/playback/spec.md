# Playback — Per-OS mpv executable resolution (delta)

## MODIFIED Requirements

### Requirement: Per-OS mpv executable resolution

The player SHALL resolve the mpv executable per platform. On Windows it
SHALL prefer the bundled `mpv/mpv.exe` inside the app's resource directory
and fall back to a PATH lookup when the bundled executable is absent. On
Linux it SHALL prefer the bundled `mpv/mpv` located through the same
install-layout search the other bundled Linux resources use (the
executable's own directory, its parent directory, and the sibling product
directories under the parent's `lib/` directory, with the application's
own `RuVox` product directory tried before any wildcard match), and fall
back to a PATH lookup when no bundled executable is present (dev runs,
Nix builds, `.deb` installs relying on the system player). A bundled
candidate that exists but is empty or a zero-byte placeholder SHALL be
rejected in favor of the PATH lookup. A missing executable SHALL surface
as a typed player error, not a panic.

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

#### Scenario: Linux with bundled mpv (AppImage)

- GIVEN the app runs on Linux from a build that bundles `mpv/mpv` under
  its resource directory (the AppImage layout
  `<exe_dir>/../lib/RuVox/mpv/mpv` included)
- WHEN the player initializes
- THEN the bundled executable is spawned

#### Scenario: Linux resolution unchanged

- GIVEN the app runs on Linux and no bundled `mpv/mpv` exists (dev
  checkout, Nix build, `.deb` without the resource)
- WHEN the player initializes
- THEN `mpv` is resolved from PATH exactly as before

#### Scenario: Linux placeholder bundle falls back to PATH

- GIVEN the app runs on Linux and the only bundled candidate
  (`<lib/*/RuVox>/mpv/mpv`) exists but is zero bytes
- WHEN the player initializes
- THEN `mpv` is resolved from PATH, never the placeholder
