# Design: linux-runtime-bundling

## Context

`ort` is built with `load-dynamic` + `disable-linking`, so the binary has no
ONNX Runtime linkage at all: at engine init it dlopens the dylib named by
`ORT_DYLIB_PATH`. The tauri resources map only had a Windows entry
(`onnxruntime.dll`), and Linux packages shipped a zero-byte placeholder — so on
a clean system Silero Native hung silently inside ONNX session creation with no
error surfaced. Piper independently needs `espeak-ng-data/`; nix builds got it
from a wrapper, packaged builds did not.

## Goals / Non-Goals

- Goal: packaged Linux installs work out of the box (both engines usable)
  without system-wide ONNX Runtime or espeak data.
- Goal: dev and nix behavior unchanged; wrapper-provided env keeps priority.
- Non-goal: vendoring ONNX Runtime into the git repo (fetched at build time,
  pinned + checksummed).
- Non-goal: changing engine selection/fallback logic.

## Decisions

### Decision: resolve paths in-process at startup, not in wrapper scripts
`.deb`/AppImage launches go through different entry points (desktop file,
AppRun, direct binary); a startup lookup is the one place that covers all of
them. `init_platform_env` runs before engines initialize and skips any variable
already present in the environment.

### Decision: product-dir priority over sibling wildcard
The `lib/` sibling scan (`/usr/lib/*/espeak-ng-data`) can match other products'
files alphabetically before ours. The scan now promotes the `RuVox` product dir
to the front of the candidate list; covered by unit tests for both lookups.
A foreign `libonnxruntime.so` would fail dlopen against the pinned ort-sys ABI
instead of degrading gracefully, which makes shadowing not just wrong but
user-visible.

### Decision: reject zero-byte placeholders explicitly
A placeholder that "exists" would pass an existence check and poison
`ORT_DYLIB_PATH`. Candidates are size-checked; placeholders are skipped.

### Decision: pinned fetch with checksum, shared by CI and local builder
`scripts/fetch-linux-onnxruntime.sh` pins version 1.24.1 (matching the
`ort-sys` API) plus sha256, supports `--check`, and is called identically from
the release workflow and `scripts/build-linux-packages.sh` so the two cannot
drift.

## Risks / Trade-offs

- A future `ort` upgrade must bump the pin in lockstep — enforced by review;
  mismatch fails loudly at first synthesis, not silently at package build.
- AppImage squashfs is zstd, which stock p7zip cannot extract; repack uses
  `unsquashfs -o <offset>` (see `scripts/fix-appimage-wayland.sh`).

## Migration Plan

Backfill change: behavior already implemented, VM-verified end-to-end for both
package formats (engine load from bundled paths, synthesis, playback on pure
Wayland). Artifacts document it; archiving syncs the specs.
