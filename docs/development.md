# Разработка

Руководство по настройке окружения и работе над RuVox.

## Требования

- **Linux** (X11 или Wayland). macOS/Windows — не поддерживаются.
- **Nix** (рекомендуется) — даёт воспроизводимое окружение целиком: Rust toolchain, Node, pnpm, Python 3.12, uv, libmpv, webkit2gtk и системные библиотеки Tauri.
- **Без Nix** — придётся вручную установить Rust stable + Node 20 + Python 3.12 + системные deps (см. `buildInputs` в `shell.nix`: `webkitgtk_4_1`, `libsoup_3`, `gtk3`, `libmpv`, `pipewire`/`pulseaudio`, `libappindicator-gtk3`, `librsvg`, `pkg-config`).

## Окружение

```bash
# Через flake (рекомендуется)
nix develop
pnpm install
pnpm tauri dev

# Или через классический shell.nix
nix-shell --run "pnpm install"
nix-shell --run "pnpm tauri dev"
```

> **Важно:** все команды (`cargo`, `pnpm`, `uv`, `tauri`, `ruff`, `pytest`) доступны только внутри `nix develop` / `nix-shell`. Не запускай команды из «уже открытой» nix-shell-сессии после правок `shell.nix` — `shellHook` (в т.ч. `XDG_DATA_DIRS`, `GIO_EXTRA_MODULES`, `WEBKIT_DISABLE_DMABUF_RENDERER`) выполняется только при входе в shell. Каждый `nix-shell --run "..."` форкает свежий subshell и получает актуальный env.

## Структура проекта

```
/
├── src/                    # React + TypeScript frontend (Vite + Mantine 8)
│   ├── components/         # AppShell, QueueList, Player, TextViewer, ThemeSwitcher, icons
│   ├── dialogs/            # PreviewDialog (FF 1.1), Settings
│   ├── lib/                # tauri.ts (typed wrappers), markdown, html, mermaid, wordHighlight, errors
│   └── stores/             # Zustand-store selectedEntry
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── pipeline/       # Нормализация: tracked_text, normalizers/, html_extractor, constants
│   │   ├── storage/        # JSON-история + аудиофайлы (схема в storage/schema.rs)
│   │   ├── tts/            # Менеджер ttsd-subprocess
│   │   ├── player/         # Обёртка tauri-plugin-mpv (ensure_mpv_alive, seek-suppress)
│   │   ├── commands/       # Tauri-команды (#[tauri::command])
│   │   ├── tray/           # Системный трей (close-to-tray, "Выход")
│   │   ├── state.rs        # AppState
│   │   └── lib.rs          # Точка входа Tauri::Builder
│   └── tests/
│       ├── fixtures/pipeline/  # Golden-фикстуры (37 кейсов × 3 файла)
│       └── golden.rs           # Интеграционный golden-тест
├── ttsd/                   # Python-subprocess (Silero TTS sidecar)
│   ├── pyproject.toml
│   └── ttsd/
│       ├── silero.py       # SileroEngine: load, synthesize
│       ├── timestamps.py   # Оценка временных меток слов
│       ├── protocol.py     # Типы request/response
│       └── main.py         # Главный цикл stdin→stdout JSON
├── docs/                   # Документация (этот каталог)
├── scripts/                # Утилиты (launch-prod, rebuild_prod)
├── shell.nix               # Nix-окружение (Rust + Node + Python + Tauri deps)
└── flake.nix               # Flake (для nix build .#ruvox и nix develop)
```

## Команды

### Запуск

```bash
nix-shell --run "pnpm tauri dev"            # dev-режим с hot-reload
nix build .#ruvox && ./result/bin/ruvox     # production-бинарь
```

### Тесты

```bash
nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml"           # все Rust-тесты
nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml --test golden"  # только golden-тесты
nix-shell --run "pnpm typecheck"                                            # TypeScript strict
nix-shell --run "cd ttsd && uv run python -m pytest"                        # Python-subprocess
```

### Сборка production

```bash
nix build .#ruvox
./result/bin/ruvox
```

`.#ruvox` собирает release-бинарь Tauri, оборачивает через `wrapProgram` (runtime `LD_LIBRARY_PATH` + `GIO_EXTRA_MODULES`), линкует `ttsd` (Silero-subprocess) и `mpv` в `PATH`.

> **Первый запуск `nix build`:** derivation `frontend` использует `pnpm.fetchDeps` с `lib.fakeHash` — Nix упадёт с hash mismatch и напишет реальный hash; его нужно подставить в `flake.nix` и повторить build. Это стандартная процедура pnpm2nix.

## Правила кода

### Общие

- Идентификаторы и комментарии — на английском. Пользовательские строки (UI, нотификации) — на русском.
- Никаких emoji в коде и коммит-сообщениях.
- Формат коммитов: `<type>(<module>): <short desc>`, `type ∈ {feat, fix, chore, refactor, docs, test, build}`.
- **Запрещено:** «Co-Authored-By: Claude …» и любое упоминание Claude в коммите.
- Комментарии — только если **WHY** неочевиден (скрытый инвариант, обход известного бага). Не комментировать **WHAT**.

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

- Стилизация через **CSS Modules** и prop `classNames`.
- **Запрещено:** `sx`, `createStyles`, `emotion`, любое легаси из Mantine 6/7.
- Формы: `@mantine/form` (не react-hook-form, не Formik).
- Нотификации: `@mantine/notifications`.
- Хуки: `@mantine/hooks`.
- Модалки: `@mantine/modals` (`modals.openConfirmModal` и т. п.).

### State

- Без Redux. Глобальное состояние — Zustand или React context. По умолчанию — props + `useState`.
- React Query не нужен — Tauri-invoke ложится в `useEffect` + `useState`.

### Routing

- Без router. Диалоги через `@mantine/modals` или non-modal floating windows (`react-rnd`, см. PreviewDialog).

### Python (ttsd)

- Python 3.12, `uv`-managed.
- Логи на stderr, JSON-запросы на stdin, JSON-ответы на stdout.
- `ruff check` и `pytest` должны быть зелёными.

## Отладка

### Логи

- **Tauri (Rust)** — через `tracing` на stderr процесса. В dev-режиме видны в терминале, где запущен `pnpm tauri dev`.
- **ttsd** — Python-логи на stderr → Rust проксирует в `tracing::info!("ttsd: ...")`.
- **Frontend** — DevTools webview (правый клик → Inspect Element в окне приложения).

### Webview DevTools

В debug-сборках Tauri webview включает DevTools. Для prod-сборки нужно явно разрешить в `tauri.conf.json` или собирать с feature `devtools`.

### Чтение `~/.cache/ruvox/`

Storage-кеш живёт в `~/.cache/ruvox/`:
- `history.json` — список `TextEntry`. Можно открыть руками.
- `audio/{uuid}.wav` — аудио. Открывается любым плеером.
- `audio/{uuid}.timestamps.json` — тайминги слов.
- `config.json` — `UIConfig`.

См. [Storage-схема](storage-schema.md) для деталей.

## Workflow

```bash
git checkout -b feat/short-description     # отдельная ветка
# ... правки ...
nix-shell --run "cargo test ..."           # прогон тестов
git commit -m "feat(<module>): <desc>"     # commit
git push -u origin feat/short-description  # push (по согласованию)
```

Новые фичи и фиксы идут обычным feature-branch-флоу.
