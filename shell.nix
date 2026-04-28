# RuVox 2.0 development shell
#
# Provides:
#   - Rust stable (rustc, cargo, rustfmt, clippy)
#   - Node.js 20 LTS + pnpm
#   - Python 3.12 + uv
#   - Tauri 2 Linux system deps (webkitgtk_4_1, libsoup_3, ...)
#   - MPV/libmpv for tauri-plugin-mpv
#
# Usage:
#   nix-shell          — enter dev shell
#   nix-shell --run "cmd"  — run a single command

{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "ruvox2-dev";

  buildInputs = with pkgs; [
    # ── Rust stable toolchain ──────────────────────────────────────────────
    rustc
    cargo
    rustfmt
    clippy
    cargo-tauri

    # ── Node.js LTS (v20) + pnpm ───────────────────────────────────────────
    nodejs_20
    pnpm

    # ── Python 3.12 + uv (for ttsd subprocess) ────────────────────────────
    python312
    uv

    # ── Build tools ────────────────────────────────────────────────────────
    pkg-config

    # ── Tauri 2 Linux system dependencies ─────────────────────────────────
    # WebKit with ABI 4.1 (required by Tauri 2; 4.0 was removed)
    webkitgtk_4_1
    # libsoup 3 (Tauri 2 requires libsoup 3, not 2)
    libsoup_3
    # GTK 3 and related
    gtk3
    glib
    glib-networking
    # App indicator (system tray) — ayatana fork, the one Tauri 2 targets
    libayatana-appindicator
    # SVG rendering (Tauri icons)
    librsvg
    # OpenSSL (reqwest / native-tls)
    openssl
    # D-Bus
    dbus

    # ── MPV / libmpv (for tauri-plugin-mpv) ────────────────────────────────
    # mpv-unwrapped provides both the library and pkg-config .pc file
    mpv-unwrapped

    # ── libopus (Opus encoder for storage::audio) ─────────────────────────
    # The `opus = "0.3"` Rust crate is an FFI binding to libopus 1.x; needs
    # the C library at link time and at runtime.
    libopus

    # ── Wayland + X11 support ──────────────────────────────────────────────
    wayland
    wayland-protocols
    libxkbcommon
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    xorg.libxcb

    # ── Audio backend ──────────────────────────────────────────────────────
    libpulseaudio
    pipewire
    alsa-lib

    # ── Additional graphics / display ──────────────────────────────────────
    libGL
    fontconfig
    freetype
    libdrm

    # ── Torch / Python native extension support ────────────────────────────
    stdenv.cc.cc.lib
    zlib
    zstd
  ];

  # Make pkg-config find the libraries
  PKG_CONFIG_PATH = pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" [
    pkgs.webkitgtk_4_1
    pkgs.libsoup_3
    pkgs.gtk3
    pkgs.glib
    pkgs.openssl
    pkgs.mpv-unwrapped
    pkgs.libayatana-appindicator
    pkgs.librsvg
    pkgs.wayland
    pkgs.libxkbcommon
    pkgs.alsa-lib
    pkgs.libpulseaudio
    pkgs.libopus
  ];

  # Runtime library path (for Python + Tauri + mpv)
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
    pkgs.stdenv.cc.cc.lib
    pkgs.zlib
    pkgs.zstd
    pkgs.openssl
    pkgs.libGL
    pkgs.fontconfig
    pkgs.freetype
    pkgs.glib
    pkgs.dbus
    pkgs.gtk3
    pkgs.webkitgtk_4_1
    pkgs.libsoup_3
    pkgs.mpv-unwrapped
    pkgs.wayland
    pkgs.libxkbcommon
    pkgs.xorg.libX11
    pkgs.xorg.libXcursor
    pkgs.xorg.libXrandr
    pkgs.xorg.libXi
    pkgs.xorg.libxcb
    pkgs.libpulseaudio
    pkgs.pipewire
    pkgs.alsa-lib
    pkgs.libdrm
    pkgs.libayatana-appindicator
    pkgs.librsvg
    pkgs.libopus
  ];

  # Help Rust openssl-sys crate find OpenSSL
  OPENSSL_DIR = "${pkgs.openssl.dev}";
  OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";

  shellHook = ''
    export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib:$LD_LIBRARY_PATH"

    # Needed by glib-networking (TLS for WebKit)
    export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules"

    # WebKitGTK inside Tauri mis-initialises window metrics on Wayland unless
    # the GSettings schemas for gsettings-desktop-schemas and gtk+3 are visible
    # via XDG_DATA_DIRS AND glib-networking's gio-modules are discoverable.
    # Without them devicePixelRatio becomes negative, innerWidth/Height go
    # negative, computed html font-size blows up to millions of px, and every
    # CSS value collapses to the same on-screen size.  Root cause: upstream
    # tauri #7354 — on non-standard distros (NixOS, similar) webkit2gtk asks
    # GSettings for scaling hints, gets nothing, and garbage is the result.
    # Fix discovered in the #7354 thread (comments by n3oney / Mange); the
    # XDG_DATA_DIRS / GIO_EXTRA_MODULES exports below are what makes it work.
    # DMABUF renderer still crashes with "Gdk-Message Error 71 (Protocol
    # error)" on KDE Plasma 6 Wayland — disable it explicitly.
    export WEBKIT_DISABLE_DMABUF_RENDERER=1

    # GSettings schemas + icon theme search path: wrapGAppsHook4 sets these
    # for the production bundle; in dev we set them manually.
    export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:${pkgs.hicolor-icon-theme}/share:$XDG_DATA_DIRS"

    echo "RuVox 2.0 development environment"
    echo "  Rust:   $(rustc --version)"
    echo "  Node:   $(node --version)"
    echo "  pnpm:   $(pnpm --version)"
    echo "  Python: $(python3 --version)"
    echo "  uv:     $(uv --version)"
    echo "  tauri:  $(cargo tauri --version)"
    echo ""
    echo "Commands:"
    echo "  cargo tauri dev          — start Tauri dev server"
    echo "  cargo tauri build        — production build"
    echo "  pnpm install             — install frontend deps"
    echo "  pnpm typecheck           — TypeScript typecheck"
    echo "  uv run python -m ttsd    — run TTS subprocess"
    echo ""
  '';
}
