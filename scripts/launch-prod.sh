#!/usr/bin/env bash
# Launches the production RuVox release binary with the same Nix environment
# `pnpm tauri dev` would have set up (XDG_DATA_DIRS / GIO_EXTRA_MODULES /
# WEBKIT_DISABLE_DMABUF_RENDERER for KDE Plasma 6 Wayland WebKitGTK quirks).
#
# The cwd is fixed to the repo so the binary can locate `../ttsd` (Python
# subprocess) via the fallback path in src-tauri/src/lib.rs.
set -euo pipefail

REPO_DIR="/home/evgen/work/github/RuVox"

# Matches rebuild_prod.sh's CARGO_TARGET_DIR override (short path — see that
# script for why), falling back to the in-tree default for binaries built
# before that override existed.
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/ruvox-prod-target}"
BIN="${CARGO_TARGET_DIR}/release/ruvox-tauri"
if [ ! -x "${BIN}" ]; then
  BIN="${REPO_DIR}/src-tauri/target/release/ruvox-tauri"
fi

if [ ! -x "${BIN}" ]; then
  notify-send -u critical "RuVox" "Production-бинарь не найден: ${BIN}. Запусти pnpm tauri build --no-bundle." 2>/dev/null || true
  echo "binary not found: ${BIN}" >&2
  exit 1
fi

cd "${REPO_DIR}"
exec nix develop -c "${BIN}"
