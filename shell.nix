{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "fast-tts-dev";

  buildInputs = with pkgs; [
    python311
    uv
    # Required for torch
    stdenv.cc.cc.lib
    zlib
    # Required for PyQt6 UI
    qt6.qtbase
    qt6.qtmultimedia
    qt6.qtsvg
    qt6.qtwayland
    libxkbcommon
    libGL
    fontconfig
    freetype
    glib
    dbus
    # X11/Wayland libs needed by Qt
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    xorg.libxcb
    xorg.xcbutilwm
    xorg.xcbutilimage
    xorg.xcbutilkeysyms
    xorg.xcbutilrenderutil
    xorg.xcbutil
    xcb-util-cursor
    wayland
    wayland-protocols
    libdecor
    # Audio libs
    libpulseaudio
    pipewire
    # Additional libs
    zstd
    krb5
    # For xdg-desktop-portal integration
    xdg-desktop-portal
  ];

  # Only include system libs, NOT Qt - let PyQt6 use its bundled Qt
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
    pkgs.stdenv.cc.cc.lib
    pkgs.zlib
    pkgs.libxkbcommon
    pkgs.libGL
    pkgs.fontconfig
    pkgs.freetype
    pkgs.glib
    pkgs.dbus
    pkgs.xorg.libX11
    pkgs.xorg.libXcursor
    pkgs.xorg.libXrandr
    pkgs.xorg.libXi
    pkgs.xorg.libxcb
    pkgs.xorg.xcbutilwm
    pkgs.xorg.xcbutilimage
    pkgs.xorg.xcbutilkeysyms
    pkgs.xorg.xcbutilrenderutil
    pkgs.xorg.xcbutil
    pkgs.xcb-util-cursor
    pkgs.wayland
    pkgs.libdecor
    pkgs.libpulseaudio
    pkgs.pipewire
    pkgs.zstd
    pkgs.krb5
  ];

  # Let Qt auto-detect platform (wayland or xcb)
  # User can override with QT_QPA_PLATFORM=xcb if needed

  shellHook = ''
    export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib:$LD_LIBRARY_PATH"

    # Unset QT_PLUGIN_PATH to let PyQt6 use its bundled plugins
    unset QT_PLUGIN_PATH

    echo "Fast TTS development environment"
    echo "Python: $(python3 --version)"
    echo "uv: $(uv --version)"
    echo ""

    # Create venv if not exists
    if [ ! -d ".venv" ]; then
      echo "Creating virtual environment..."
      uv venv
    fi

    # Activate venv
    source .venv/bin/activate

    # Sync dependencies
    echo "Syncing dependencies..."
    uv sync --all-extras

    echo ""
    echo "Commands:"
    echo "  uv run pytest             - run all tests"
    echo "  uv run pytest -v          - verbose output"
    echo "  uv run fast-tts-ui        - run UI application"
    echo "  uv run python scripts/tts_generate.py FILE  - generate speech"
    echo ""
  '';
}
