# Design: add-appimage-mpv-bundle

## Context

`tauri-plugin-mpv` controls a spawned `mpv` subprocess over JSON IPC; the
Linux resolver used to return plain `mpv` (PATH). The v0.5.0 AppImage
panics on a clean system (#265). The `.deb` already solves this via
`Depends: mpv` (#266); the AppImage cannot declare dependencies — it must
carry the player. Any bundled binary needs its shared libraries inside the
AppImage, because the host may not have `libmpv2`/`libav*` at all.

The Ubuntu 24.04 `mpv` package is built "with everything": the binary and
`libmpv2` DT_NEED ~215 non-core shared libraries (the full ffmpeg/vo/va
world). Ubuntu enables `-z now`, so every DT_NEEDED entry must resolve at
process start — the closure cannot be trimmed below "what the loader
needs".

## Goals / Non-goals

- Goal: AppImage runs and plays on a clean system with no system `mpv`,
  using the same distro-trusted player the `.deb` pulls in.
- Goal: `.deb`, Nix builds, and dev runs behave exactly as before.
- Non-goal: shrinking the bundle below the loader closure; runtime env
  manipulation; source-built mpv.

## Decisions

### D1: distro mpv + DT_NEEDED closure, not a slim source build

Bundle Ubuntu's `mpv` binary plus the transitive DT_NEEDED closure of
non-core libraries (glibc family and `libstdc++`/`libgcc_s` excluded —
AppImage core libs the host always provides). A minimal audio-only mpv
built from source (~20 MB instead of ~260 MB unpacked) was rejected: it
adds a vendored ffmpeg+mpv build pipeline, CVE/rebuild maintenance, and a
supply chain the project does not otherwise carry. The size cost is
accepted for 0.5.1 (the Windows installer already ships its mpv); if it
proves unacceptable, a slim build can be revisited as its own change.

### D2: pinned launchpad `.deb` files + sha256 manifest

`scripts/fetch-linux-mpv.sh` embeds the exact list of 202
(`filename, sha256`) pairs captured from `ubuntu:24.04` with the current
`noble` versions (mpv `0.37.0-1ubuntu4`, ffmpeg `7:6.1.1-3ubuntu5`, …) and
downloads from
`https://launchpad.net/ubuntu/+archive/primary/+files/<name>` — launchpad
keeps every published version, so the pins are stable (epoch prefix
`N%3a` stripped for the URL; verified 202/200-OK). Rejected:
`apt-get download` at build time (unpinned, versions drift, apt throttles
burst downloads). The manifest lives in the script like the pinned
onnxruntime URL+sha does. When Ubuntu moves and we want a newer mpv, the
list is regenerated with the documented docker one-liner; a sha mismatch
fails the fetch loudly instead of shipping a silent drift.

### D3: AppImage-only scope via a CLI config overlay

`tauri.linux.conf.json` resources are shared by the `.deb` and AppImage
builds, which run from the same config — adding the mpv resource there
would bloat the `.deb` and contradict the approved #266 model. Instead the
resource mapping (`"resources/mpv-linux" → "mpv"`) lives in a new
`src-tauri/tauri.appimage.conf.json` overlay applied only by the AppImage
build invocation: `pnpm tauri build --bundles appimage --config
src-tauri/tauri.appimage.conf.json`. Tauri merges the CLI overlay over the
base + platform configs (resources maps merge per key). The `.deb` build
stays `--bundles deb` with no overlay; Nix keeps invoking `cargo tauri
build --bundles deb` and never sees the overlay.

### D4: `RPATH=$ORIGIN` on every shipped file, not LD_LIBRARY_PATH

`tauri-plugin-mpv` spawns the child with an inherited environment; there
is no config knob to inject `LD_LIBRARY_PATH`, and AppRun environment games
are fragile. Each bundled file gets `patchelf --set-rpath '$ORIGIN'` —
including the libraries, because modern linkers emit `DT_RUNPATH`, which
covers only the object's own direct dependencies; setting it on every file
makes the whole flat directory self-resolving regardless of load order.
The fetch script applies it at assemble time; `patchelf` is already a
dependency of the AppImage bundling in CI and in the Docker builder.

### D5: discovery reuses the existing install-layout search

`find_bundled_espeak_data` / `find_bundled_onnxruntime` already encode the
`.deb`/AppImage resource layout (`<exe_dir>/rel`, `<exe_dir>/../rel`,
`<exe_dir>/../lib/*/rel` with `RuVox` priority and non-empty checks) and
are VM-proven inside the AppImage. They are generalized into one
`find_bundled(exe_dir, rel, pred)` walker; `find_bundled_mpv` looks up
`mpv/mpv` (non-empty file). The non-Windows `resolve_mpv_path` prefers the
bundle and falls back to PATH — dev and Nix keep working with zero
configuration. Rejected: `resource_dir()`-based lookup — the exe-dir
layout search is what already works inside the squashfs-mounted AppImage.

### D6: alternatives-managed sonames are flattened from their subdirs

Ubuntu ships `libblas.so.3` / `liblapack.so.3` as `update-alternatives`
symlinks that only exist after `postinst` runs — the `.deb` payload keeps
the real files in `usr/lib/x86_64-linux-gnu/{blas,lapack}/`, and
`libpulse0` keeps `libpulsecommon` in `usr/lib/x86_64-linux-gnu/pulseaudio/`.
The assembler therefore flattens those three subdirs into the bundle root
(next to `$ORIGIN`), which the loader resolves via D4. A strict loader
gate on the assembled bundle — a clean-environment `ldd` sweep that fails
when any non-core dependency resolves `not found` OR outside the bundle —
runs inside the fetch script, so a missing or system-borrowed library
fails the build instead of the user's playback (build images carrying
`libmpv-dev` would otherwise mask gaps with system libraries).

## Risks / Trade-offs

- **AppImage size ~97 MB → ~250 MB.** Accepted (D1); every Linux user
  downloads the player once, updates re-download it (no delta updates).
- **Noble glibc floor.** The bundled libraries need glibc ≥ 2.39 — the
  same floor as the main binary (built on the noble CI runner), so the
  supported-host set does not change.
- **Pinned archive churn.** Ubuntu can silently drop/replace pinned
  launchpad files only in extraordinary cases; sha verification fails the
  release build loudly, and the regeneration recipe is documented in the
  fetch script header.
- **Player/library skew.** The bundled mpv is frozen until re-pinned;
  playback of locally synthesized opus is stable across mpv 0.37.x. The
  re-pin procedure (docker one-liner) is in the script header.
