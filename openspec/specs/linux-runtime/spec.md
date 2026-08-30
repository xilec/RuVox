# linux-runtime Specification

## Purpose
Covers the Linux runtime bundling for the packaged builds (.deb, AppImage): how the pinned ONNX Runtime, espeak-ng data, and Piper voice resources are placed next to the installed binary, how the app resolves them at startup outside the Nix dev shell, and what the packages must carry so a clean distro install synthesizes speech without extra setup.

## Requirements

### Requirement: Linux startup environment for bundled TTS resources

On Linux, before any worker threads or TTS engines are initialized, the
startup sequence SHALL locate bundled runtime resources and, when found, set
the corresponding process environment variables:

- `espeak-ng-data/` (Piper phonemization) → `PIPER_ESPEAKNG_DATA_DIRECTORY`
- `libonnxruntime.so` (Silero Native ONNX Runtime dylib) → `ORT_DYLIB_PATH`

The lookup SHALL examine the executable's own directory, its parent directory,
and the sibling product directories under the parent's `lib/` directory. The
application's own product directory (`RuVox`) SHALL be tried before any
wildcard match over siblings, so another product's files can never shadow the
bundled resources. A candidate that exists but is a zero-byte placeholder
SHALL be rejected.

Pre-existing values of these variables in the process environment SHALL NOT
be overridden (a nix wrapper or the user keeps priority). If no bundled
resources are found — e.g. a dev build run via `cargo tauri dev` — startup
MUST proceed unchanged with the variables left unset.

#### Scenario: Deb install synthesizes from bundled resources

- GIVEN the app was installed from a Linux `.deb` package on a clean system
  without a system-wide `libonnxruntime`
- WHEN the application starts and Silero Native synthesis is requested
- THEN `PIPER_ESPEAKNG_DATA_DIRECTORY` points at `/usr/lib/RuVox/espeak-ng-data`,
  `ORT_DYLIB_PATH` points at `/usr/lib/RuVox/libonnxruntime.so`, and synthesis
  completes without falling back to another engine

#### Scenario: Own product dir wins over sibling products

- GIVEN `lib/RuVox/espeak-ng-data` and `lib/<Other>/espeak-ng-data` both exist
  next to the executable
- WHEN the startup lookup resolves the bundled data directory
- THEN the `RuVox` copy is selected regardless of alphabetical order

#### Scenario: Nix build keeps wrapper-provided values

- GIVEN the app runs as a nix build with `PIPER_ESPEAKNG_DATA_DIRECTORY` and
  `ORT_DYLIB_PATH` already set by the wrapper
- WHEN the application starts
- THEN both values are preserved exactly as provided

#### Scenario: Dev build without bundles starts normally

- GIVEN the app runs from a dev checkout without any bundled resource
  directories
- WHEN the application starts
- THEN neither variable is set and engine selection behaves as before this
  capability existed

### Requirement: Linux packages bundle pinned runtime resources

Linux release packages (`.deb`, `.AppImage`) SHALL include as resources:
`espeak-ng-data/` and `libonnxruntime.so` fetched at a pinned upstream version
whose API matches the `ort-sys` crate ABI, verified by checksum. The release
workflow and the local Docker builder SHALL fetch it the same way.

#### Scenario: Package contents

- GIVEN a Linux package built by the release workflow or the local builder
- WHEN its payload is inspected
- THEN `usr/lib/RuVox/espeak-ng-data/` and `usr/lib/RuVox/libonnxruntime.so`
  are present and non-empty

### Requirement: Linux packages provide the mpv player

Linux release packages SHALL make the `mpv` player available to the app
without manual user setup, per package model:

- The `.deb` SHALL declare `mpv` in its `Depends` field so the system
  package manager installs the player with the app.
- The `.AppImage` SHALL bundle the `mpv` player executable together with
  its non-core shared-library closure under the `mpv/` bundle resource
  directory. The bundle SHALL be assembled from pinned Ubuntu noble
  `.deb` packages verified against a sha256 manifest, and every shipped
  file SHALL carry `RPATH=$ORIGIN` so the player resolves its libraries
  from its own directory without environment setup. Core libraries
  (glibc family, `libstdc++`, `libgcc_s`) SHALL NOT be bundled — the host
  provides them.

The release workflow and the local Docker builder SHALL fetch and
assemble the bundle the same way (shared fetch script). Dev builds and
Nix builds bundle nothing: the player resolution falls back to the PATH
lookup, and the Nix wrapper keeps providing `mpv` via PATH.

#### Scenario: AppImage payload carries the player

- GIVEN an AppImage built by the release workflow (or the local Docker
  builder) after the mpv fetch step
- WHEN its payload is inspected
- THEN the `mpv/` resource directory contains a non-empty `mpv` ELF
  executable and the non-core shared libraries it DT_NEEDs, and the
  executable carries an `RPATH` of `$ORIGIN`

#### Scenario: Deb payload is unchanged

- GIVEN a `.deb` built by the release workflow
- WHEN its payload and control file are inspected
- THEN it contains no bundled `mpv/` resource directory and its `Depends`
  field includes `mpv`

#### Scenario: Fetch is reproducible and fails loudly

- GIVEN the fetch script's pinned manifest of `.deb` filenames and
  sha256 checksums
- WHEN the fetch step downloads each file from the pinned archive URL
- THEN every checksum is verified and any mismatch or failed download
  aborts the build before `tauri build` runs
