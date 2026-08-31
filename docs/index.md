# RuVox

**RuVox** is a desktop application for voicing technical Russian texts with local TTS engines: native Silero v5 on ONNX Runtime (the `silero-native` crate, default), Piper (fallback), or Silero via the Python `ttsd` sidecar (fallback). It normalizes English terms, abbreviations, code, numbers, and URLs before passing them to the TTS engine, so the synthesizer can correctly read material it wasn't designed for.

## Problem → Solution

A bare Russian TTS engine cannot correctly pronounce:
- English words and IT terms (`feature` → silence or distortion)
- Abbreviations (`API`, `HTTP`, `JSON`)
- URLs, emails, IP addresses, paths
- Code identifiers (`getUserData`, `my_variable`)
- Special characters and operators (`->`, `>=`, `!=`)

```
"Вызови getUserData() через API" → "Вызови гет юзер дата через эй пи ай"
```

## Features

- **Adding text** — paste, drag a `.txt`/`.md`/`.html` file or a link onto the window, or use the split Add button («Файл…», «Файл с кодировкой…», «По ссылке…»); encoding is auto-detected (BOM, UTF-8, CP1251/KOI8-R/CP866, …) with a manual override.
- **Preview dialog** — a separate floating window shows the original and normalized version side-by-side before synthesis; the source format selector (auto / plain / Markdown / HTML) lives there, and the original can be edited before synthesis.
- **Edit mode** — edit `original_text` directly in the viewer; changes are saved on the entry.
- **Queue** — all entries with status badges (`pending` / `processing` / `ready` / `playing` / `error`); the context menu regenerates audio, exports it («Сохранить аудио как…»: WAV / Ogg Opus) and shows the parameters each recording was made with (also on double-click).
- **Word highlight** — synchronized highlighting of the word being read in markdown mode, via binary search over `WordTimestamp`.
- **Code block modes** — «Кратко» (default) replaces fenced code blocks with a marker sentence; «Читать полностью» narrates them in full; switched in Settings, applies live.
- **Mermaid** — diagrams render in the UI; for TTS they are replaced with the marker "тут мермэйд диаграмма".
- **Playback** — mpv-based player with speed up to 3.0×, persisted across restarts.
- **Localization** — Russian and English UI, selected in Settings.
- **System tray** — close-to-tray, warm mpv re-init when the window is shown.
- **Auto-updates** — the Windows installer and the Linux AppImage check GitHub Releases and self-update with signature verification.

## Stack

| Layer | Technology |
|------|------------|
| Shell | [Tauri 2](https://tauri.app/) (Rust + native webview) |
| Frontend | React 18 + TypeScript 5 + [Mantine 8](https://mantine.dev/) |
| Backend | Rust (normalization pipeline, storage, TTS manager, player wrapper) |
| TTS | native Silero v5 (in-process, ONNX Runtime, `silero-native` crate, default); Piper (in-process, `piper-rs`, fallback); Silero via Python 3.12 subprocess `ttsd` (fallback) |
| Audio | [`tauri-plugin-mpv`](https://crates.io/crates/tauri-plugin-mpv) (libmpv with `scaletempo2`) |
| Build environment | Nix (`flake.nix`; dev shell in `nix/devshell.nix`) |

## Documentation

### Architecture and history

- [CHANGELOG.md](../CHANGELOG.md) — version chronology.
- [openspec/changes/archive/](../openspec/changes/archive/) — archived change proposals (the audit history of behavior changes).

### Behavior specs (OpenSpec)

Current behavior is specified in [openspec/specs/](../openspec/specs/) — the single source of truth:

- **Backend:** `text-pipeline`, `position-mapping`, `text-import`, `storage`, `ipc-commands`, `ttsd-protocol`, `silero-native-engine`, `logging`
- **Frontend / UX:** `ui`, `preview-dialog`, `queue-lifecycle`, `playback`, `text-display`, `word-highlight`, `tray`, `html-ingestion`, `viewer-copy-actions`, `auto-update`
- **Packaging / runtime:** `windows-installer`, `linux-runtime`, `windows-runtime`

### Development

- [Development](development.md) — environment, commands, debugging.
- [Contributing](contributing.md) — how to add a term to the dictionary, commit and style rules.

## License

GPL-3.0 — see [LICENSE.md](../LICENSE.md).
