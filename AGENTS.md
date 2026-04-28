# RuVox — Development Guide

> **NOTE:** Always reply in Russian when communicating with the user (assistant
> instruction; the documentation itself is in English).

## Project overview

**RuVox 2.0** is a desktop application for narrating technical Russian-language texts.

**Stack:**
- **Shell:** Tauri 2 (Rust-based desktop shell with native webview)
- **Frontend:** React 18 + TypeScript 5 + Mantine 8
- **Backend:** Rust (text normalization pipeline, storage, TTS subprocess manager, player wrapper)
- **TTS engine:** Python subprocess `ttsd` wrapping Silero TTS (runs as a sidecar process)

**Goal** unchanged: normalize technical text (API, URLs, code identifiers, numbers) before passing it to Silero TTS, which cannot read English or special characters.

### Problem → Solution

```
"Вызови getUserData() через API"
        ↓
"Вызови гет юзер дата через эй пи ай"
```

## Documentation

| File / Section | Description |
|----------------|-------------|
| [docs/ipc-contract.md](docs/ipc-contract.md) | IPC contract: Tauri commands, events, ttsd protocol |
| [docs/storage-schema.md](docs/storage-schema.md) | Storage schema: history.json, timestamps, config |
| [docs/pipeline.md](docs/pipeline.md) | Text normalization stages |
| [docs/ui.md](docs/ui.md) | Frontend structure |
| [docs/use-cases.md](docs/use-cases.md) | User scenarios |
| [docs/preview-dialog.md](docs/preview-dialog.md) | Normalization preview dialog |

## Quick start

> **All commands must be run via `nix-shell --run "..."`** — `cargo`, `pnpm`, `uv` and other tooling are only available inside the nix-shell (defined in `shell.nix` at the repo root).
>
> **Do not run commands from an "already open" nix-shell session** after editing `shell.nix`. The `shellHook` (including `XDG_DATA_DIRS` / `GIO_EXTRA_MODULES` / `WEBKIT_DISABLE_DMABUF_RENDERER` — required for WebKit2GTK in Tauri to read GSettings correctly and avoid `devicePixelRatio`=negative, see [tauri #7354](https://github.com/tauri-apps/tauri/issues/7354)) only runs on shell entry. Each `nix-shell --run "..."` forks a fresh subshell and always picks up the up-to-date env. Running `pnpm tauri dev` "bare" in the current session breaks fonts and window metrics.

```bash
nix-shell --run "pnpm install"                                              # frontend deps
nix-shell --run "pnpm tauri dev"                                            # run the app
nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml"          # Rust tests
nix-shell --run "pnpm typecheck"                                            # TS typecheck
nix-shell --run "cd ttsd && uv run python -m pytest"                        # Python subprocess tests
```

## Project layout

```
/
├── src/              # React + TypeScript frontend (Vite + Mantine 8)
├── src-tauri/        # Rust backend
│   ├── src/
│   │   ├── pipeline/ # Text normalization (port of legacy Python pipeline)
│   │   ├── storage/  # JSON history + audio files
│   │   ├── tts/      # ttsd subprocess manager
│   │   ├── player/   # tauri-plugin-mpv wrapper
│   │   ├── commands/ # Tauri commands (#[tauri::command])
│   │   └── tray/     # System tray
│   └── tests/        # Rust integration tests (golden pipeline fixtures)
├── ttsd/             # Python subprocess (Silero TTS sidecar)
│   ├── pyproject.toml
│   └── ttsd/
│       ├── silero.py      # SileroEngine: load, synthesize
│       ├── timestamps.py  # Word-level timestamp estimation
│       ├── protocol.py    # request/response types
│       └── main.py        # main stdin→stdout JSON loop
├── docs/             # Project documentation
├── scripts/          # Utility scripts
├── shell.nix         # Nix dev environment (Rust + Node + Python)
└── flake.nix         # Production build via `nix build .#ruvox`
```

## Development rules

### General

- Code (identifiers, comments) is in English. User-facing strings (UI, notifications, logs visible to the user) are in Russian.
- No emoji in code or commit messages.
- Commit format: `<type>(<module>): <short desc>`, where `type` ∈ `{feat, fix, chore, refactor, docs, test, build}`. Commit messages (subject + body) must be in **English**.
- **Forbidden:** "Co-Authored-By: Claude …" or any mention of Claude in commits.
- Comments only when WHY is non-obvious (hidden invariant, workaround for a known bug). Do not comment WHAT.

### Documentation language

- The primary language for everything in the repo is English: `docs/`, `README.md`, `AGENTS.md`, `CLAUDE.md`, GitHub issues, PR descriptions, code comments.
- The only translated artifact is `README.ru.md` (Russian localization of `README.md`). Whenever `README.md` changes, `README.ru.md` must be updated in the same PR.
- User-facing UI strings (notifications, button labels, dialog text) stay in Russian — the product targets a Russian-speaking audience.

### Rust

- Edition 2021 (or newer if a dependency requires it).
- `tracing` for logging, `thiserror` for domain errors, `anyhow::Result` only at boundaries.
- No `unwrap` in production paths — use `?` + typed errors.
- `cargo fmt` and `cargo clippy` must be clean.

### TypeScript / React

- `strict: true` in tsconfig. Avoid `any` unless absolutely necessary.
- Function components only. Do not use `React.FC`.
- Hooks-first. No class components.
- Prettier for formatting.

### Mantine 8

- Styling via CSS Modules and the `classNames` prop.
- **Forbidden:** `sx`, `createStyles`, emotion, any legacy from Mantine 6/7.
- Forms: `@mantine/form` (not react-hook-form, not Formik).
- Notifications: `@mantine/notifications`.
- Hooks: `@mantine/hooks`.
- Modals: `@mantine/modals` (`modals.openConfirmModal` etc.).

### State

- No Redux. For global state use Zustand or React context. By default — props + `useState`.
- React Query is not needed — Tauri `invoke` fits naturally into `useEffect` + `useState`.

### Routing

- No router for now. Dialogs go through `@mantine/modals`.

### Python (ttsd)

- Python 3.12, managed by `uv`.
- Logs to stderr; JSON requests on stdin, JSON responses on stdout.
- `ruff check` and `pytest` must be green.

## TTS Pipeline: notes

### Mermaid diagrams

Mermaid blocks (` ```mermaid ... ``` `) **are not narrated**. The pipeline replaces them with the marker `"Тут мермэйд диаграмма"` to indicate that a diagram is present. The user can pause playback and inspect the diagram in the UI.

### English text

Silero TTS **cannot read English**. All English text must be transliterated to Cyrillic before being passed to the TTS engine:

- Wherever English may appear (URLs, code, headings, etc.), a **transliteration fallback** is mandatory.
- If text processing leaves English words behind, they must remain available for further normalization by the English normalizer.

### Pipeline implementation

The pipeline lives in `src-tauri/src/pipeline/` (Rust). Correctness is verified by golden fixtures in `src-tauri/tests/fixtures/pipeline/`.

## Tests

```bash
# Rust (all tests)
nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml"

# Rust pipeline golden tests
nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml --test golden"

# Python subprocess
nix-shell --run "cd ttsd && uv run python -m pytest"

# TypeScript
nix-shell --run "pnpm typecheck"
```
