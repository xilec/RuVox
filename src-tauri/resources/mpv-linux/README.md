# mpv-linux — placeholder tree

Compile-time placeholder for the `resources/mpv-linux → mpv` bundle
resource referenced by `src-tauri/tauri.appimage.conf.json` (AppImage
builds only — the `.deb` depends on the system `mpv` instead, #266). The
real bundle (the pinned Ubuntu noble `mpv` binary plus its non-core
shared-library closure, every file RPATH'd to `$ORIGIN`) is downloaded
and assembled at release-build time by `scripts/fetch-linux-mpv.sh`
(pinned launchpad `.deb` files, sha256-verified).

Never bundle a build where this is still the placeholder: the player
resolution falls back to a PATH lookup when the bundled `mpv` binary is
absent, so a placeholder build silently uses the system player —
`--check` mode of the fetch script guards release builds.
