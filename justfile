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
test: test-rust test-silero-native test-ts test-python

# Rust tests (incl. golden pipeline fixtures)
test-rust:
    cargo test --manifest-path src-tauri/Cargo.toml

# silero-native engine tests
test-silero-native:
    cargo test --manifest-path silero-native/Cargo.toml

# TypeScript typecheck + unit tests
test-ts:
    pnpm typecheck
    pnpm test:unit

# ttsd pytest
test-python:
    bash -c "cd ttsd && uv run python -m pytest"

# All static checks (fmt, clippy, deny, eslint, knip, typecheck, ruff)
lint:
    cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
    cargo fmt --manifest-path silero-native/Cargo.toml -- --check
    cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --no-deps -- -D warnings
    cargo clippy --manifest-path silero-native/Cargo.toml --all-targets --no-deps -- -D warnings
    cargo deny --manifest-path src-tauri/Cargo.toml check
    pnpm lint
    pnpm knip
    pnpm typecheck
    bash -c "cd ttsd && uv run ruff check"

# Validate OpenSpec specs (specs only; in-flight changes are validated by their own cycle)
validate:
    pnpm dlx @fission-ai/openspec@1.6.0 validate --specs --strict

# Production build, slim (Piper + native Silero)
build:
    nix build .#ruvox

# Production build, full (adds the Python Silero sidecar)
build-full:
    nix build .#ruvox-with-silero

# Draft release notes for the next release into tmp/release-notes-draft.md.
# Raw material only: never writes to CHANGELOG.md — merge it by hand or via
# the .agents/skills/release-notes skill (see docs/contributing.md).
release-notes:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p tmp
    git cliff --config cliff.toml --unreleased --output tmp/release-notes-draft.md
    echo "Draft written to tmp/release-notes-draft.md:"
    cat tmp/release-notes-draft.md
