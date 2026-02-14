{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "ruvox-dev";

  buildInputs = with pkgs; [
    python311
    uv
    # PyGObject for D-Bus (used by dasbus for global hotkeys)
    gobject-introspection
    (python311Packages.pygobject3)
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
    # libmpv for audio playback (python-mpv backend)
    mpv
    # Required by PyQt6 FFmpeg multimedia plugin
    xorg.libXext
    brotli
    bzip2
    libdrm
    # Required by torch
    zstd
    krb5
    # For patching bundled QtWebEngineProcess ELF interpreter
    patchelf
    # Required by PyQt6-WebEngine (Chromium runtime dependencies)
    nss
    nspr
    xorg.libXcomposite
    xorg.libXdamage
    xorg.libXtst
    xorg.libXfixes
    xorg.libXrender
    xorg.libxshmfence
    xorg.libxkbfile
    expat
    alsa-lib
    libgbm
    systemdMinimal
    # For xdg-desktop-portal integration
    xdg-desktop-portal
    # For clipboard access on Wayland
    wl-clipboard
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
    pkgs.mpv
    pkgs.xorg.libXext
    pkgs.brotli
    pkgs.bzip2
    pkgs.libdrm
    pkgs.zstd
    pkgs.krb5
    pkgs.nss
    pkgs.nspr
    pkgs.xorg.libXcomposite
    pkgs.xorg.libXdamage
    pkgs.xorg.libXtst
    pkgs.xorg.libXfixes
    pkgs.xorg.libXrender
    pkgs.xorg.libxshmfence
    pkgs.xorg.libxkbfile
    pkgs.expat
    pkgs.alsa-lib
    pkgs.libgbm
    pkgs.systemdMinimal
  ];

  # Let Qt auto-detect platform (wayland or xcb)
  # User can override with QT_QPA_PLATFORM=xcb if needed

  # nix-ld: allow bundled QtWebEngineProcess (non-NixOS ELF) to run
  NIX_LD = "${pkgs.glibc}/lib/ld-linux-x86-64.so.2";

  shellHook = ''
    export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib:$LD_LIBRARY_PATH"
    export NIX_LD_LIBRARY_PATH="$LD_LIBRARY_PATH"

    # Unset QT_PLUGIN_PATH to let PyQt6 use its bundled plugins
    unset QT_PLUGIN_PATH

    # Patch bundled QtWebEngineProcess so it can run on NixOS
    _webengine=".venv/lib/python3.11/site-packages/PyQt6/Qt6/libexec/QtWebEngineProcess"
    if [ -f "$_webengine" ]; then
      _current_interp=$(patchelf --print-interpreter "$_webengine" 2>/dev/null || true)
      if [ "$_current_interp" = "/lib64/ld-linux-x86-64.so.2" ]; then
        chmod +w "$_webengine"
        patchelf --set-interpreter "$NIX_LD" "$_webengine"
      fi
    fi

    echo "RuVox development environment"
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
    uv sync --all-extras --no-binary-package regex

    echo ""
    echo "Commands:"
    echo "  uv run pytest             - run all tests"
    echo "  uv run pytest -v          - verbose output"
    echo "  uv run ruvox              - run UI application"
    echo "  uv run python scripts/tts_generate.py FILE  - generate speech"
    echo ""
  '';
}
