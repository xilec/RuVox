# Changelog

Хронология заметных изменений RuVox.

Формат: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), версии по [SemVer](https://semver.org/).

## [0.2.0] — 2026-04-20 — RuVox 2 rewrite

Полный переход с PyQt6 + Python-моно-процесса на Tauri 2 + Rust + Python-subprocess. Цель — сохранить функциональность 0.1.x, убрать жёсткую связность UI ↔ TTS, перевести pipeline нормализации на типизированный Rust с golden-тестами.

### Added
- **Tauri 2 shell** — нативный webview без Python в main-thread.
- **React 18 + Mantine 8 UI** — функциональные компоненты, CSS Modules, typed IPC.
- **Rust-pipeline** — порт всех normalizer'ов (Number, English, Abbreviations, Code, URL, Symbol, CodeBlock) из `legacy/src/ruvox/tts_pipeline/` на Rust, проверка корректности golden-тестами против legacy.
- **ttsd subprocess** — Silero TTS изолирован в `ttsd/`, общается с Rust-бэкендом через stdin/stdout JSON-protocol.
- **Storage service** — типизированная JSON-история с отдельными файлами timestamps.
- **Preview dialog** (FF 1.1) — предпросмотр нормализованного текста перед синтезом для длинных входов.
- **Edit mode** (FF 1.2) — правка текста прямо в viewer'е с сохранением `edited_text` на entry и использованием при re-synth.
- **Word highlighting** — синхронная подсветка читаемого слова в markdown-режиме с использованием `data-orig-start/end` атрибутов и binary-search по WordTimestamp.
- **Mermaid rendering** — диаграммы рендерятся через mermaid.js в UI, click-to-zoom через Mantine Modal; в TTS заменяются маркером.
- **HTML format support** — отдельный режим рендера с sanitization (DOMPurify).
- **Settings dialog** — централизованные настройки (speaker, rate, hotkeys, cache).
- **Notifications** — `@mantine/notifications` для user-facing сообщений (ошибки TTS, сохранение правок и т. п.).
- **Tray menu** — Read Now / Read Later / Open Settings.
- **Nix flake** — `nix build .#ruvox` собирает production-бинарь с bundled `ttsd` и `mpv`.

### Changed
- **UI-фреймворк:** PyQt6 → React 18 + Mantine 8.
- **Главный процесс:** Python + PyQt6 QApplication → Rust + Tauri 2 + webview.
- **Pipeline:** Python (moving target, slow тесты) → Rust (typed, golden-tests, ~10x быстрее).
- **TTS:** embedded Python в UI-процессе → subprocess `ttsd` с JSON-protocol (UI не падает, если модель не прогрелась).
- **Player:** `python-mpv` → `tauri-plugin-mpv` (управление mpv-процессом через IPC, scaletempo2 сохранён).
- **Hotkeys:** `dasbus` + PyGObject → `@mantine/hooks::useHotkeys` (для in-app) + Tauri global shortcuts (для системных).
- **Storage:** SQLite-слой через SQLAlchemy → JSON-файлы с типизированными схемами в Rust.

### Removed
- `legacy/` — старая PyQt6-реализация временно сохранена как reference, удалится после подтверждения фичи-паритета.

### Fixed
- TTS subprocess crash при старте (SIGSEGV в BERT TorchScript при омографах): `torch` теперь импортируется на уровне модуля до spawn worker thread.
- Утечка listener'ов word-highlighting при быстрой смене entry: добавлен `editMode` guard в subscribe-effect, listeners пере-подписываются при выходе из edit mode (не blocker, follow-up — перевод на канонический `Promise<UnlistenFn>[]` паттерн как в `Player.tsx`).

### Developer notes
- Все команды запускаются внутри `nix-shell` или `nix develop`.
- Rust edition 2021, `tracing` для логов, `thiserror` для доменных ошибок, `anyhow` только на границах.
- TypeScript `strict: true`, без `React.FC`, CSS Modules (без `sx`/emotion/createStyles).
- Commit-формат: `<type>(<module>): <desc>`, `type ∈ {feat, fix, chore, refactor, docs, test, build}`. Никаких emoji в коде и коммитах.

## [0.1.x] — Legacy PyQt6

PyQt6-реализация, исходный стек: Python 3.11 + PyQt6 + PyQt6-WebEngine + Silero TTS + python-mpv + dasbus. Сохранена в `legacy/` как reference; снимается после достижения фичи-паритета RuVox 2.
