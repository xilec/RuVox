# <img src="docs/images/logo.svg" width="40" align="top" alt=""/> RuVox

[Русская версия](./README.ru.md)

[![CI](https://github.com/xilec/RuVox/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/xilec/RuVox/actions/workflows/ci.yml)
![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-green)

A desktop application for narrating technical Russian-language texts.

Normalizes English terms, abbreviations, code, numbers, and URLs, then pipes the result into [Silero TTS](https://github.com/snakers4/silero-models). Unlike a bare TTS, RuVox knows how to read `getUserData()` as «гет юзер дата», `API` as «эй пи ай», and `/api/v2/users` as a path rather than letter by letter.

![RuVox screenshot](docs/images/screenshot.png)

## Stack

| Layer | Technology |
|-------|------------|
| Shell | [Tauri 2](https://tauri.app/) (Rust + native webview) |
| Frontend | React 18 + TypeScript 5 + [Mantine 8](https://mantine.dev/) |
| Backend | Rust (normalization pipeline, storage, TTS manager) |
| TTS | Python 3.12 subprocess `ttsd`, wrapping Silero TTS |
| Audio | `tauri-plugin-mpv` (libmpv with `scaletempo2`) |

## Features

- **Normalization** — English (camelCase / snake_case), abbreviations, numbers, dates, URLs, email, code.
- **Markdown + HTML** — rendered and narrated while preserving meaning.
- **Mermaid diagrams** — visualized in the UI; replaced with a «тут мермэйд диаграмма» marker for TTS.
- **Word highlight** — synchronous highlighting of the currently narrated word during playback.
- **Preview dialog** — preview the normalized text before synthesis.
- **System tray** — close-to-tray, background mode.

## Requirements

- **OS:** Linux (X11 or Wayland).
- **Nix:** recommended — the entire toolchain (Rust, Node, Python, Tauri deps) is built from `shell.nix` / `flake.nix`.
- **Without Nix:** install Rust stable + Node 20 + Python 3.12 + Tauri 2 system deps (webkitgtk 4.1, libsoup 3, gtk3, libmpv, libopus, pipewire/pulseaudio) manually — see `buildInputs` in `shell.nix`.

## Dev environment

```bash
# Via flake (recommended)
nix develop
pnpm install
pnpm tauri dev

# Or via the classic shell.nix
nix-shell --run "pnpm install"
nix-shell --run "pnpm tauri dev"
```

All commands in the docs assume execution inside `nix develop` / `nix-shell`.

## Production build

```bash
nix build .#ruvox
./result/bin/ruvox
```

`.#ruvox` builds the Tauri release binary, wraps it via `wrapProgram` (runtime `LD_LIBRARY_PATH` + `GIO_EXTRA_MODULES`), and links `ttsd` (Silero subprocess) and `mpv` into `PATH`.

> **First `nix build` run:** the `frontend` derivation uses `pnpm.fetchDeps` with `lib.fakeHash` — Nix will fail with a hash mismatch and print the real hash; substitute it into `flake.nix` and re-run the build. This is the standard pnpm2nix procedure.

## Tests

```bash
pnpm typecheck                                                  # TypeScript
cargo test --manifest-path src-tauri/Cargo.toml                 # Rust (incl. pipeline golden tests)
cargo test --manifest-path src-tauri/Cargo.toml --test golden   # golden tests only
cd ttsd && uv run python -m pytest                              # Python subprocess
```

## Documentation

| File | Description |
|------|-------------|
| [AGENTS.md](AGENTS.md) | Development rules, project structure, conventions |
| [docs/ipc-contract.md](docs/ipc-contract.md) | Tauri commands, events, ttsd protocol |
| [docs/storage-schema.md](docs/storage-schema.md) | `history.json` schema, timestamps, config |
| [CHANGELOG.md](CHANGELOG.md) | Change history |

## License

GPL-3.0 — see [LICENSE.md](LICENSE.md).
