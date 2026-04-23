#!/usr/bin/env bash
# Launch ruvox, wait for the window, focus it via KWin D-Bus, screenshot, kill.
#
# Usage:   scripts/screenshot-app.sh [output-path]
# Default: screenshots/app.png
#
# Budget (binary already built):
#   ready-detect ≈ 1 s, focus ≈ 0.3 s, settle = 3 s, capture ≈ 0.3 s.
# Cold build adds cargo time, capped by BUILD_TIMEOUT (default 120 s).
set -euo pipefail

OUT="${1:-screenshots/app.png}"
BUILD_TIMEOUT="${BUILD_TIMEOUT:-120}"
SETTLE="${SETTLE:-3}"
mkdir -p "$(dirname "$OUT")"

LOG="$(mktemp -t tauri-screenshot.log.XXXXXX)"
# Shared cargo target so switching commits rebuilds only changed crates.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/ruvox-bisect-target}"

cleanup() {
    local pid="${TAURI_PID:-}"
    [[ -n "$pid" ]] && kill "$pid" 2>/dev/null || true
    pkill -f "target/debug/ruvox-tauri" 2>/dev/null || true
    pkill -f "tauri_plugin_mpv_socket_" 2>/dev/null || true
    pkill -f "uv run python -m ttsd" 2>/dev/null || true
}
trap cleanup EXIT

pkill -f "target/debug/ruvox-tauri" 2>/dev/null || true
pkill -f "/vite/bin/vite.js" 2>/dev/null || true
sleep 0.3

export WEBKIT_DISABLE_DMABUF_RENDERER=1

pnpm tauri dev > "$LOG" 2>&1 &
TAURI_PID=$!

echo "waiting for ruvox window (BUILD_TIMEOUT=${BUILD_TIMEOUT}s)..." >&2
deadline=$(( $(date +%s) + BUILD_TIMEOUT ))
while (( $(date +%s) < deadline )); do
    if grep -Eq "Running.*ruvox-tauri" "$LOG"; then
        break
    fi
    sleep 0.3
done

if ! grep -Eq "Running.*ruvox-tauri" "$LOG"; then
    echo "ruvox did not reach 'Running' within ${BUILD_TIMEOUT}s; log:" >&2
    tail -30 "$LOG" >&2
    exit 1
fi

sleep "$SETTLE"

if ! pgrep -f "target/debug/ruvox-tauri" > /dev/null; then
    echo "ruvox-tauri exited before screenshot; log:" >&2
    tail -30 "$LOG" >&2
    exit 1
fi

# Raise/focus the RuVox window so `spectacle -a` targets it, not the terminal.
SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
SCRIPT_ID=$(qdbus org.kde.KWin /Scripting loadScript "$SCRIPT_DIR/kwin-activate-ruvox.js" 2>/dev/null || echo "")
if [[ -n "$SCRIPT_ID" ]]; then
    qdbus "org.kde.KWin" "/Scripting/Script${SCRIPT_ID}" "org.kde.kwin.Script.run" >/dev/null 2>&1 || true
    qdbus "org.kde.KWin" "/Scripting/Script${SCRIPT_ID}" "org.kde.kwin.Script.stop" >/dev/null 2>&1 || true
fi
sleep 0.2

spectacle -a -b -n -o "$OUT" -d 200
echo "screenshot saved to: $OUT" >&2
