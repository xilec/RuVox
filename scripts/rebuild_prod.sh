#!/usr/bin/env bash
# Rebuilds the production RuVox binary (frontend bundled into the Rust release
# binary, no .deb/AppImage/.rpm). The result is launched via
# scripts/launch-prod.sh and the ~/Desktop/RuVox.desktop entry.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "${REPO_DIR}"

# Short, fixed target dir: espeak-rs-sys's vendored espeak-ng hits a
# hardcoded 160-byte path_home buffer during its cmake phoneme-compile step
# (compiledata.c::LoadSpect). Deep checkout paths (e.g. a Claude Code git
# worktree under .claude/worktrees/<name>/) push the default
# src-tauri/target/release/build/.../phsource/... path past that limit and
# the build fails with "Bad vowel file" / a cmake exit. See flake.nix's
# preBuild for the equivalent substituteInPlace fix on the Nix package build.
# launch-prod.sh looks for the binary here first.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/ruvox-prod-target}"

exec nix develop -c pnpm tauri build --no-bundle
