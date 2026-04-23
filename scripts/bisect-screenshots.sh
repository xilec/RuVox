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
# reference it by absolute path so it survives `git checkout`.  The KWin
# activation JS also goes into /tmp — same reason.
HELPER=/tmp/bisect-screenshot-helper.sh
ACTIVATE_JS=/tmp/bisect-kwin-activate-ruvox.js

cat > "$ACTIVATE_JS" <<'JS_EOF'
var wins = typeof workspace.windowList === 'function'
    ? workspace.windowList()
    : workspace.windows;
for (var i = 0; i < wins.length; i++) {
    var w = wins[i];
    var caption = w.caption || '';
    var resourceClass = w.resourceClass || '';
    if (caption.indexOf('RuVox') !== -1
        || resourceClass.toLowerCase().indexOf('ruvox') !== -1) {
        try { w.minimized = false; } catch (e) {}
        workspace.activeWindow = w;
        if (typeof workspace.raiseWindow === 'function') {
            workspace.raiseWindow(w);
        }
        break;
    }
}
JS_EOF

cat > "$HELPER" <<HELPER_EOF
#!/usr/bin/env bash
set -eu
OUT="\$1"
LOG="\$(mktemp -t bisect-tauri.log.XXXXXX)"
export CARGO_TARGET_DIR="\${CARGO_TARGET_DIR:-/tmp/ruvox-bisect-target}"
BUILD_TIMEOUT="\${BUILD_TIMEOUT:-180}"
SETTLE="\${SETTLE:-8}"

cleanup() {
    [[ -n "\${TAURI_PID:-}" ]] && kill "\$TAURI_PID" 2>/dev/null || true
    pkill -f "target/debug/ruvox-tauri" 2>/dev/null || true
    pkill -f "tauri_plugin_mpv_socket_" 2>/dev/null || true
    pkill -f "uv run python -m ttsd" 2>/dev/null || true
}
trap cleanup EXIT

pkill -f "target/debug/ruvox-tauri" 2>/dev/null || true
pkill -f "/vite/bin/vite.js" 2>/dev/null || true
sleep 0.3

export GDK_BACKEND=x11
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export WEBKIT_DISABLE_COMPOSITING_MODE=1
pnpm tauri dev > "\$LOG" 2>&1 &
TAURI_PID=\$!

deadline=\$(( \$(date +%s) + BUILD_TIMEOUT ))
while (( \$(date +%s) < deadline )); do
    if grep -Eq "Running.*ruvox-tauri" "\$LOG"; then
        break
    fi
    sleep 0.3
done

if ! grep -Eq "Running.*ruvox-tauri" "\$LOG"; then
    echo "ruvox did not reach 'Running' within \${BUILD_TIMEOUT}s; log:" >&2
    tail -30 "\$LOG" >&2
    exit 1
fi

sleep "\$SETTLE"

if ! pgrep -f "target/debug/ruvox-tauri" > /dev/null; then
    echo "ruvox-tauri exited before screenshot; log:" >&2
    tail -30 "\$LOG" >&2
    exit 1
fi

# KWin activation so spectacle -a targets RuVox, not the terminal.
SCRIPT_ID=\$(qdbus org.kde.KWin /Scripting loadScript "$ACTIVATE_JS" 2>/dev/null || echo "")
if [[ -n "\$SCRIPT_ID" ]]; then
    qdbus "org.kde.KWin" "/Scripting/Script\${SCRIPT_ID}" "org.kde.kwin.Script.run" >/dev/null 2>&1 || true
    qdbus "org.kde.KWin" "/Scripting/Script\${SCRIPT_ID}" "org.kde.kwin.Script.stop" >/dev/null 2>&1 || true
fi
sleep 0.3

spectacle -a -b -n -o "\$OUT" -d 200
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
