# Changelog

Notable changes in RuVox, in chronological order.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versions follow [SemVer](https://semver.org/).

## [Unreleased]

### Added
- **Auto-detected source format in the preview dialog** — the format selector now defaults to «Авто», which classifies pasted text as HTML, Markdown or plain instead of always assuming Markdown; file and URL imports still preselect the format they know.
- **"Save audio as…" audio export** — the queue context menu exports an entry's audio to any folder via the native save dialog (#225).

## [0.4.0] — 2026-08-25 — Linux packages

### Added
- **Linux packages (`.deb`, `.AppImage`)** — release artifacts for Ubuntu 24.04-class systems, built by a GitHub Actions job. The `.deb` installs via `dpkg -i`; the AppImage runs directly on systems with FUSE and via `--appimage-extract-and-run` anywhere else.
- **TTS works out of the box on Linux** — packages bundle `espeak-ng-data` and a pinned `libonnxruntime.so` (sha256-verified at build time), so Silero (native) and Piper synthesize on a clean system without system-wide ONNX Runtime; previously Silero hung silently at startup.

### Fixed
- **Linux user data moved out of XDG cache** — history and audio live in XDG data/config dirs now, so cache cleaners cannot wipe them; corrupted `config.json` writes recover from `.bak` (#222).

## [0.3.1] — 2026-08-20

### Added
- **Update checks are logged** — both the background check and "Проверить обновления" in Settings now write their outcome (up to date / update available / failure reason) to the diagnostic log file (#207).

## [0.3.0] — 2026-08-19 — Windows support

### Added
- **"Silero (native)" engine** — a third TTS engine: Silero v5 runs in-process on ONNX Runtime (the `silero-native/` crate), no Python sidecar. The model bundle (~230 MB) downloads on demand from GitHub Releases with sha256 verification; full parity with the Python version (auto stress marks, homographs, ё). The Python Silero build remains as a fallback.
- **Windows 10 22H2+ / 11 (x86_64) support** — NSIS installer with an embedded WebView2 bootstrapper; bundled mpv, ONNX Runtime and the VC++ CRT are provisioned at build time (pinned resources, sha256-verified).
- **Auto-updates** — `tauri-plugin-updater`: checks from Settings and in the background, signed updates (private key in GitHub Secrets), manifest via `latest.json` from GitHub Releases.
- **Release pipeline** — pushing a `v*` tag runs a GitHub Actions job: builds on `windows-latest`, signs the updater artifacts, publishes a draft release with the installer and `latest.json`.

### Changed
- **Default engine is now "Silero (native)"** (aidar voice, 24000 Hz) instead of Piper. Existing configs are untouched: new defaults apply only to keys missing from `config.json`; if the model bundle is not downloaded, the app still starts on Piper and offers to download the bundle in Settings.

## [0.2.0] — 2026-04-20 — RuVox 2 rewrite

Full migration from PyQt6 + Python single-process to Tauri 2 + Rust + Python subprocess. Goal: keep the 0.1.x feature set, decouple UI ↔ TTS, and move the normalization pipeline to typed Rust with golden tests.

### Added
- **Tauri 2 shell** — native webview, no Python on the main thread.
- **React 18 + Mantine 8 UI** — functional components, CSS Modules, typed IPC.
- **Rust pipeline** — port of all normalizers (Number, English, Abbreviations, Code, URL, Symbol, CodeBlock) from `legacy/src/ruvox/tts_pipeline/` to Rust, verified by golden tests against legacy.
- **ttsd subprocess** — Silero TTS isolated in `ttsd/`, talking to the Rust backend over a stdin/stdout JSON protocol.
- **Storage service** — typed JSON history with separate timestamp files.
- **Preview dialog** (FF 1.1) — preview of the normalized text before synthesis for long inputs.
- **Edit mode** (FF 1.2) — edit text right in the viewer, persisting `edited_text` on the entry and using it on re-synthesis.
- **Word highlighting** — synchronized highlighting of the spoken word in markdown mode via `data-orig-start/end` attributes and binary search over WordTimestamp.
- **Mermaid rendering** — diagrams rendered with mermaid.js in the UI, click-to-zoom via Mantine Modal; replaced with a marker for TTS.
- **HTML format support** — dedicated render mode with sanitization (DOMPurify).
- **Settings dialog** — centralized settings (speaker, rate, hotkeys, cache).
- **Notifications** — `@mantine/notifications` for user-facing messages (TTS errors, edit saving, etc.).
- **Tray menu** — Read Now / Read Later / Open Settings.
- **Nix flake** — `nix build .#ruvox` builds the production binary with bundled `ttsd` and `mpv`.

### Changed
- **UI framework:** PyQt6 → React 18 + Mantine 8.
- **Main process:** Python + PyQt6 QApplication → Rust + Tauri 2 + webview.
- **Pipeline:** Python (moving target, slow tests) → Rust (typed, golden tests, ~10x faster).
- **TTS:** embedded Python in the UI process → `ttsd` subprocess with a JSON protocol (the UI survives a model warmup crash).
- **Player:** `python-mpv` → `tauri-plugin-mpv` (mpv process managed over IPC, scaletempo2 preserved).
- **Hotkeys:** `dasbus` + PyGObject → `@mantine/hooks::useHotkeys` (in-app) + Tauri global shortcuts (system-wide).
- **Storage:** SQLite layer via SQLAlchemy → JSON files with typed schemas in Rust.

### Removed
- `legacy/` — the old PyQt6 implementation temporarily kept as a reference; to be removed once feature parity is confirmed.

### Fixed
- TTS subprocess crash on startup (SIGSEGV in BERT TorchScript on homographs): `torch` is now imported at module level before spawning the worker thread.
- Word-highlighting listener leak on rapid entry switching: added an `editMode` guard in the subscribe effect; listeners are re-subscribed when leaving edit mode (not a blocker; follow-up — migrate to the canonical `Promise<UnlistenFn>[]` pattern as in `Player.tsx`).

### Developer notes
- All commands run inside `nix-shell` or `nix develop`.
- Rust edition 2021, `tracing` for logs, `thiserror` for domain errors, `anyhow` only at boundaries.
- TypeScript `strict: true`, no `React.FC`, CSS Modules (no `sx`/emotion/createStyles).
- Commit format: `<type>(<module>): <desc>`, `type ∈ {feat, fix, chore, refactor, docs, test, build}`. No emoji in code or commits.

## [0.1.x] — Legacy PyQt6

The PyQt6 implementation, original stack: Python 3.11 + PyQt6 + PyQt6-WebEngine + Silero TTS + python-mpv + dasbus. Kept in `legacy/` as a reference; removed once RuVox 2 reaches feature parity.
