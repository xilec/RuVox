# RuVox — task runner. Recipes assume the dev shell: run `nix develop` first,
# or one-shot from outside as `nix develop -c just <recipe>`.

# List available recipes
default:
    @just --list

# Install frontend deps
install:
    pnpm install

# Start the Tauri dev server
dev:
    pnpm tauri dev

# Run all tests (Rust + TS + Python)
test: test-rust test-ts test-python

# Rust tests (incl. golden pipeline fixtures)
test-rust:
    cargo test --manifest-path src-tauri/Cargo.toml

# TypeScript typecheck + unit tests
test-ts:
    pnpm typecheck
    pnpm test:unit

# ttsd pytest
test-python:
    bash -c "cd ttsd && uv run python -m pytest"

# All static checks (fmt, clippy, typecheck, ruff)
lint:
    cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
    cargo clippy --manifest-path src-tauri/Cargo.toml --no-deps -- -D warnings
    pnpm typecheck
    bash -c "cd ttsd && uv run ruff check"

# Validate OpenSpec specs (and in-flight changes, if any)
validate:
    pnpm dlx @fission-ai/openspec validate --specs --strict

# Production build, slim (Piper only)
build:
    nix build .#ruvox

# Production build, full (Piper + Silero sidecar)
build-full:
    nix build .#ruvox-with-silero
