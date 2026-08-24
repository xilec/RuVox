#!/usr/bin/env bash
# Make a Tauri AppImage robust on pure-Wayland sessions (no XWayland).
#
# tauri's linuxdeploy-plugin-gtk injects `export GDK_BACKEND=x11` into the
# AppImage's AppRun hook (a workaround for tauri#8541). On a session without
# XWayland this makes GTK fail to initialize ("Failed to initialize GTK") even
# though the app renders fine under Wayland. We let GDK auto-detect the backend
# (preferring wayland) so the AppImage works both with and without XWayland.
#
# Usage:
#   scripts/fix-appimage-wayland.sh <input.AppImage> [output.AppImage]
#
# Repackages the AppImage with appimagetool. appimagetool is fetched on demand
# (its AppImage is extracted without FUSE to run on hosts that lack /dev/fuse).
set -euo pipefail

SRC="${1:?usage: $0 <input.AppImage> [output.AppImage]}"
DST="${2:-${SRC%.AppImage}.wayland.AppImage}"

TMP="$(mktemp -d)"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

APPDIR="$TMP/appdir"

# --- pick an extractor ---
# An AppImage is an ELF runtime followed by a squashfs image at a page-aligned
# offset. unsquashfs with an explicit -o offset is the primary extractor: the
# squashfs may be zstd-compressed (tauri's linuxdeploy output is), which
# neither p7zip 16 (`7z` on ubuntu-24.04) nor 7-Zip < 24.09 can read, while
# squashfs-tools supports zstd natively. 7z/7zz remain as fallback for hosts
# without squashfs-tools.
# $1 — destination directory ($SRC is the AppImage to extract).
EXTRACT() {
  local dest="$1" off
  if command -v unsquashfs >/dev/null 2>&1; then
    # The ELF runtime can contain the bytes "hsqs" itself (false positive),
    # so try every match until unsquashfs accepts one as a real superblock.
    while read -r off; do
      if unsquashfs -f -d "$dest" -o "$off" "$SRC" >/dev/null 2>&1; then
        return 0
      fi
      rm -rf "$dest"
    done < <(grep -abo hsqs "$SRC" | cut -d: -f1)
    echo "error: no valid squashfs superblock found in $SRC" >&2
    return 1
  elif command -v 7zz >/dev/null 2>&1; then
    7zz x -y -o"$dest" "$SRC" >/dev/null
  elif command -v 7z >/dev/null 2>&1; then
    7z x -y -o"$dest" "$SRC" >/dev/null
  else
    echo "error: need unsquashfs, 7zz or 7z to extract the AppImage" >&2
    exit 1
  fi
}

# --- locate appimagetool (avoid FUSE: extract its AppImage) ---
# Pinned continuous-build asset + sha256 (same discipline as
# fetch-*-resources scripts). The asset is replaced on upstream updates —
# bump both values together when that happens.
AIT_URL="https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
AIT_SHA256="b90f4a8b18967545fda78a445b27680a1642f1ef9488ced28b65398f2be7add2"
AIT=""
if command -v appimagetool >/dev/null 2>&1; then
  AIT="appimagetool"
else
  AITBIN="$TMP/appimagetool.AppImage"
  if [ ! -s "$AITBIN" ]; then
    command -v wget >/dev/null 2>&1 || { echo "error: need wget to fetch appimagetool" >&2; exit 1; }
    wget -q "$AIT_URL" -O "$AITBIN"
    echo "$AIT_SHA256  $AITBIN" | sha256sum -c - || {
      echo "error: appimagetool sha256 mismatch — the pinned upstream asset was likely replaced; update AIT_URL/AIT_SHA256" >&2
      exit 1
    }
  fi
  AITDIR="$TMP/ait"; mkdir -p "$AITDIR"
  # The appimagetool squashfs is plain gzip — the same offset-based extract
  # works (EXTRACT reads $SRC; point it at the AIT download for this call).
  src_save="$SRC"; SRC="$AITBIN"
  EXTRACT "$AITDIR"
  SRC="$src_save"
  chmod -R +x "$AITDIR/AppRun" "$AITDIR/usr/bin" "$AITDIR/usr/lib" 2>/dev/null || true
  if [ -x "$AITDIR/squashfs-root/AppRun" ]; then
    AIT="$AITDIR/squashfs-root/AppRun"
  elif [ -x "$AITDIR/AppRun" ]; then
    AIT="$AITDIR/AppRun"
  else
    echo "error: could not locate appimagetool AppRun" >&2
    exit 1
  fi
fi

command -v file >/dev/null 2>&1 || { echo "error: appimagetool needs 'file'" >&2; exit 1; }

# --- extract target AppImage ---
EXTRACT "$APPDIR"

# --- patch GDK_BACKEND ---
HOOK="$APPDIR/apprun-hooks/linuxdeploy-plugin-gtk.sh"
if [ -f "$HOOK" ]; then
  sed -i 's|^export GDK_BACKEND=x11.*|export GDK_BACKEND="${GDK_BACKEND:-wayland}"|' "$HOOK"
fi

# --- restore exec bits (7z drops them) ---
chmod +x "$APPDIR/AppRun" "$APPDIR/AppRun.wrapped" 2>/dev/null || true
find "$APPDIR/usr/bin" -type f -exec chmod +x {} \; 2>/dev/null || true
find "$APPDIR/usr/lib/x86_64-linux-gnu/webkit2gtk-4.1" -type f -exec chmod +x {} \; 2>/dev/null || true

# --- repackage ---
"$AIT" "$APPDIR" "$DST" >/dev/null
echo "wrote $DST"
