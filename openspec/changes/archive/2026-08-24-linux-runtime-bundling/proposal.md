## Why

The Linux release packages (`.deb` and `.AppImage`) shipped without the runtime
pieces the TTS engines need at startup: `espeak-ng-data/` (required by Piper
phonemization) and a real `libonnxruntime.so` (required by the `ort` crate,
which is built with `load-dynamic` and dlopens the dylib named by
`ORT_DYLIB_PATH`). On a fresh install Silero Native therefore hung silently at
ONNX session creation and Piper had no phonemization data, so no engine could
synthesize. The Nix build never hit this because its wrapper sets these paths —
but end users do not run Nix.

## What Changes

- Linux packages bundle `espeak-ng-data/` and a pinned `libonnxruntime.so`
  (1.24.1, matching the `ort`/`ort-sys` ABI) as resources, for both `.deb`
  (`/usr/lib/RuVox/`) and `.AppImage` (squashfs `usr/lib/RuVox/`).
- On Linux, before engines initialize, startup locates the bundled directories
  and sets `PIPER_ESPEAKNG_DATA_DIRECTORY` and `ORT_DYLIB_PATH` in the process
  environment. The lookup prefers our own product dir over sibling wildcard
  matches, rejects zero-byte placeholder files, and leaves any pre-existing
  environment values untouched (the nix wrapper keeps priority).
- If nothing is bundled or found (e.g. a dev build), startup proceeds exactly
  as before; the variables are simply not set.
- CI (release workflow) and the local Docker builder both fetch the pinned
  ONNX Runtime with checksum verification and apply the wayland AppImage fix.

This is a backfill change: the behavior described here is already implemented
and verified end-to-end in a VM (engine load from bundled paths, synthesis and
playback for both package formats on pure Wayland); the artifacts document it
and sync the specs.

## Capabilities

### New Capabilities

- `linux-runtime`: how the backend adapts to Linux at runtime — startup
  environment for bundled TTS runtime resources (`espeak-ng-data`,
  `libonnxruntime`), the lookup order and guards, and what happens when they
  are absent.

### Modified Capabilities

- `windows-runtime`: the "Non-Windows startup" scenario under "Windows startup
  environment" stated that the startup code does not set
  `PIPER_ESPEAKNG_DATA_DIRECTORY` on Linux. With Linux bundling this is no
  longer true in general; the requirement is reworded to allow the Linux
  startup path to set it when bundled data ships with the package (nix builds
  remain covered by the wrapper).

## Impact

- `src-tauri/src/lib.rs` (`init_platform_env`, `find_bundled_espeak_data`,
  `find_bundled_onnxruntime` + unit tests)
- `src-tauri/tauri.linux.conf.json` / `tauri.windows.conf.json` (resources map)
- `scripts/fetch-linux-onnxruntime.sh`, `scripts/build-linux-packages.sh`,
  `scripts/docker/Dockerfile`, `scripts/fix-appimage-wayland.sh`
- `.github/workflows/release.yml` (linux-packages job)
