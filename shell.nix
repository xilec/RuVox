{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "fast-tts-dev";

  buildInputs = with pkgs; [
    python311
    uv
  ];

  shellHook = ''
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
    echo "  uv run pytest           - run all tests"
    echo "  uv run pytest -v        - verbose output"
    echo "  uv run pytest -x        - stop on first failure"
    echo "  uv run pytest -k 'name' - run matching tests"
    echo ""
  '';
}
