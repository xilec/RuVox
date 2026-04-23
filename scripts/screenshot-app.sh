#!/usr/bin/env bash
# Minimal e2e smoke: launch ruvox, wait for the window, screenshot it, kill it.
#
# Usage:   scripts/screenshot-app.sh [output-path]
# Default: screenshots/app.png
#
# Time budget once the binary has been built:
#   build detection ≈ 1 s
#   settle          = 2 s
#   screenshot      ≈ 0.3 s
# Total ≈ 3–4 s.  First build or any Rust change pays the cargo cost.
set -euo pipefail

OUT="${1:-screenshots/app.png}"
mkdir -p "$(dirname "$OUT")"

LOG="$(mktemp -t tauri-screenshot.log.XXXXXX)"

cleanup() {
    local pid="${TAURI_PID:-}"
    [[ -n "$pid" ]] && kill "$pid" 2>/dev/null || true
    pkill -f "target/debug/ruvox-tauri" 2>/dev/null || true
    pkill -f "tauri_plugin_mpv_socket_" 2>/dev/null || true
    pkill -f "uv run python -m ttsd" 2>/dev/null || true
}
trap cleanup EXIT

# Kill any leftover dev server holding port 1420.
pkill -f "target/debug/ruvox-tauri" 2>/dev/null || true
pkill -f "/vite/bin/vite.js" 2>/dev/null || true
sleep 0.3

export WEBKIT_DISABLE_DMABUF_RENDERER=1

pnpm tauri dev > "$LOG" 2>&1 &
TAURI_PID=$!

echo "waiting for ruvox window..." >&2
# Max 300 s ceiling (cold rust build on an older commit); finishes as soon as
# the "Running target/debug/ruvox-tauri" line appears.
for _ in $(seq 1 300); do
    if grep -q "Running .target/debug/ruvox-tauri" "$LOG"; then
        sleep 2
        break
    fi
    sleep 0.5
done

if ! pgrep -f "target/debug/ruvox-tauri" > /dev/null; then
    echo "ruvox did not start; see $LOG" >&2
    tail -20 "$LOG" >&2
    exit 1
fi

spectacle -a -b -n -o "$OUT" -d 200
echo "screenshot saved to: $OUT" >&2
