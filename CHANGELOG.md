# Changelog

Notable changes in RuVox, in chronological order.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versions follow [SemVer](https://semver.org/).

## [Unreleased]

### Changed
- **Code block narration is now a working setting** — the "Озвучка блоков
  кода" selector in Settings («Кратко» / «Читать полностью») drives the
  pipeline live: brief mode (new default) replaces fenced code blocks with a
  short marker sentence, full mode reads the code out; the change applies
  without restarting the app (#89).

### Removed
- **The inline `<!-- ruvox-code: … -->` directives** — code block narration
  is controlled solely by the Settings option (#89).
- **The dead `read_operators` config field** — it never affected synthesis;
  old configs carrying it keep parsing (#89).

## [0.5.1] — 2026-08-30

### Added
- **Linux AppImage bundles the `mpv` player** — the AppImage now works on a
  clean system without installing anything; the `.deb` keeps installing the
  system `mpv` automatically via `Depends` (#265).

## [0.5.0] — 2026-08-30 — Localization, import and export

### Added
- **Russian and English interface** — the whole UI is localized; the language
  is picked in Settings (Russian by default), and backend and engine errors
  are shown as readable localized messages instead of raw error strings (#240).
- **Import from files and web pages** — «Добавить» becomes a split button with
  «Файл…», «Файл с кодировкой…» and «По ссылке…», and dragging a .txt/.md/.html
  file or a link onto the window adds a new entry; text encoding is
  auto-detected (BOM, UTF-8, then CP1251/KOI8-R/CP866 and other Cyrillic
  encodings) with a manual override for misdetected files (#224).
- **Auto-detected source format in the preview dialog** — the format selector now defaults to «Авто», which classifies pasted text as HTML, Markdown or plain instead of always assuming Markdown; file and URL imports still preselect the format they know.
- **"Save audio as…" audio export** — the queue context menu exports an entry's audio to any folder via the native save dialog (#225).
- **WAV audio export** — «Сохранить аудио как…» gains a format chooser in the save dialog (WAV default, Ogg Opus alternative), converting an Opus recording to editable 16-bit PCM WAV on export; the cache keeps the Opus original (#252).
- **Voiceover parameters in the history** — the queue context menu gains «Параметры записи…», a read-only view of the source, engine, voice, sample rate, model and settings that produced each recording (#243).
- **Linux AppImage auto-updates** — AppImage installs now check GitHub releases in-app and self-update with signature verification; .deb and source builds get no update UI (#226).
- **Linux packages need `mpv`** — the .deb installs the player binary automatically as a dependency; AppImage users install it once via the system package manager (`sudo apt install mpv`). Bundling `mpv` into the AppImage is planned for 0.5.1 (#265).

### Changed
- **Playback speed up to 3.0× and restored on startup** — the speed limit
  rises from 2.0× to 3.0×, and the saved speech rate is applied to the player
  itself on startup, not just to the slider (#227).
- **Regeneration asks before overwriting** — «Перегенерировать аудио» now opens the normalization preview first: the old audio is deleted only after you confirm, cancelling keeps it, and «Читать сейчас» plays the fresh audio.

### Fixed
- **Silero (native) recordings are stored as Opus again** — the Opus transcode rejected the engine's 16-bit WAV, so its entries stayed as lossy-playback `.wav` fallbacks; existing `.wav` recordings are converted to `.opus` on the next launch (#254).

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
