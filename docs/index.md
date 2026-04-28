# RuVox

**RuVox** is a desktop application for voicing technical Russian texts via Silero TTS. It normalizes English terms, abbreviations, code, numbers, and URLs before passing them to the TTS engine, so the synthesizer can correctly read material it wasn't designed for.

## Problem → Solution

Silero TTS cannot correctly pronounce:
- English words and IT terms (`feature` → silence or distortion)
- Abbreviations (`API`, `HTTP`, `JSON`)
- URLs, emails, IP addresses, paths
- Code identifiers (`getUserData`, `my_variable`)
- Special characters and operators (`->`, `>=`, `!=`)

```
"Вызови getUserData() через API" → "Вызови гет юзер дата через эй пи ай"
```

## Features

- **Add button** — copy text to the clipboard, press Add → the entry lands in the queue and gets synthesized.
- **Preview dialog** — for long texts a separate floating window shows the original and normalized version side-by-side; you can edit the original before synthesis.
- **Edit mode** — edit `original_text` directly in the viewer; changes are saved on the entry.
- **Queue** — list of all entries with status badges (`pending` / `processing` / `ready` / `playing` / `error`).
- **Word highlight** — synchronized highlighting of the word being read in markdown mode, via binary search over `WordTimestamp`.
- **Mermaid** — diagrams render in the UI; for TTS they are replaced with the marker "тут мермэйд диаграмма".
- **System tray** — close-to-tray, warm mpv re-init when the window is shown.

## Stack

| Layer | Technology |
|------|------------|
| Shell | [Tauri 2](https://tauri.app/) (Rust + native webview) |
| Frontend | React 18 + TypeScript 5 + [Mantine 8](https://mantine.dev/) |
| Backend | Rust (normalization pipeline, storage, TTS manager, player wrapper) |
| TTS | Python 3.12 subprocess `ttsd`, a wrapper around [Silero TTS](https://github.com/snakers4/silero-models) |
| Audio | [`tauri-plugin-mpv`](https://crates.io/crates/tauri-plugin-mpv) (libmpv with `scaletempo2`) |
| Build environment | Nix (`shell.nix` + `flake.nix`) |

## Documentation

### Architecture and history

- [RewriteNotes.md](../RewriteNotes.md) — architectural decisions and the rationale behind the stack choice.
- [RewriteTaskPlan.md](../RewriteTaskPlan.md) — detailed rewrite task plan, dependency graph.
- [task_history.md](../task_history.md) — task execution log.
- [CHANGELOG.md](../CHANGELOG.md) — version chronology.

### Reference

- [IPC contract](ipc-contract.md) — Tauri commands, events, ttsd JSON protocol.
- [Storage schema](storage-schema.md) — `history.json`, `config.json`, `{uuid}.timestamps.json`, `{uuid}.wav`.
- [Normalization pipeline](pipeline.md) — processing stages, normalizers, golden tests.
- [UI components](ui.md) — React app structure, components, styling.
- [Preview dialog (FF 1.1)](preview-dialog.md) — behavior, settings, interaction flow.

### Use cases and development

- [Use cases](use-cases.md) — user scenarios: adding text, plain/markdown modes, mermaid, word highlight.
- [Development](development.md) — environment, commands, debugging.
- [Contributing](contributing.md) — how to add a term to the dictionary, commit and style rules.

## License

GPL-3.0 — see [LICENSE.md](../LICENSE.md).
