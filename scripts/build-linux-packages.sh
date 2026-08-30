#!/usr/bin/env bash
# Build the Linux release packages (.deb / .AppImage) locally in Docker.
#
# Usage:
#   scripts/build-linux-packages.sh [deb|appimage|both]     (default: both)
#
# Replicates the release workflow's linux-packages job
# (.github/workflows/release.yml) inside a stock ubuntu:24.04 container:
#
#   1. pnpm install + frontend build (tauri-codegen needs dist/)
#   2. cargo rustc --lib --crate-type=rlib  (espeak-rs-sys emits espeak-ng-data)
#   3. copy espeak-ng-data into src-tauri/resources/
#   4. scripts/fetch-linux-onnxruntime.sh   (pinned libonnxruntime for `ort`)
#   4b. scripts/fetch-linux-mpv.sh          (pinned player bundle, AppImage
#       only — the .deb depends on the system mpv instead, #266)
#   5. pnpm tauri build --bundles deb, then --bundles appimage with the
#      appimage overlay (bundled mpv resource, #265)
#   6. AppImage only: apply scripts/fix-appimage-wayland.sh in place —
#      linuxdeploy-plugin-gtk hardcodes GDK_BACKEND=x11 in the AppRun hook,
#      which makes the AppImage fail to start on pure-Wayland sessions.
#
# Why Docker: release artifacts must not embed Nix-store paths (a build under
# `nix develop` links against /nix/store dylibs that users don't have). The
# container is a standard ubuntu:24.04 userspace, matching the CI runner.
#
# Caches live under tmp/ (safe to delete, keyed to the container layout):
#   tmp/docker-cargo-registry, tmp/docker-cargo-git  — cargo downloads
#   tmp/docker-target                                — cargo build dir
#   tmp/docker-pnpm                                  — pnpm store
#   tmp/docker-cache                                 — bundler downloads (linuxdeploy etc.)
#   tmp/docker-out                                   — finished artifacts
set -euo pipefail

sel="${1:-both}"
case "$sel" in
    deb | appimage | both) ;;
    *)
        echo "usage: $0 [deb|appimage|both]" >&2
        exit 1
        ;;
esac

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cache="$repo_root/tmp"
out="$cache/docker-out"
mkdir -p "$out"

command -v docker >/dev/null || { echo "error: docker is required" >&2; exit 1; }

echo "==> [1/2] building image"
docker build -q -t ruvox-build "$repo_root/scripts/docker"

echo "==> [2/2] building packages: $sel"
docker run --rm -i \
    -e BUNDLES="$sel" \
    -v "$repo_root":/src:ro \
    -v "$cache/docker-cargo-registry":/root/.cargo/registry \
    -v "$cache/docker-cargo-git":/root/.cargo/git \
    -v "$cache/docker-target":/target \
    -v "$cache/docker-pnpm":/pnpm-store \
    -v "$cache/docker-cache":/root/.cache \
    -v "$out":/out \
    ruvox-build bash -s <<'EOF'
set -euo pipefail
# codegen-units=1 + the big tauri crate occasionally overflows rustc's
# default stack in LLVM (hit 2026-08-29); raise it.
export RUST_MIN_STACK=16777216
case "$BUNDLES" in
    both) BUNDLES="deb appimage" ;;
esac

echo "[copy] repo -> /work"
# Exclude the heavy host-side trees: tmp/ holds the VM stands and docker
# caches (they ride in through the mounts below instead), target/ is the
# host build dir replaced by the mounted cache, node_modules is host-owned.
mkdir -p /work
cd /src
tar cf - \
    --exclude=./tmp \
    --exclude=./node_modules \
    --exclude=./src-tauri/target \
    --exclude=./dist \
    --exclude=./.git \
    . | tar xf - -C /work
cd /work
export CI=true CARGO_TARGET_DIR=/target
pnpm config set store-dir /pnpm-store

echo "[pnpm] install"
pnpm install --frozen-lockfile

echo "[frontend] build (tauri-codegen needs dist/ during cargo build)"
pnpm build

echo "[espeak] rlib build produces vendored espeak-ng-data"
cargo rustc --release --lib --crate-type=rlib --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml
src=$(ls -d /target/release/build/espeak-rs-sys-*/out/share/espeak-ng-data | head -1)
# Sanity: the Russian pipeline needs these three at minimum.
test -f "$src/ru_dict" && test -f "$src/phondata" && test -f "$src/intonations"
mkdir -p src-tauri/resources/espeak-ng-data
cp -r "$src/." src-tauri/resources/espeak-ng-data/
test -f src-tauri/resources/espeak-ng-data/ru_dict

echo "[onnxruntime] pinned libonnxruntime (download only when absent)"
bash scripts/fetch-linux-onnxruntime.sh --check || bash scripts/fetch-linux-onnxruntime.sh

# Bundled mpv for the AppImage (#265) — the .deb stays on Depends: mpv
# (#266), so the resource is only wired for appimage builds via the CLI
# overlay.
case "$BUNDLES" in
    *appimage*)
        echo "[mpv] pinned player bundle (download only when absent)"
        bash scripts/fetch-linux-mpv.sh --check || bash scripts/fetch-linux-mpv.sh
        ;;
esac

if [[ "$BUNDLES" == *deb* ]]; then
    echo "[tauri] build .deb"
    pnpm tauri build --bundles deb
fi
if [[ "$BUNDLES" == *appimage* ]]; then
    echo "[tauri] build .AppImage (bundles mpv via the appimage overlay)"
    pnpm tauri build --bundles appimage --config src-tauri/tauri.appimage.conf.json
fi

case "$BUNDLES" in
    *appimage*)
        echo "[appimage] applying wayland fix (GDK_BACKEND autodetect)"
        appimage=$(ls /target/release/bundle/appimage/*.AppImage | head -1)
        bash scripts/fix-appimage-wayland.sh "$appimage" "$appimage"
        ;;
esac

case "$BUNDLES" in
    *deb*) cp -a /target/release/bundle/deb/*.deb /out/ ;;
esac
case "$BUNDLES" in
    *appimage*) cp -a /target/release/bundle/appimage/*.AppImage /out/ ;;
esac
chown -R "$(stat -c%u /out):$(stat -c%g /out)" /out || true
EOF

echo "==> artifacts:"
ls -la "$out"
