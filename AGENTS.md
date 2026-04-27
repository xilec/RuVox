# RuVox - Инструкции для разработки

> **ВАЖНО:** Всегда отвечай на русском языке.

## Обзор проекта

**RuVox 2.0** — desktop-приложение для озвучивания технических текстов на русском языке.

**Стек:**
- **Shell:** Tauri 2 (Rust-based desktop shell с нативным webview)
- **Frontend:** React 18 + TypeScript 5 + Mantine 8
- **Backend:** Rust (pipeline нормализации, storage, subprocess-менеджер TTS, обёртка плеера)
- **TTS-движок:** Python-subprocess `ttsd`, оборачивающий Silero TTS (запускается как sidecar-процесс)

**Назначение** не изменилось: нормализовать технический текст (API, URL, идентификаторы кода, числа) перед передачей в Silero TTS, который не умеет читать английский и спецсимволы.

### Проблема → Решение

```
"Вызови getUserData() через API"
        ↓
"Вызови гет юзер дата через эй пи ай"
```

## Документация

| Файл / Раздел | Описание |
|---------------|----------|
| [docs/ipc-contract.md](docs/ipc-contract.md) | IPC-контракт: Tauri-команды, события, протокол ttsd |
| [docs/storage-schema.md](docs/storage-schema.md) | Схема хранилища: history.json, timestamps, config |
| [docs/pipeline.md](docs/pipeline.md) | Этапы нормализации текста |
| [docs/ui.md](docs/ui.md) | Структура фронтенда |
| [docs/use-cases.md](docs/use-cases.md) | Пользовательские сценарии |
| [docs/preview-dialog.md](docs/preview-dialog.md) | Preview-диалог нормализации |

## Быстрый старт

> **Все команды запускаются через `nix-shell --run "..."`** — `cargo`, `pnpm`, `uv` и прочие инструменты доступны только внутри nix-shell (определён в `shell.nix` в корне).
>
> **Не запускай команды из «уже открытой» nix-shell-сессии** после правок `shell.nix`. `shellHook` (в т.ч. `XDG_DATA_DIRS` / `GIO_EXTRA_MODULES` / `WEBKIT_DISABLE_DMABUF_RENDERER` — нужны чтобы WebKit2GTK в Tauri корректно читал GSettings и не выдавал `devicePixelRatio`=negative, см. [tauri #7354](https://github.com/tauri-apps/tauri/issues/7354)) запускается только при входе в shell. Каждый `nix-shell --run "..."` форкает свежий subshell → всегда получает актуальный env. Запуск `pnpm tauri dev` «голым» в текущей сессии ломает шрифты/метрики окна.

```bash
nix-shell --run "pnpm install"                                              # фронт-зависимости
nix-shell --run "pnpm tauri dev"                                            # запуск приложения
nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml"          # Rust-тесты
nix-shell --run "pnpm typecheck"                                            # проверка типов TS
nix-shell --run "cd ttsd && uv run python -m pytest"                        # тесты Python-subprocess
```

## Структура проекта

```
/
├── src/              # React + TypeScript frontend (Vite + Mantine 8)
├── src-tauri/        # Rust backend
│   ├── src/
│   │   ├── pipeline/ # Нормализация текста (port из legacy Python-pipeline)
│   │   ├── storage/  # История JSON + аудиофайлы
│   │   ├── tts/      # Менеджер ttsd-subprocess
│   │   ├── player/   # Обёртка tauri-plugin-mpv
│   │   ├── commands/ # Tauri-команды (#[tauri::command])
│   │   └── tray/     # Системный трей
│   └── tests/        # Rust integration tests (golden-фикстуры pipeline)
├── ttsd/             # Python-subprocess (Silero TTS sidecar)
│   ├── pyproject.toml
│   └── ttsd/
│       ├── silero.py      # SileroEngine: load, synthesize
│       ├── timestamps.py  # Оценка временных меток слов
│       ├── protocol.py    # Типы request/response
│       └── main.py        # Главный цикл stdin→stdout JSON
├── docs/             # Проектная документация
├── scripts/          # Утилитарные скрипты
├── shell.nix         # Nix-окружение (Rust + Node + Python)
└── flake.nix         # Production-сборка через nix build .#ruvox
```

## Правила разработки

### Общие

- Код (идентификаторы, комментарии) — на английском. Пользовательские строки (UI, уведомления, логи) — на русском.
- Никаких emoji в коде и коммит-сообщениях.
- Формат коммитов: `<type>(<module>): <short desc>`, где `type` ∈ `{feat, fix, chore, refactor, docs, test, build}`. Сообщения коммитов (subject + body) пишутся **на английском**.
- **Запрещено:** «Co-Authored-By: Claude …» и любое упоминание Claude в коммите.
- Комментарии — только если WHY неочевиден (скрытый инвариант, обход известного бага). Не комментировать WHAT.

### Rust

- Edition 2021 (или новее, если требует зависимость).
- `tracing` для логов, `thiserror` для доменных ошибок, `anyhow::Result` — только на границах.
- Запрещён `unwrap` в production-путях — использовать `?` + типизированные ошибки.
- `cargo fmt` и `cargo clippy` должны быть чистыми.

### TypeScript / React

- `strict: true` в tsconfig. Без `any` без крайней необходимости.
- Только функциональные компоненты. Не использовать `React.FC`.
- Hooks-first. Без class-компонентов.
- Prettier для форматирования.

### Mantine 8

- Стилизация через CSS Modules и prop `classNames`.
- **Запрещено:** `sx`, `createStyles`, emotion, любое легаси из Mantine 6/7.
- Формы: `@mantine/form` (не react-hook-form, не Formik).
- Нотификации: `@mantine/notifications`.
- Хуки: `@mantine/hooks`.
- Модалки: `@mantine/modals` (`modals.openConfirmModal` и т.п.).

### State

- Без Redux. Для глобального состояния — Zustand или React context. По умолчанию — props + `useState`.
- React Query не нужен — Tauri-invoke нормально ложится в `useEffect` + `useState`.

### Routing

- Без router на старте. Диалоги через `@mantine/modals`.

### Python (ttsd)

- Python 3.12, `uv`-managed.
- Логи на stderr, JSON-запросы на stdin, JSON-ответы на stdout.
- `ruff check` и `pytest` должны быть зелёными.

## TTS Pipeline: особенности

### Mermaid-диаграммы

Mermaid-блоки (` ```mermaid ... ``` `) **не озвучиваются**. Pipeline заменяет их маркером `"Тут мермэйд диаграмма"`, чтобы обозначить наличие диаграммы. Пользователь может приостановить воспроизведение и рассмотреть диаграмму в UI.

### Английский текст

Silero TTS **не умеет читать английский**. Весь английский текст должен быть транслитерирован в кириллицу до передачи в TTS-движок:

- Во всех местах, где может появиться английский (ссылки, код, заголовки и т.д.), обязателен **fallback для транслитерации**.
- Если обработка текста оставляет английские слова, они должны оставаться доступными для дальнейшей нормализации английским нормализатором.

### Реализация pipeline

Pipeline реализован в `src-tauri/src/pipeline/` (Rust). Корректность проверяется golden-фикстурами в `src-tauri/tests/fixtures/pipeline/`.

## Тесты

```bash
# Rust (все тесты)
nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml"

# Rust golden-тесты pipeline
nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml --test golden"

# Python subprocess
nix-shell --run "cd ttsd && uv run python -m pytest"

# TypeScript
nix-shell --run "pnpm typecheck"
```
