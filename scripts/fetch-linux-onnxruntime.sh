#!/usr/bin/env bash
# Downloads and verifies the Linux onnxruntime shared library that gets
# bundled into the .deb and .AppImage packages. The Linux twin of the
# onnxruntime part of scripts/fetch-windows-resources.sh.
#
# `ort` is built with `load-dynamic` (see silero-native/Cargo.toml): it
# dlopens libonnxruntime at runtime from ORT_DYLIB_PATH, which the app sets
# to the bundled copy (src-tauri/src/lib.rs init_platform_env). The pinned
# version must match the ort-sys bindings (2.0.0-rc.12 targets the ONNX
# Runtime 1.24 API) — bump both together.
#
# Runs on the Linux CI runner before `tauri build` (release.yml
# linux-packages job); also usable for local package builds. The committed
# 0-byte placeholder at src-tauri/resources/libonnxruntime.so keeps compile-
# time resource validation green between releases — never bundle a build
# where it is still a placeholder.

set -euo pipefail

ORT_VERSION="1.24.1"
ORT_ARCHIVE="onnxruntime-linux-x64-${ORT_VERSION}.tgz"
ORT_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/${ORT_ARCHIVE}"
ORT_SHA256="9142552248b735920f9390027e4512a2cacf8946a1ffcbe9071a5c210531026f"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="$repo_root/src-tauri/resources/libonnxruntime.so"

if [ "${1:-}" = "--check" ]; then
    # Release-build guard: the file must exist and be a real ELF, not the
    # placeholder (mirrors the "never bundle placeholders" rule).
    [ -s "$target" ] || { echo "error: $target is missing or a placeholder — run $0 first" >&2; exit 1; }
    head -c 4 "$target" | grep -q $'\x7fELF' || { echo "error: $target is not an ELF shared library" >&2; exit 1; }
    exit 0
fi

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

echo "==> downloading ${ORT_ARCHIVE}"
wget -q "$ORT_URL" -O "$tmp_dir/$ORT_ARCHIVE"

echo "$ORT_SHA256  $tmp_dir/$ORT_ARCHIVE" | sha256sum -c -

echo "==> extracting libonnxruntime.so"
# Extract the versioned file directly: in the archive libonnxruntime.so is a
# symlink into it, and extracting the symlink alone leaves it dangling.
tar xzf "$tmp_dir/$ORT_ARCHIVE" -C "$tmp_dir" \
    "onnxruntime-linux-x64-${ORT_VERSION}/lib/libonnxruntime.so.${ORT_VERSION}"
cp "$tmp_dir/onnxruntime-linux-x64-${ORT_VERSION}/lib/libonnxruntime.so.${ORT_VERSION}" "$target"

./"$0" --check
echo "==> ok: $target ($(stat -c%s "$target") bytes)"
