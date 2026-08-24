# RuVox 2.0 development shell
#
# Provides:
#   - Rust stable (rustc, cargo, rustfmt, clippy)
#   - Node.js 24 LTS + pnpm
#   - Python 3.12 + uv
#   - Tauri 2 Linux system deps (webkitgtk_4_1, libsoup_3, ...)
#   - MPV/libmpv for tauri-plugin-mpv
#   - git-cliff (release-notes draft generator, see cliff.toml)
#
# Usage (canonical, via flake):
#   nix develop          — enter dev shell
#   nix develop -c cmd   — run a single command (use `bash -c "..."` for chains)
#
# This file is consumed from flake.nix `devShells.default`. It is not
# auto-loaded by `nix-shell` — that's intentional, so users land on the
# pinned, reproducible flake environment instead of the system NIX_PATH one.

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

    # ── Node.js LTS (v24) + pnpm ───────────────────────────────────────────
    nodejs_24
    pnpm

    # ── Python 3.12 + uv (for ttsd subprocess) ────────────────────────────
    python312
    uv

    # ── Task runner + pre-commit hooks ─────────────────────────────────────
    just
    lefthook

    # ── Release tooling ───────────────────────────────────────────────────
    # Drafts release notes (`just release-notes`); see docs/contributing.md
    # ("Release notes & CHANGELOG") for the workflow and ownership rules.
    git-cliff

    # ── Dependency auditing (advisories, licenses, duplicates) ─────────────
    cargo-deny

    # ── Build tools ────────────────────────────────────────────────────────
    pkg-config
    cmake
    clang
    llvmPackages.libclang

    # ── Piper TTS native runtime ───────────────────────────────────────────
    # onnxruntime is load-bearing for BOTH native TTS engines: piper-rs and
    # the silero-native engine (see silero-native/docs/architecture.md) link
    # libonnxruntime via `ort` (pykeio/ort v2) with the `load-dynamic`
    # feature, which dlopens the shared library at runtime from
    # ORT_DYLIB_PATH (set below). Do not download ort's prebuilt binaries:
    # impure in dev and impossible in the offline Nix build sandbox.
    # espeak-rs-sys vendors libespeak-ng and
    # builds it via cmake, so we don't need the package for linking — but
    # the cmake build's espeak-ng-data ends up in target/debug/build/.../out
    # which espeak-rs never looks at (it checks $CWD/espeak-ng-data and
    # $exe_dir/espeak-ng-data only). Without PIPER_ESPEAKNG_DATA_DIRECTORY
    # (set in shellHook) the library initialises with NULL data path, the
    # ru_dict / phondata / intonations files are not loaded, and Russian
    # phonemization falls back to skeleton defaults — manifesting as
    # consistently wrong word stress on every Piper voice.
    onnxruntime
    espeak-ng

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

    # ── libsonic (espeak-ng dependency) ────────────────────────────────────
    # espeak-rs-sys 0.2.0 vendors espeak-ng and builds it via cmake; its
    # deps.cmake does find_library(SONIC_LIB sonic) and falls back to
    # git-cloning https://github.com/waywardgeek/sonic via FetchContent,
    # which is flaky and breaks `cargo clippy` in the pre-push hook
    # (same rationale as `sonic` in flake.nix buildInputs).
    sonic

    # ── Wayland + X11 support ──────────────────────────────────────────────
    wayland
    wayland-protocols
    libxkbcommon
    libx11
    libxcursor
    libxrandr
    libxi
    libxcb

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
    pkgs.libx11
    pkgs.libxcursor
    pkgs.libxrandr
    pkgs.libxi
    pkgs.libxcb
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

  # bindgen (used by sonic-rs-sys, espeak-rs-sys, ort-sys) needs libclang.
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

  # `ort` with the `load-dynamic` feature dlopens libonnxruntime at runtime.
  ORT_DYLIB_PATH = "${pkgs.onnxruntime}/lib/libonnxruntime.so";

  # Point espeak-rs at the full nixpkgs espeak-ng data dir. The crate looks
  # for `<this>/espeak-ng-data/` and falls back to NULL (= no data path) if
  # the directory is missing — see comment next to `espeak-ng` in buildInputs.
  PIPER_ESPEAKNG_DATA_DIRECTORY = "${pkgs.espeak-ng}/share";

  shellHook = ''
    export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib:$LD_LIBRARY_PATH"

    # espeak-rs-sys 0.2.0 finds the system libsonic via CMake's
    # find_library() and links it into the static espeak-ng target with
    # `target_link_libraries(... PRIVATE ''${SONIC_LIB})`. CMake does not
    # propagate PRIVATE link libraries of a STATIC target to consumers, so
    # build.rs never emits `cargo:rustc-link-lib=sonic` and dev/test
    # binaries fail to link with undefined references to sonic* symbols.
    # Same workaround as the production build (flake.nix `env.RUSTFLAGS`);
    # the -L search path comes from buildInputs' `sonic` via NIX_LDFLAGS.
    export RUSTFLAGS="-C link-arg=-lsonic''${RUSTFLAGS:+ $RUSTFLAGS}"

    # bindgen needs the C system include paths from stdenv.cc — without these,
    # `#include <stdio.h>` fails inside the espeak-rs-sys / sonic-rs-sys build
    # scripts because clang has no implicit C system headers under nix.
    if [ -f "${pkgs.stdenv.cc}/nix-support/libcxx-cxxflags" ]; then
      _cxxflags="$(< ${pkgs.stdenv.cc}/nix-support/libcxx-cxxflags)"
    else
      _cxxflags=""
    fi
    export BINDGEN_EXTRA_CLANG_ARGS="$(< ${pkgs.stdenv.cc}/nix-support/libc-crt1-cflags) $(< ${pkgs.stdenv.cc}/nix-support/libc-cflags) $(< ${pkgs.stdenv.cc}/nix-support/cc-cflags) $_cxxflags"

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

    # tauri-cli's AppImage bundler locates libayatana-appindicator3 by running
    # `pkg-config --libs-only-L ayatana-appindicator3-0.1` and stripping ONLY
    # the leading "-L" off the output (tauri-cli rust.rs `get_library_path`,
    # tauri <= 2.11.4). Under Nix that query returns one -L flag per
    # transitive dependency, so the whole flag list is misparsed into a single
    # bogus directory and AppImage bundling aborts with "Failed to copy
    # custom files". Shim pkg-config to answer that exact query with the one
    # appindicator libdir; every other invocation passes through untouched.
    if [ ! -e /tmp/ruvox-pkgconfig-shim/pkg-config ]; then
      mkdir -p /tmp/ruvox-pkgconfig-shim
      cat > /tmp/ruvox-pkgconfig-shim/pkg-config <<'EOF'
#!/usr/bin/env bash
if [ "$1" = "--libs-only-L" ] && [ "$2" = "ayatana-appindicator3-0.1" ]; then
  echo "-L$(REAL_PKG_CONFIG --variable=libdir ayatana-appindicator3-0.1)"
  exit 0
fi
exec REAL_PKG_CONFIG "$@"
EOF
    fi
    _real_pc=$(command -v pkg-config)
    sed -i "s|REAL_PKG_CONFIG|$_real_pc|g" /tmp/ruvox-pkgconfig-shim/pkg-config
    chmod +x /tmp/ruvox-pkgconfig-shim/pkg-config
    export PATH="/tmp/ruvox-pkgconfig-shim:$PATH"

    # Install pre-commit hooks (idempotent; silently skipped outside a git
    # checkout or when lefthook.yml is absent)
    lefthook install > /dev/null 2>&1 || true

    # lefthook v2 computes pre-push files via `git diff --name-only HEAD
    # @{push}`; on the FIRST push of a branch there is no upstream yet, so it
    # falls back to diffing against the *local* branch named by
    # refs/remotes/origin/HEAD (i.e. `main`).  When local `main` is absent
    # (deleted, or checked out in another clone/worktree), that diff fails
    # and every pre-push hook exits 128, rejecting the push — only
    # `--no-verify` helps.  Keep a local `main` ref around so the fallback
    # never breaks (best-effort, offline-safe, idempotent; staleness is
    # harmless because our pre-push hooks don't use the file list).
    git show-ref --verify --quiet refs/heads/main \
      || git branch --no-track main origin/main > /dev/null 2>&1 \
      || true

    echo "RuVox 2.0 development environment"
    echo "  Rust:   $(rustc --version)"
    echo "  Node:   $(node --version)"
    echo "  pnpm:   $(pnpm --version)"
    echo "  Python: $(python3 --version)"
    echo "  uv:     $(uv --version)"
    echo "  tauri:  $(cargo tauri --version)"
    echo ""
    echo "Commands:"
    echo "  just --list              — all routine tasks (test, lint, dev, build)"
    echo "  just test / just lint    — run all tests / all static checks"
    echo "  cargo tauri dev          — start Tauri dev server"
    echo "  pnpm install             — install frontend deps"
    echo "  uv run python -m ttsd    — run TTS subprocess"
    echo ""
  '';
}
