# <img src="docs/images/logo.svg" width="40" align="top" alt=""/> RuVox

[English version](./README.md)

[![CI](https://github.com/xilec/RuVox/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/xilec/RuVox/actions/workflows/ci.yml)
![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-green)

Desktop-приложение для озвучивания технических текстов на русском языке.

Нормализует английские термины, аббревиатуры, код, числа, URL и передаёт результат в [Piper](https://github.com/rhasspy/piper) (in-process, через `piper-rs`, основной движок) или, опционально, в [Silero TTS](https://github.com/snakers4/silero-models) (out-of-process, через Python-сайдкар `ttsd`). В отличие от голого TTS, RuVox умеет читать `getUserData()` как «гет юзер дата», `API` как «эй пи ай», `/api/v2/users` как путь, а не по буквам.

![Скриншот RuVox](docs/images/screenshot.png)

## Стек

| Слой | Технология |
|------|------------|
| Shell | [Tauri 2](https://tauri.app/) (Rust + нативный webview) |
| Frontend | React 18 + TypeScript 5 + [Mantine 8](https://mantine.dev/) |
| Backend | Rust (pipeline нормализации, storage, TTS-менеджер) |
| TTS | Piper (in-process, `piper-rs` + `onnxruntime`); Silero (опционально, Python 3.12 subprocess `ttsd`) |
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
- **Nix:** рекомендуется — всё окружение (Rust, Node, Python, Tauri-deps) собирается из `flake.nix` (dev-shell живёт в `nix/devshell.nix`).
- **Без Nix:** дистрибутив Linux, в котором есть `webkit2gtk-4.1` (Ubuntu 24.04+, Debian 13+, Fedora 40+, Arch). Подробная пошаговая инструкция по сборке: [docs/install.md](docs/install.md) (на английском). Python 3.12 + `uv` нужны только если хотите Silero (сайдкар `ttsd`).

## Dev-окружение

```bash
# Интерактивная оболочка
nix develop
pnpm install
pnpm tauri dev

# Или одну команду без входа в оболочку
nix develop -c pnpm install
nix develop -c pnpm tauri dev
```

Все команды в документации подразумевают запуск внутри `nix develop` (либо через `nix develop -c ...`).

## Сборка production-бинаря

```bash
# По-умолчанию (slim) — только Piper, без Python/torch/Silero в closure.
nix build .#ruvox
./result/bin/ruvox

# Опционально (full) — со встроенным сайдкаром ttsd, чтобы был доступен Silero.
nix build .#ruvox-with-silero
./result/bin/ruvox
```

Оба варианта собирают release-бинарь Tauri и оборачивают его через `wrapProgram` (runtime `LD_LIBRARY_PATH` + `GIO_EXTRA_MODULES`); `mpv` в обоих случаях попадает в `PATH`. Вариант `.#ruvox-with-silero` дополнительно кладёт в `PATH` бинарь `ttsd` (Silero subprocess). Slim-вариант его не содержит — на runtime в Settings опция Silero окрашена серым.

> **Первый запуск `nix build`:** derivation `frontend` использует `pnpm.fetchDeps` с `lib.fakeHash` — Nix упадёт с hash mismatch, напишет реальный hash; его нужно подставить в `flake.nix` и повторить build. Это стандартная процедура pnpm2nix.

## Тесты

```bash
pnpm typecheck                                                  # TypeScript
cargo test --manifest-path src-tauri/Cargo.toml                 # Rust (включая golden-тесты pipeline)
cargo test --manifest-path src-tauri/Cargo.toml --test golden   # только golden-тесты
cd ttsd && uv run python -m pytest                              # Python subprocess
```

## Документация

| Файл | Описание |
|------|----------|
| [AGENTS.md](AGENTS.md) | Правила разработки, структура проекта, соглашения |
| [docs/install.md](docs/install.md) | Сборка из исходников на Linux без Nix (Ubuntu 24.04+, на английском) |
| [docs/ipc-contract.md](docs/ipc-contract.md) | Tauri-команды, события, протокол ttsd |
| [docs/storage-schema.md](docs/storage-schema.md) | Схема `history.json`, timestamps, config |
| [CHANGELOG.md](CHANGELOG.md) | Хронология изменений |

## Лицензия

GPL-3.0 — см. [LICENSE.md](LICENSE.md).
