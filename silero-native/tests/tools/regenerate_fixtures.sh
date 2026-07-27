#!/usr/bin/env bash
# Regenerate all golden/parity fixtures for the silero-native test suite.
#
# Runs every generator inside the silero-native/export uv environment
# (pinned torch CPU + onnxruntime). Requirements and background: see
# README.md in this directory. For non-default model/bundle paths run the
# generators directly (they take `--model-path` / `--bundle`).
set -euo pipefail

TOOLS="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cd "$TOOLS/../../export"
uv run python "$TOOLS/gen_chunking_fixtures.py"
uv run python "$TOOLS/gen_frontend_fixtures.py"
uv run python "$TOOLS/gen_accentor_fixtures.py"
uv run python "$TOOLS/gen_parity_fixtures.py"
