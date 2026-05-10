#!/usr/bin/env bash
# Rebuilds the production RuVox binary (frontend bundled into the Rust release
# binary, no .deb/AppImage/.rpm). The result is launched via
# scripts/launch-prod.sh and the ~/Desktop/RuVox.desktop entry.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "${REPO_DIR}"

exec nix develop -c pnpm tauri build --no-bundle
