# Proposal: add-appimage-mpv-bundle

## Why

Issue #265: the v0.5.0 `.AppImage` does not ship or depend on the `mpv`
player, so on a clean system the app panics on startup (`mpv init failed …
Is mpv installed and in your PATH?`). The maintainer-approved hybrid fix
shipped the `.deb` half in v0.5.0 (PR #266: `mpv` added to `Depends`); this
change implements the deferred AppImage half for 0.5.1: **bundle `mpv` into
the AppImage** so it works out of the box like the Windows NSIS installer
does.

## What Changes

- The release AppImage bundles a pinned Ubuntu 24.04 (`noble`) `mpv` player:
  the `/usr/bin/mpv` binary plus its non-core shared-library closure
  (DT_NEEDED transitive closure, glibc family excluded), fetched at
  release-build time from pinned launchpad `.deb` files verified by a
  sha256 manifest, and shipped as the `mpv/` bundle resource next to the
  existing `espeak-ng-data` / `libonnxruntime.so` resources.
- Every shipped mpv file carries `RPATH=$ORIGIN` (patchelf), so the player
  finds its libraries in its own directory without any environment setup —
  `tauri-plugin-mpv` spawns the subprocess with an inherited environment.
- The player's non-Windows mpv resolution (`src-tauri/src/player/mod.rs`)
  now prefers the bundled `mpv/mpv` found via the same install-layout search
  the espeak-ng-data / libonnxruntime lookups use (executable dir, parent,
  parent's `lib/*/` with `RuVox` priority), and falls back to a PATH lookup
  (dev runs, Nix wrapper, system installs) — mirroring the existing Windows
  bundled/PATH behavior.
- The mpv resource is wired through a **new CLI config overlay**
  (`src-tauri/tauri.appimage.conf.json`) applied only to AppImage builds
  (`tauri build --bundles appimage --config …`), so the `.deb` — which stays
  on the approved `Depends: mpv` model of #266 — is byte-for-byte unaffected.
- The release workflow, the local Docker builder, and a new
  `scripts/fetch-linux-mpv.sh` implement the fetch/assemble step the same
  way the pinned libonnxruntime fetch already works.

## Impact

- **Affected specs:** `openspec/specs/linux-runtime/spec.md` (ADDED
  requirement: Linux packages provide the mpv player);
  `openspec/specs/playback/spec.md` (MODIFIED requirement: Per-OS mpv
  executable resolution — Linux gains the bundled-first lookup).
- **Affected code:** `src-tauri/src/lib.rs` (generic bundled-resource
  lookup shared by espeak/onnx/mpv + `find_bundled_mpv` + tests);
  `src-tauri/src/player/mod.rs` (non-Windows resolver + tests);
  new `src-tauri/tauri.appimage.conf.json`; new
  `scripts/fetch-linux-mpv.sh`; `scripts/build-linux-packages.sh`;
  `.github/workflows/release.yml`; `src-tauri/resources/README.md`;
  `docs/install.md`; `CHANGELOG.md`.
- **Accepted cost:** the Ubuntu `mpv` links the full ffmpeg/vo world, so
  the AppImage grows from ~97 MB to roughly ~250 MB (260 MB unpacked
  bundle). This is the same "ship the player" model the Windows installer
  has used since 0.3.x.
- **Out of scope / non-goals:** changing the `.deb` (stays `Depends: mpv`
  per the #265/#266 decision); building a slim audio-only mpv from source
  (rejected for maintainer/supply-chain burden, see design D1); Windows
  packaging; updater flow changes (the AppImage is re-signed after the
  existing wayland repack, unchanged).

## Non-goals

- No slimming of the bundled closure below "everything the loader needs at
  startup" (Ubuntu hardening `-z now` makes removing DT_NEEDED entries
  unsafe).
- No runtime PATH manipulation or wrapper scripts around the bundled mpv.
- No changes to the Nix packages (the overlay is CLI-scoped; the nix build
  keeps its PATH-provided mpv).
