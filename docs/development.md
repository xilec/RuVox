# Development

Guide to setting up the environment and working on RuVox.

## Requirements

- **Linux** (X11 or Wayland). macOS/Windows are not supported.
- **Nix** (recommended) — provides a fully reproducible environment: Rust toolchain, Node, pnpm, Python 3.12, uv, libmpv, webkit2gtk, and Tauri's system libraries.
- **Without Nix** — you'll have to manually install Rust stable + Node 20 + Python 3.12 + system deps (see `buildInputs` in `shell.nix`: `webkitgtk_4_1`, `libsoup_3`, `gtk3`, `libmpv`, `pipewire`/`pulseaudio`, `libappindicator-gtk3`, `librsvg`, `pkg-config`).

## Environment

```bash
# Via flake (recommended)
nix develop
pnpm install
pnpm tauri dev

# Or via the classic shell.nix
nix-shell --run "pnpm install"
nix-shell --run "pnpm tauri dev"
```

> **Important:** all commands (`cargo`, `pnpm`, `uv`, `tauri`, `ruff`, `pytest`) are only available inside `nix develop` / `nix-shell`. Don't run commands from an "already open" nix-shell session after editing `shell.nix` — `shellHook` (including `XDG_DATA_DIRS`, `GIO_EXTRA_MODULES`, `WEBKIT_DISABLE_DMABUF_RENDERER`) is only executed when entering the shell. Each `nix-shell --run "..."` forks a fresh subshell and gets the up-to-date env.

## Project structure

```
/
├── src/                    # React + TypeScript frontend (Vite + Mantine 8)
│   ├── components/         # AppShell, QueueList, Player, TextViewer, icons
│   ├── dialogs/            # PreviewDialog (FF 1.1), Settings
│   ├── lib/                # tauri.ts (typed wrappers), markdown, html, mermaid, wordHighlight, errors
│   └── stores/             # Zustand store selectedEntry
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── pipeline/       # Normalization: tracked_text, normalizers/, html_extractor, constants
│   │   ├── storage/        # JSON history + audio files (schema in storage/schema.rs)
│   │   ├── tts/            # ttsd subprocess manager
│   │   ├── player/         # tauri-plugin-mpv wrapper (ensure_mpv_alive, seek-suppress)
│   │   ├── commands/       # Tauri commands (#[tauri::command])
│   │   ├── tray/           # System tray (close-to-tray, "Выход")
│   │   ├── state.rs        # AppState
│   │   └── lib.rs          # Tauri::Builder entry point
│   └── tests/
│       ├── fixtures/pipeline/  # Golden fixtures (37 cases × 3 files)
│       └── golden.rs           # Golden integration test
├── ttsd/                   # Python subprocess (Silero TTS sidecar)
│   ├── pyproject.toml
│   └── ttsd/
│       ├── silero.py       # SileroEngine: load, synthesize
│       ├── timestamps.py   # Word timestamp estimation
│       ├── protocol.py     # Request/response types
│       └── main.py         # Main stdin→stdout JSON loop
├── docs/                   # Documentation (this directory)
├── scripts/                # Utilities (launch-prod, rebuild_prod)
├── shell.nix               # Nix environment (Rust + Node + Python + Tauri deps)
└── flake.nix               # Flake (for nix build .#ruvox and nix develop)
```

## Commands

### Run

```bash
nix-shell --run "pnpm tauri dev"            # dev mode with hot reload
nix build .#ruvox && ./result/bin/ruvox     # production binary
```

### Tests

```bash
nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml"           # all Rust tests
nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml --test golden"  # golden tests only
nix-shell --run "pnpm typecheck"                                            # TypeScript strict
nix-shell --run "cd ttsd && uv run python -m pytest"                        # Python subprocess
```

### Production build

```bash
nix build .#ruvox
./result/bin/ruvox
```

`.#ruvox` builds the Tauri release binary, wraps it via `wrapProgram` (runtime `LD_LIBRARY_PATH` + `GIO_EXTRA_MODULES`), and links `ttsd` (the Silero subprocess) and `mpv` into `PATH`.

> **First `nix build` run:** the `frontend` derivation uses `pnpm.fetchDeps` with `lib.fakeHash` — Nix will fail with a hash mismatch and print the real hash; substitute it into `flake.nix` and rerun the build. This is the standard pnpm2nix procedure.

## Code rules

### General

- Identifiers and comments are in English. User-facing strings (UI, notifications) are in Russian.
- No emoji in code or commit messages.
- Commit format: `<type>(<module>): <short desc>`, `type ∈ {feat, fix, chore, refactor, docs, test, build}`.
- **Forbidden:** "Co-Authored-By: Claude …" or any mention of Claude in a commit.
- Comments only when **WHY** is non-obvious (a hidden invariant, a workaround for a known bug). Don't comment **WHAT**.

### Rust

- Edition 2021 (or newer if a dependency requires it).
- `tracing` for logs, `thiserror` for domain errors, `anyhow::Result` only at boundaries.
- `unwrap` is forbidden in production paths — use `?` + typed errors.
- `cargo fmt` and `cargo clippy` must be clean.

### TypeScript / React

- `strict: true` in tsconfig. No `any` unless absolutely necessary.
- Functional components only. Don't use `React.FC`.
- Hooks-first. No class components.
- Prettier for formatting.

### Mantine 8

- Styling via **CSS Modules** and the `classNames` prop.
- **Forbidden:** `sx`, `createStyles`, `emotion`, any Mantine 6/7 legacy.
- Forms: `@mantine/form` (not react-hook-form, not Formik).
- Notifications: `@mantine/notifications`.
- Hooks: `@mantine/hooks`.
- Modals: `@mantine/modals` (`modals.openConfirmModal`, etc.).

### State

- No Redux. Global state — Zustand or React context. By default — props + `useState`.
- React Query is not needed — Tauri invoke fits well with `useEffect` + `useState`.

### Routing

- No router. Dialogs go through `@mantine/modals` or non-modal floating windows (`react-rnd`, see PreviewDialog).

### Python (ttsd)

- Python 3.12, `uv`-managed.
- Logs to stderr, JSON requests on stdin, JSON responses on stdout.
- `ruff check` and `pytest` must be green.

## Debugging

### Logs

- **Tauri (Rust)** — via `tracing` to the process's stderr. In dev mode they're visible in the terminal where `pnpm tauri dev` is running.
- **ttsd** — Python logs go to stderr → Rust proxies them to `tracing::info!("ttsd: ...")`.
- **Frontend** — webview DevTools (right click → Inspect Element in the application window).

### Webview DevTools

In debug builds Tauri's webview enables DevTools. For prod builds you have to either explicitly allow them in `tauri.conf.json` or build with the `devtools` feature.

### Reading `~/.cache/ruvox/`

The storage cache lives in `~/.cache/ruvox/`:
- `history.json` — list of `TextEntry`. You can open it manually.
- `audio/{uuid}.wav` — audio. Plays in any player.
- `audio/{uuid}.timestamps.json` — word timings.
- `config.json` — `UIConfig`.

See [Storage schema](storage-schema.md) for details.

## Workflow

```bash
git checkout -b feat/short-description     # separate branch
# ... edits ...
nix-shell --run "cargo test ..."           # run tests
git commit -m "feat(<module>): <desc>"     # commit
git push -u origin feat/short-description  # push (after approval)
```

New features and fixes follow the standard feature-branch flow.
