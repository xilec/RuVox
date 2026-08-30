# Linux Runtime — mpv player provisioning (delta)

## ADDED Requirements

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
