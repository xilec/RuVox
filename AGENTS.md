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
| [RewriteNotes.md](RewriteNotes.md) | Архитектурные решения, выбор стека, обоснования |
| [RewriteTaskPlan.md](RewriteTaskPlan.md) | Детальный план задач, граф зависимостей, глобальные правила |
| [task_history.md](task_history.md) | Живой журнал исполнения задач агентами |
| [docs/ipc-contract.md](docs/ipc-contract.md) | IPC-контракт: Tauri-команды, события, протокол ttsd (создаётся задачей F3) |
| [docs/storage-schema.md](docs/storage-schema.md) | Схема хранилища: history.json, timestamps, config (создаётся задачей F4) |
| `legacy/` | Замороженная PyQt6-реализация — референс для копирования логики. Удаляется после достижения фичи-паритета |

## Быстрый старт

> **Все команды запускаются через `nix-shell --run "..."`** — `cargo`, `pnpm`, `uv` и прочие инструменты доступны только внутри nix-shell (определён в `shell.nix` в корне).

```bash
nix-shell --run "pnpm install"                                              # фронт-зависимости (после F1+F2)
nix-shell --run "pnpm tauri dev"                                            # запуск приложения
nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml"          # Rust-тесты
nix-shell --run "pnpm typecheck"                                            # проверка типов TS
nix-shell --run "pnpm lint"                                                 # линтинг фронта
nix-shell --run "cd ttsd && uv run python -m pytest"                        # тесты Python-subprocess
```

Для legacy-кода используй отдельный shell:
```bash
nix-shell legacy/shell.nix --run "cd legacy && uv run python -m pytest"    # legacy-тесты
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
├── legacy/           # PyQt6-реализация (заморожена, только для чтения)
│   ├── src/
│   ├── tests/
│   ├── scripts/
│   ├── shell.nix
│   └── pyproject.toml
├── scripts/          # Утилитарные скрипты (generate_golden.py и др.)
├── shell.nix         # Nix-окружение для ruvox2 (Rust + Node + Python)
├── RewriteNotes.md
├── RewriteTaskPlan.md
└── task_history.md
```

## Правила разработки

### Общие

- Код (идентификаторы, комментарии) — на английском. Пользовательские строки (UI, уведомления, логи) — на русском.
- Никаких emoji в коде и коммит-сообщениях.
- Формат коммитов: `<type>(<module>): <short desc>`, где `type` ∈ `{feat, fix, chore, refactor, docs, test, build}`.
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

- Python 3.12, `uv`-managed. Не смешивать с legacy pyproject.
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

Pipeline реализован в `src-tauri/src/pipeline/` (Rust). Логика портирована из `legacy/src/ruvox/tts_pipeline/`. Корректность проверяется golden-тестами против legacy Python-версии (`src-tauri/tests/fixtures/pipeline/`).

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

# Legacy (только как референс)
nix-shell legacy/shell.nix --run "cd legacy && uv run python -m pytest --collect-only -q"
```
