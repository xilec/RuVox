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
#
# cargo-tauri (tauri-cli v2) is NOT in nixpkgs 25.11.
# Install it separately: cargo install tauri-cli --version "^2"
# After installation it is available as `cargo tauri`.

{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "ruvox2-dev";

  buildInputs = with pkgs; [
    # ── Rust stable toolchain ──────────────────────────────────────────────
    rustc
    cargo
    rustfmt
    clippy

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
    # App indicator (system tray)
    libappindicator-gtk3
    # SVG rendering (Tauri icons)
    librsvg
    # OpenSSL (reqwest / native-tls); dev output needed for cargo build scripts
    openssl
    openssl.dev
    # D-Bus
    dbus

    # ── MPV / libmpv (for tauri-plugin-mpv) ────────────────────────────────
    # mpv-unwrapped provides both the library and pkg-config .pc file
    mpv-unwrapped

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
    pkgs.libappindicator-gtk3
    pkgs.librsvg
    pkgs.wayland
    pkgs.libxkbcommon
    pkgs.alsa-lib
    pkgs.libpulseaudio
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
    pkgs.libappindicator-gtk3
    pkgs.librsvg
  ];

  # Help Rust openssl-sys crate find OpenSSL
  OPENSSL_DIR = "${pkgs.openssl.dev}";
  OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";

  shellHook = ''
    export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib:$LD_LIBRARY_PATH"

    # Needed by glib-networking (TLS for WebKit)
    export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules"

    echo "RuVox 2.0 development environment"
    echo "  Rust:   $(rustc --version)"
    echo "  Node:   $(node --version)"
    echo "  pnpm:   $(pnpm --version)"
    echo "  Python: $(python3 --version)"
    echo "  uv:     $(uv --version)"
    echo ""
    echo "Note: cargo-tauri is not in nixpkgs."
    echo "Install with: cargo install tauri-cli --version '^2'"
    echo "(one-time, stored in ~/.cargo/bin/cargo-tauri)"
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
