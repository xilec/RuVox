# <img src="docs/images/logo.svg" width="40" align="top" alt=""/> RuVox

[Русская версия](./README.md)

[![CI](https://github.com/xilec/RuVox/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/xilec/RuVox/actions/workflows/ci.yml)
![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-green)

A desktop application for narrating technical Russian-language texts.

Normalizes English terms, abbreviations, code, numbers, and URLs, then pipes the result into one of three TTS engines: Silero TTS v5 in-process on ONNX Runtime (the [`silero-native`](silero-native/) crate, default; model bundle downloaded on demand), [Piper](https://github.com/rhasspy/piper) (in-process via `piper-rs`, zero-dependency fallback), or, optionally, [Silero TTS](https://github.com/snakers4/silero-models) out-of-process via the `ttsd` Python sidecar (kept as a fallback). Unlike a bare TTS, RuVox knows how to read `getUserData()` as «гет юзер дата», `API` as «эй пи ай», and `/api/v2/users` as a path rather than letter by letter.

All synthesis runs locally on your machine — no cloud TTS, nothing is sent anywhere. The network is only used to download voice models on demand, check for app updates, and import text by URL.

![RuVox screenshot](docs/images/screenshot.png)

## Installation

Ready-made builds are published on [Releases](https://github.com/xilec/RuVox/releases):

| Platform | Package | Notes |
|----------|---------|-------|
| Windows 10 22H2+ / 11 (x86_64) | `*-setup.exe` | NSIS installer; WebView2, mpv and ONNX Runtime are bundled; in-app auto-updates |
| Debian / Ubuntu | `*.deb` | `sudo dpkg -i ./ruvox_*.deb`; the system `mpv` is installed automatically as a dependency; updates with the system |
| Any Linux distribution | `*.AppImage` | runs directly (requires FUSE; where it is unavailable, use `--appimage-extract-and-run`); mpv is bundled; in-app auto-updates |

The engine's voice model is downloaded on demand at first use (the Silero Native bundle is ~230 MB).

## Stack

| Layer | Technology |
|-------|------------|
| Shell | [Tauri 2](https://tauri.app/) (Rust + native webview) |
| Frontend | React 18 + TypeScript 5 + [Mantine 8](https://mantine.dev/) |
| Backend | Rust (normalization pipeline, storage, TTS manager) |
| TTS | native Silero v5 (in-process, ONNX Runtime, [`silero-native`](silero-native/) crate, default); Piper (in-process, `piper-rs` + `onnxruntime`, fallback); Silero via `ttsd` (optional Python 3.12 subprocess, fallback) |
| Audio | `tauri-plugin-mpv` (libmpv with `scaletempo2`) |

## Features

- **[Normalization](#normalization)** — English (camelCase / snake_case), abbreviations, numbers, dates, URLs, email, code.
- **Markdown + HTML** — rendered and narrated while preserving meaning; the source format is detected automatically.
- **Import from files and URLs** — «Файл…» / File, «Файл с кодировкой…» / File with encoding, «По ссылке…» / From URL, plus drag-and-drop of `.txt`/`.md`/`.html` files and links onto the window; the encoding (UTF-8, CP1251, KOI8-R, and others) is auto-detected with a manual override.
- **Mermaid diagrams** — visualized in the UI; replaced with a «Тут мермэйд диаграмма» marker for TTS.
- **Word highlight** — synchronous highlighting of the currently narrated word during playback.
- **Preview dialog** — preview the normalized text before synthesis.
- **Audio export** — «Сохранить аудио как…» / Save audio as… in the queue context menu: WAV or Ogg Opus.
- **Recording parameters** — in the queue context menu and on double-click: which engine, voice, and settings produced a recording.
- **Playback speed** — up to 3×, persisted across restarts.
- **System tray** — close-to-tray, background mode.
- **Notifications** — when a recording is ready and on errors (can be disabled in Settings).
- **Localization** — Russian and English UI, switched in Settings.
- **Auto-updates** — the Windows installer and the AppImage check for and apply updates themselves, with signature verification.

## Normalization

TTS engines can only read plain Russian text: English words, code and special symbols cannot be pronounced as-is. Before narration, RuVox rewrites the text so that it sounds natural:

- code identifiers: `getUserData` → «гет юзер дата», `user_id` → «юзер ай ди»;
- abbreviations: `HTTP` → «эйч ти ти пи», `API` → «эй пи ай»;
- numbers, versions and dates: `v1.2.3` → «один точка два точка три», `2024-05-12` → «двенадцатое мая две тысячи двадцать четвёртого года»;
- URLs and email: `user@example.com` → «юзер собака экзампл точка ком»;
- operators and symbols: `!=` → «не равно», `===` → «строго равно», `->` → «стрелка», `α` → «альфа»;
- code blocks: replaced by default with a sentence like «далее следует пример кода на пайтон» ("next is a code sample in Python"), and in the «Читать полностью» / Read fully mode narrated in full with identifiers and operators spelled out; a mermaid diagram is always replaced with the phrase «Тут мермэйд диаграмма».

The preview dialog (opens when you add text) shows the result before synthesis: the source on the left, what will actually be spoken on the right.

### Steering it

- **Source format** in the preview dialog: «Авто» / Auto, «Обычный текст» / Plain, «Markdown», or «HTML». «Auto» is the default — RuVox detects the format itself: readable text is extracted from HTML, and Markdown markup (headings, lists, code blocks) is processed by meaning instead of being read symbol by symbol. HTML markup is detected only when the text both starts and ends with a tag — so a pasted changelog and technical prose with angle brackets (`Vec<T>`, `<type>(<module>): <desc>`) stay plain text or Markdown. If detection gets it wrong, pick the format manually.
- **Code block narration** in Settings: «Кратко» / Brief (the default) replaces a code block with a "далее следует пример кода на <язык>" sentence; «Читать полностью» / Read fully speaks the code out loud, with identifiers, operators, and brackets normalized. The setting applies immediately, without restarting the app.

## Requirements

- **OS:** Windows 10 22H2+ / 11 (x86_64) or Linux (X11 or Wayland; Ubuntu 24.04+, Debian 13+, Fedora 40+, Arch).
- **Nix** (for building from source on Linux): recommended — the entire toolchain (Rust, Node, Python, Tauri deps) is built from `flake.nix` (dev shell lives in `nix/devshell.nix`).
- **Without Nix** (building from source on Linux): a distribution that ships `webkit2gtk-4.1` (Ubuntu 24.04+, Debian 13+, Fedora 40+, Arch). Detailed step-by-step build guide: [docs/install.md](docs/install.md). Python 3.12 + `uv` are only required for the Python Silero engine (the `ttsd` sidecar) — Piper and the native Silero engine need neither.

## Dev environment

```bash
# Interactive shell
nix develop
pnpm install
pnpm tauri dev

# Or run a single command without entering the shell
nix develop -c pnpm install
nix develop -c pnpm tauri dev
```

All commands in the docs assume execution inside `nix develop` (or via `nix develop -c ...`).

## Production build

```bash
# Default (slim) — Piper + native Silero, no Python/torch in the closure.
nix build .#ruvox
./result/bin/ruvox

# Opt-in (full) — additionally bundles the ttsd sidecar, so the Python
# Silero engine is also available.
nix build .#ruvox-with-silero
./result/bin/ruvox
```

Both variants build the Tauri release binary and wrap it via `wrapProgram` (runtime `LD_LIBRARY_PATH` + `GIO_EXTRA_MODULES`) with `mpv` in `PATH`. `.#ruvox-with-silero` additionally puts the `ttsd` (Silero Python subprocess) binary in `PATH`; `.#ruvox` does not, and the Settings dialog greys the Python Silero engine out at runtime. The native Silero engine works in both variants — its ~230 MB ONNX model bundle is downloaded on demand from Settings.

The Windows installer and the Linux packages (`.deb`, `.AppImage`) are built in CI on a `v*` tag push (see [.github/workflows/release.yml](.github/workflows/release.yml)).

> **First `nix build` run:** the `frontend` derivation uses `pnpm.fetchDeps` with `lib.fakeHash` — Nix will fail with a hash mismatch and print the real hash; substitute it into `flake.nix` and re-run the build. This is the standard pnpm2nix procedure.

## Tests

All commands below run inside `nix develop` (or via `nix develop -c ...`).

```bash
just lint                                                       # all static checks (fmt, clippy, cargo-deny, eslint, knip, tsc, ruff)
just test                                                       # all tests (Rust + TypeScript + Python)
pnpm typecheck                                                  # TypeScript
pnpm test:unit                                                  # TS unit tests
cargo test --manifest-path src-tauri/Cargo.toml                 # Rust (incl. pipeline golden tests)
cargo test --manifest-path src-tauri/Cargo.toml --test golden   # golden tests only
cargo test --manifest-path silero-native/Cargo.toml             # native Silero engine (bundle-gated tests skip without SILERO_NATIVE_BUNDLE)
cd ttsd && uv run python -m pytest                              # Python subprocess
```

## Documentation

| File | Description |
|------|-------------|
| [docs/index.md](docs/index.md) | Documentation index: architecture, behavior specs, development process |
| [AGENTS.md](AGENTS.md) | Development rules, project structure, conventions |
| [docs/install.md](docs/install.md) | Building from source on Linux without Nix (Ubuntu 24.04+) |
| [docs/development.md](docs/development.md) | Dev environment, commands, debugging |
| [docs/contributing.md](docs/contributing.md) | Contributing: dictionaries, commit and style rules |
| [silero-native/](silero-native/) | Native Silero v5 engine crate (ONNX Runtime): architecture, bundle export, parity tests |
| [openspec/specs/](openspec/specs/) | Behavior specs (OpenSpec): IPC, storage, pipeline, UI, playback |
| [CHANGELOG.md](CHANGELOG.md) | Change history |

## License

Application code is GPL-3.0 — see [LICENSE.md](LICENSE.md).

> **Important:** the voice model of the default engine (Silero Native) is distributed under the CC BY-NC-SA 4.0 license and may be used for non-commercial purposes only. See [silero-native/NOTICE](silero-native/NOTICE) for details. For use without license restrictions, choose the Piper engine — both Piper itself and its voice models are MIT-licensed.
