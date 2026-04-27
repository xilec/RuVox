# <img src="docs/images/logo.svg" width="40" align="top" alt=""/> RuVox

[![CI](https://github.com/xilec/RuVox/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/xilec/RuVox/actions/workflows/ci.yml)
![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-green)

Desktop-приложение для озвучивания технических текстов на русском языке.

Нормализует английские термины, аббревиатуры, код, числа, URL и передаёт результат в [Silero TTS](https://github.com/snakers4/silero-models). В отличие от голого TTS, RuVox умеет читать `getUserData()` как «гет юзер дата», `API` как «эй пи ай», `/api/v2/users` как путь, а не по буквам.

![Скриншот RuVox](docs/images/screenshot.png)

## Стек

| Слой | Технология |
|------|------------|
| Shell | [Tauri 2](https://tauri.app/) (Rust + нативный webview) |
| Frontend | React 18 + TypeScript 5 + [Mantine 8](https://mantine.dev/) |
| Backend | Rust (pipeline нормализации, storage, TTS-менеджер) |
| TTS | Python 3.12 subprocess `ttsd`, обёртка над Silero TTS |
| Аудио | `tauri-plugin-mpv` (libmpv с `scaletempo2`) |

## Возможности

- **Нормализация** — английский (camelCase/snake_case), аббревиатуры, числа, даты, URL, email, код.
- **Markdown + HTML** — рендер и озвучивание с сохранением смысла.
- **Mermaid-диаграммы** — визуализация в UI; для TTS заменяются маркером «тут мермэйд диаграмма».
- **Подсветка слов** — синхронная подсветка читаемого слова в тексте во время воспроизведения.
- **Preview-диалог** — предпросмотр нормализованного текста до синтеза.
- **Системный трей** — close-to-tray, фоновый режим.

## Требования

- **ОС:** Linux (X11 или Wayland).
- **Nix:** рекомендуется — всё окружение (Rust, Node, Python, Tauri-deps) собирается из `shell.nix` / `flake.nix`.
- **Без Nix:** вручную установить Rust stable + Node 20 + Python 3.12 + system-deps Tauri 2 (webkitgtk 4.1, libsoup 3, gtk3, libmpv, pipewire/pulseaudio) — см. `buildInputs` в `shell.nix`.

## Dev-окружение

```bash
# Через flake (рекомендуется)
nix develop
pnpm install
pnpm tauri dev

# Или через классический shell.nix
nix-shell --run "pnpm install"
nix-shell --run "pnpm tauri dev"
```

Все команды в документации подразумевают запуск внутри `nix develop` / `nix-shell`.

## Сборка production-бинаря

```bash
nix build .#ruvox
./result/bin/ruvox
```

`.#ruvox` собирает release-бинарь Tauri, оборачивает его через `wrapProgram` (runtime LD_LIBRARY_PATH + GIO_EXTRA_MODULES), линкует `ttsd` (Silero-subprocess) и `mpv` в `PATH`.

> **Первый запуск `nix build`:** derivation `frontend` использует `pnpm.fetchDeps` с `lib.fakeHash` — Nix упадёт с hash mismatch, напишет реальный hash; его нужно подставить в `flake.nix` и повторить build. Это стандартная процедура pnpm2nix.

## Тесты

```bash
pnpm typecheck                                            # TypeScript
cargo test --manifest-path src-tauri/Cargo.toml           # Rust (включая golden-тесты pipeline)
cargo test --manifest-path src-tauri/Cargo.toml --test golden   # только golden-тесты
cd ttsd && uv run python -m pytest                        # Python subprocess
```

## Документация

| Файл | Описание |
|------|----------|
| [AGENTS.md](AGENTS.md) | Правила разработки, структура проекта, соглашения |
| [docs/ipc-contract.md](docs/ipc-contract.md) | Tauri-команды, события, протокол ttsd |
| [docs/storage-schema.md](docs/storage-schema.md) | Схема history.json, timestamps, config |
| [CHANGELOG.md](CHANGELOG.md) | Хронология изменений |

## Лицензия

GPL-3.0 — см. [LICENSE.md](LICENSE.md).
