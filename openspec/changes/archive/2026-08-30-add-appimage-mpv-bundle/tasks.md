# Tasks: add-appimage-mpv-bundle

## 1. Runtime resolution (Rust)

- [x] 1.1 `src-tauri/src/lib.rs`: generalize the bundled-resource lookup —
      extract `find_bundled(exe_dir, rel, pred)` from the duplicated
      espeak/onnxruntime walkers (layout candidates, `RuVox` priority,
      predicate-based acceptance); re-express both existing helpers on top
      of it; add `find_bundled_mpv` (`mpv/mpv`, non-empty file).
- [x] 1.2 `src-tauri/src/player/mod.rs`: non-Windows `resolve_mpv_path`
      prefers the bundled `mpv/mpv` via `find_bundled_mpv` (exe-dir based),
      falls back to PATH; update the doc comments.
- [x] 1.3 Tests: `lib.rs` — bundled mpv found in flat and deb/AppImage
      layouts, `RuVox` product dir wins over siblings, absent → `None`,
      zero-byte placeholder → `None`; `player/mod.rs` — PATH fallback
      cases hold, placeholder-only bundle → PATH.

## 2. Packaging

- [x] 2.1 `scripts/fetch-linux-mpv.sh`: pinned manifest (202 noble debs,
      sha256) downloaded from launchpad (epoch prefix stripped in URLs);
      assemble `src-tauri/resources/mpv-linux/` — `mpv` binary + non-core
      lib closure flattened from `x86_64-linux-gnu` top level plus the
      `blas/`, `lapack/`, `pulseaudio/` subdirs; `patchelf --set-rpath
      $ORIGIN` on every file; `--check` mode gates release builds
      (ELF-ness of the binary). Regeneration recipe documented in the
      header.
- [x] 2.2 `src-tauri/tauri.appimage.conf.json`: AppImage-only overlay
      mapping `resources/mpv-linux → mpv`.
- [x] 2.3 `.github/workflows/release.yml`: split the Linux build into
      `--bundles deb` (no overlay) and `--bundles appimage` (overlay);
      run the mpv fetch before the AppImage build; keep the wayland repack
      and signing flow unchanged.
- [x] 2.4 `scripts/build-linux-packages.sh`: mirror the split (deb without
      the overlay, AppImage with it); run the mpv fetch with the same
      `--check || fetch` caching pattern as the onnxruntime fetch.
- [x] 2.5 `src-tauri/resources/README.md`: document the `mpv-linux/`
      placeholder tree.

## 3. Docs

- [x] 3.1 `docs/install.md`: AppImage no longer requires a system `mpv`
      (only the `.deb`'s apt dependency and from-source builds do).
- [x] 3.2 `CHANGELOG.md`: `[Unreleased]` note — AppImage bundles the mpv
      player (no system install needed); `.deb` unchanged.

## 4. Validation

- [x] 4.1 `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml`
      green (incl. new lookup/resolver tests).
- [x] 4.2 `nix develop -c just lint` green.
- [x] 4.3 Docker builder: `scripts/build-linux-packages.sh` produces both
      packages; AppImage payload contains `usr/lib/RuVox/mpv/mpv` + libs;
      `.deb` payload has no `mpv/` and `Depends: mpv` intact.
- [x] 4.4 VM pass (clean Ubuntu 24.04 overlay from the golden disk, no
      system mpv): the AppImage starts without a panic, `/proc/<pid>/exe`
      of the spawned player points inside the extracted AppImage
      (`…/usr/lib/RuVox/mpv/mpv`), and that bundled player plays a test
      wav headlessly; no mpv init errors in the app log. `.deb`
      regression: `apt-get install ./RuVox_0.5.0_amd64.deb` pulls system
      mpv 0.37.0-1ubuntu4 via `Depends`, the app starts and the spawned
      player exe is `/usr/bin/mpv`. (GUI synthesis→playback e2e untouched
      by this change; the bundled player is exercised directly.)
