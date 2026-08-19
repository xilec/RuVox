#!/usr/bin/env bash
# Downloads and verifies the third-party Windows binaries that get bundled
# into the NSIS installer (OpenSpec change windows-installer-and-release,
# decision D2). Runs on the Windows CI runner under Git Bash; also usable
# locally on NixOS for a manual installer build.
#
# Versions and sha256 are pinned here — bump both together.
#
#   mpv          shinchiro/mpv-winbuild-cmake release (7z) — the player
#                subprocess; lands at <install>/mpv/mpv.exe
#   onnxruntime  microsoft/onnxruntime release (zip) — DL'd by ort
#                (load-dynamic); lands next to the exe
#   VC++ CRT     copied app-local from the runner's Visual Studio
#                (license allows shipping these DLLs next to the exe).
#                A clean Windows has no MSVCP140.dll — the first v0.3.0
#                VM run failed to start without it.
#
# espeak-ng-data is NOT here: it is extracted from the espeak-rs-sys build
# tree after `cargo build` (see release.yml), so it always matches the
# linked library version.

set -euo pipefail

MPV_TAG="20260814"
MPV_ARCHIVE="mpv-x86_64-${MPV_TAG}-git-7b8915bc1d.7z"
MPV_URL="https://github.com/shinchiro/mpv-winbuild-cmake/releases/download/${MPV_TAG}/${MPV_ARCHIVE}"
MPV_SHA256="1bf3b029da2c98e605e00e85f21ee3142f22a1dcc4ceb5c827b5c51e36e390f9"

# The shinchiro 7z carries no license text; the spec (windows-installer)
# requires one — fetch mpv's Copyright file from the matching upstream tag.
MPV_COPYRIGHT_URL="https://raw.githubusercontent.com/mpv-player/mpv/v0.41.0/Copyright"
MPV_COPYRIGHT_SHA256="bfe9ee4cceabcb8ecbfadf208d04156f73d801e6a57369a5606bb8341e204a23"

ORT_VERSION="1.24.2"
ORT_ARCHIVE="onnxruntime-win-x64-${ORT_VERSION}.zip"
ORT_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/${ORT_ARCHIVE}"
ORT_SHA256="8e3e9c826375352e29cb2614fe44f3d7a4b0ff7b8028ad7a456af9d949a7e8b0"

# Destination layout consumed by bundle.resources in tauri.conf.json.
RESOURCES_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/src-tauri/resources"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

fetch() {
  local url=$1 out=$2 sha=$3
  echo ">> $out"
  curl -fL --retry 3 --retry-delay 5 -o "$out" "$url"
  echo "$sha  $out" | sha256sum -c - >/dev/null
}

fetch "$MPV_URL" "$TMP_DIR/$MPV_ARCHIVE" "$MPV_SHA256"
fetch "$ORT_URL" "$TMP_DIR/$ORT_ARCHIVE" "$ORT_SHA256"
fetch "$MPV_COPYRIGHT_URL" "$TMP_DIR/mpv-Copyright" "$MPV_COPYRIGHT_SHA256"

mkdir -p "$RESOURCES_DIR/mpv"
7z x -y -o"$RESOURCES_DIR/mpv" "$TMP_DIR/$MPV_ARCHIVE" >/dev/null
cp "$TMP_DIR/mpv-Copyright" "$RESOURCES_DIR/mpv/Copyright"

# Only the runtime DLL is bundled; the zip also carries headers/libs/pdb.
# `7z x` + include pattern (paths preserved, then moved flat) — `7z e`
# trips an old-p7zip directory-entry bug on this archive.
7z x -y -o"$TMP_DIR/ort" "$TMP_DIR/$ORT_ARCHIVE" \
  "onnxruntime-win-x64-${ORT_VERSION}/lib/onnxruntime.dll" >/dev/null
cp "$TMP_DIR/ort/onnxruntime-win-x64-${ORT_VERSION}/lib/onnxruntime.dll" \
  "$RESOURCES_DIR/onnxruntime.dll"

# VC++ 2015-2022 CRT, app-local. Not a download: copied from the runner's
# Visual Studio install (newest MSVC toolset dir wins). Off-runner (local
# NixOS) the path does not exist — warn and skip, Linux builds don't need it.
CRT_SRC=$(ls -d "/c/Program Files/Microsoft Visual Studio/"2022/*/VC/Redist/MSVC/*/x64/Microsoft.VC143.CRT 2>/dev/null | sort -V | tail -1 || true)
if [ -n "$CRT_SRC" ]; then
  mkdir -p "$RESOURCES_DIR/crt"
  cp "$CRT_SRC"/*.dll "$RESOURCES_DIR/crt/"
  # The committed placeholder only exists to satisfy tauri-build's
  # compile-time glob check; it must NOT reach the installer.
  rm -f "$RESOURCES_DIR/crt/PLACEHOLDER.dll"
  echo ">> CRT bundled app-local from $CRT_SRC"
else
  echo ">> WARN: VS CRT redist not found (expected off-runner); skipping app-local CRT"
fi

echo "OK: resources ready in $RESOURCES_DIR"
