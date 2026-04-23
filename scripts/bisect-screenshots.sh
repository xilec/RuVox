#!/usr/bin/env bash
# Walk a list of commits SEQUENTIALLY (linear, not git-bisect binary search),
# build each, screenshot ruvox, hash the PNG, and log the result to a
# manifest.  On exit restore whichever ref was checked out at start.
#
# Usage:
#   scripts/bisect-screenshots.sh [COMMIT ...]
#
# No args → use DEFAULT_COMMITS below.  Include only commits that actually
# run — earlier commits crash on startup (tray.png + async_runtime bugs
# that 4ccd6ab fixed) and would only produce ERROR rows.
#
# Clean working tree required.  Output:
#   screenshots/bisect/<sha>.png
#   screenshots/bisect/manifest.txt  (hashes, groups of identical frames)
set -euo pipefail

# Linear order, oldest → newest.
DEFAULT_COMMITS=(
    4ccd6ab  # fix(runtime): tauri dev startup (first running build)
    c151632  # fix(ui): smoke-test fixes (word span, seek, pitch, pause, theme)
    af631e6  # fix(ui): font-size 15px, main layout, delete in context menu
    92aecb0  # fix(player): kill mpv on exit, reap orphan mpv on startup
    6f13be0  # fix(ui,player): crash on seek, duration first play, font 17, layout
    58fcdd9  # fix(ui): font scale, seek slider drag, right-click menu, main minH
)

COMMITS=("$@")
[[ ${#COMMITS[@]} -eq 0 ]] && COMMITS=("${DEFAULT_COMMITS[@]}")

OUT_DIR="screenshots/bisect"
mkdir -p "$OUT_DIR"
MANIFEST="$OUT_DIR/manifest.txt"
: > "$MANIFEST"

# The screenshot helper may not exist at older commits; write one in /tmp and
# reference it by absolute path so it survives `git checkout`.
HELPER=/tmp/bisect-screenshot-helper.sh
cat > "$HELPER" <<'HELPER_EOF'
#!/usr/bin/env bash
set -eu
OUT="$1"
LOG="$(mktemp -t bisect-tauri.log.XXXXXX)"

cleanup() {
    [[ -n "${TAURI_PID:-}" ]] && kill "$TAURI_PID" 2>/dev/null || true
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

for _ in $(seq 1 600); do
    if grep -q "Running .target/debug/ruvox-tauri" "$LOG"; then
        sleep 2
        break
    fi
    sleep 0.5
done

if ! pgrep -f "target/debug/ruvox-tauri" > /dev/null; then
    echo "ruvox did not start; log:" >&2
    tail -30 "$LOG" >&2
    exit 1
fi

spectacle -a -b -n -o "$OUT" -d 200
HELPER_EOF
chmod +x "$HELPER"

ORIG_REF="$(git symbolic-ref --quiet --short HEAD 2>/dev/null || git rev-parse HEAD)"
echo "start ref: $ORIG_REF" >&2

restore() {
    echo "restoring $ORIG_REF" >&2
    git checkout -q "$ORIG_REF"
}
trap restore EXIT

for commit in "${COMMITS[@]}"; do
    short=$(git rev-parse --short "$commit")
    echo "=== $short ===" >&2
    git checkout -q "$commit"

    png="$OUT_DIR/${short}.png"
    rm -f "$png"
    if "$HELPER" "$png" 2>>"$OUT_DIR/${short}.log"; then
        hash=$(sha256sum "$png" | awk '{print substr($1,1,12)}')
        size=$(stat -c %s "$png")
        echo "$short  $hash  ${size}B  $(git log -1 --format=%s "$commit")" | tee -a "$MANIFEST" >&2
    else
        echo "$short  ERROR  see $OUT_DIR/${short}.log" | tee -a "$MANIFEST" >&2
    fi
done

# Mark visually-identical groups (same hash = same bytes).
echo "" >> "$MANIFEST"
echo "# groups by hash (same = pixel-identical screenshot)" >> "$MANIFEST"
awk '$2 ~ /^[0-9a-f]+$/ { print $2, $1 }' "$MANIFEST" | sort | \
    awk '{ hash=$1; sha=$2; hashes[hash]=hashes[hash]" "sha } END { for (h in hashes) print h":"hashes[h] }' \
    >> "$MANIFEST"

cat "$MANIFEST" >&2
echo "" >&2
echo "manifest: $MANIFEST" >&2
