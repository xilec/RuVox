{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "fast-tts-dev";

  buildInputs = with pkgs; [
    python311
    uv
    # Required for torch
    stdenv.cc.cc.lib
    zlib
  ];

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
    pkgs.stdenv.cc.cc.lib
    pkgs.zlib
  ];

  shellHook = ''
    export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib:$LD_LIBRARY_PATH"

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
    echo "  uv run python scripts/tts_generate.py FILE  - generate speech"
    echo ""
  '';
}
