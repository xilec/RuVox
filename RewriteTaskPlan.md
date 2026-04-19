# RuVox 2.0 — Детальный план задач

Документ дополняет [`RewriteNotes.md`](RewriteNotes.md): декомпозиция работы на задачи, граф зависимостей, критерии приёмки.

Этот файл читают **все** агенты-исполнители как базу знаний о проекте. Решения, принятые в `RewriteNotes.md`, считаются обязательными.

## Легенда

- **ID:** уникальный ID задачи (`F1`, `R3`, `U5` …).
- **Blocked by:** задачи, которые должны завершиться до начала этой.
- **Module:** что меняет задача (определяет post-merge чеки).
- **Branch:** имя ветки `task/<slug>` для feature-branch агента.

## Глобальные правила для всех агентов

Перед любой работой агент обязан прочитать:
1. `AGENTS.md` и `CLAUDE.md` в корне репозитория.
2. `RewriteNotes.md` — полностью.
3. Этот файл (`RewriteTaskPlan.md`) — свою задачу + зависимости.

### Стек и версии (жёстко)

- **Rust edition 2021** (или свежее, если `tauri-plugin-mpv` требует).
- **Tauri 2.x** (v2, не v1).
- **React 18.x** + **TypeScript 5.x (strict mode)**.
- **Mantine 8.x**. Стилизация: CSS Modules и prop `classNames`. **Запрещено:** `sx`, `createStyles`, `emotion`, любое легаси из Mantine 6.
- **Форма:** `@mantine/form` (не react-hook-form, не Formik).
- **Нотификации:** `@mantine/notifications`.
- **Hooks:** `@mantine/hooks`.
- **Роутинг:** по возможности не используется (одно окно с диалогами). Если очень нужно — `@tanstack/react-router`.
- **Сборщик фронта:** Vite.
- **Package manager фронта:** `pnpm`.
- **Python в ttsd:** 3.12, `uv` для окружения, `pyproject.toml` отдельный от legacy.
- **Regex в Rust pipeline:** крейт `regex` (DFA). Для словарных lookup'ов — `aho-corasick`.

### Стилистические правила

- Комментарии писать только если WHY неочевиден (скрытый инвариант, обход известного бага). Не комментировать WHAT.
- Никаких emoji в коде/коммитах, кроме явного пожелания пользователя.
- Без backwards-compatibility shims, без «on-your-way» рефакторингов.
- В Rust: `tracing` для логов, `thiserror` для ошибок доменного уровня, `anyhow::Result` — только на границах.
- В React: функциональные компоненты, hooks. Никаких `React.FC`.
- В TS: `strict: true`, без `any` без крайней необходимости.
- Коммиты на русском допустимы, но код, идентификаторы, английские комментарии — по-английски.
- `cargo fmt` и `prettier` обязательны.

### Коммиты и ветки

- Агент работает в своей ветке `task/<slug>`, форкнутой от `ruvox2`.
- Каждый агент может сделать 1+ коммитов по мере работы; финальный коммит должен оставить репу в билдабельном состоянии (или явно документировать незавершённое в description задачи).
- Сообщение коммита формата: `<type>(<module>): <short desc>` — `type` ∈ `{feat, fix, chore, refactor, docs, test, build}`.
- **Запрещено:** «Co-Authored-By: Claude …» и любое упоминание Claude в коммит-сообщении.
- `git push` — не делать, только локальные коммиты.

### Что делать при блокере

Если задача не может быть выполнена (отсутствует предусловие, неочевидная неоднозначность, обнаружен конфликт с решением в RewriteNotes.md):
1. Оставить в своей ветке файл `BLOCKED.md` с описанием проблемы и предложениями.
2. Сделать коммит `chore(<module>): block on <reason>`.
3. Завершить работу.

Координатор разберётся.

---

## Задачи

### Фаза 1 — Foundation

#### F1. Nix flake для сборочного окружения
- **Module:** nix
- **Branch:** `task/f1-nix-flake`
- **Blocked by:** —
- **Deliverable:** `shell.nix` (или `flake.nix`) в корне, предоставляющий:
  - Rust toolchain (`rustc`, `cargo`, `rustfmt`, `clippy`) последней стабильной версии.
  - Node.js + `pnpm`.
  - Python 3.12 + `uv`.
  - Системные зависимости для Tauri на Linux: `webkitgtk-4.1`, `libappindicator-gtk3`, `librsvg`, `gtk3`, `libsoup`, `openssl`, `pkg-config`.
  - Системные зависимости для `tauri-plugin-mpv`: `mpv`, `libmpv`.
  - `cargo-tauri` CLI.
- **Acceptance:** `nix-shell --run "cargo --version && pnpm --version && uv --version && tauri --version"` завершается без ошибок. Nix-shell реентерабелен из текущего состояния репо.
- **Ссылка на legacy:** `legacy/shell.nix` — там был только Python. Сохранить паттерны (уважение к `uv`), расширить остальным.

#### F2. Tauri 2 init с React + TypeScript + Mantine
- **Module:** frontend + src-tauri
- **Branch:** `task/f2-tauri-init`
- **Blocked by:** F1
- **Deliverable:**
  - `src-tauri/` — Rust-крейт инициализирован через `cargo tauri init` или руками. `Cargo.toml` с Tauri 2, `tauri.conf.json` с окном 900×600, title "RuVox", identifier `com.ruvox.app`.
  - `src/` — React + TypeScript скелет, Vite конфиг, `package.json` с pnpm-зависимостями.
  - `src/App.tsx` — `MantineProvider` с `defaultColorScheme="auto"`, пустая `AppShell` с заголовком "RuVox 2".
  - `src/main.tsx` — `createRoot`, `import '@mantine/core/styles.css'`.
  - `pnpm run tauri dev` запускается и показывает пустое окно без ошибок.
  - `pnpm typecheck` (скрипт в `package.json`) проходит.
- **Acceptance:**
  - `nix-shell --run "cd src && pnpm install"` — ок.
  - `nix-shell --run "cd src-tauri && cargo build"` — ок.
  - Окно открывается и рендерит заголовок.
- **Ссылка:** [tauri.app/start/create-project](https://tauri.app/start/create-project).

#### F3. IPC-контракт (документация)
- **Module:** docs
- **Branch:** `task/f3-ipc-contract`
- **Blocked by:** —
- **Deliverable:** `docs/ipc-contract.md` — полный перечень:
  - **Tauri commands** (frontend → backend): сигнатуры с именами, типами аргументов, типами возврата, возможными ошибками.
  - **Tauri events** (backend → frontend): имена, payload-типы.
  - **TTS subprocess protocol** (backend → ttsd): JSON-схема запросов и ответов, коды ошибок, warmup-команда.
- **Набор команд (минимум):**
  - `add_clipboard_entry(play_when_ready: bool) -> EntryId`
  - `get_entries() -> Vec<TextEntry>`
  - `get_entry(id: EntryId) -> Option<TextEntry>`
  - `delete_entry(id: EntryId) -> ()`
  - `delete_audio(id: EntryId) -> ()`
  - `play_entry(id: EntryId) -> ()`
  - `pause_playback() -> ()`
  - `resume_playback() -> ()`
  - `seek_to(position_sec: f64) -> ()`
  - `set_speed(speed: f32) -> ()`
  - `set_volume(volume: f32) -> ()`
  - `get_config() -> UIConfig`
  - `update_config(patch: UIConfigPatch) -> ()`
  - `get_timestamps(id: EntryId) -> Vec<WordTimestamp>`
- **Набор событий (минимум):**
  - `entry_updated(entry: TextEntry)`
  - `playback_position(position_sec: f64, entry_id: EntryId)`
  - `playback_started(entry_id: EntryId)`
  - `playback_stopped()`
  - `model_loading()`
  - `model_loaded()`
  - `model_error(message: String)`
  - `tts_error(entry_id: EntryId, message: String)`
- **Subprocess protocol:** см. `RewriteNotes.md` раздел «Protocol». Расширить: ошибочные коды (`error: "model_not_loaded" | "synthesis_failed" | "bad_input" | "internal"`).
- **Acceptance:** документ самодостаточен. Не обязан содержать Rust-код; типы описаны TypeScript-like синтаксисом + примерами JSON.

#### F4. Storage-схема (документация + Rust-типы)
- **Module:** src-tauri
- **Branch:** `task/f4-storage-schema`
- **Blocked by:** F2
- **Deliverable:**
  - `docs/storage-schema.md` — описание схем `history.json`, `{uuid}.wav`, `{uuid}.timestamps.json`, `config.json` с полной обратной совместимостью с legacy-версией.
  - `src-tauri/src/storage/schema.rs` — Rust-типы `TextEntry`, `EntryStatus`, `UIConfig`, `WordTimestamp`, `Timestamps`, `HistoryFile` с `serde::{Serialize, Deserialize}`. Имена полей и значения enum через `#[serde(rename_all = "snake_case")]` или явные `#[serde(rename)]` чтобы совпадать с JSON от legacy.
  - Unit-test в `src-tauri/src/storage/schema.rs`: десериализует пример legacy `history.json` и сериализует обратно, JSON совпадает по структуре (стабильно по порядку).
- **Ссылка:** legacy/src/ruvox/ui/models/entry.py, legacy/src/ruvox/ui/models/config.py, legacy/src/ruvox/ui/services/storage.py — источник истины для схемы.
- **Acceptance:** `cargo test --package ruvox-tauri` проходит (или пакет с соответствующим именем).

#### F5. Golden-test фикстуры из legacy Python pipeline
- **Module:** tests (pipeline fixtures)
- **Branch:** `task/f5-golden-fixtures`
- **Blocked by:** —
- **Deliverable:**
  - `src-tauri/tests/fixtures/pipeline/` — директория с парами `<case>.input.txt` и `<case>.expected.txt` + `<case>.char_map.json`.
  - Минимум 30 кейсов, покрывающих каждый нормалайзер: numbers, english, abbreviations, code (camel/snake/kebab), urls/paths/emails/ips, sizes, versions, ranges, percentages, operators, symbols, mermaid-блоки, inline-code, markdown-headers.
  - Скрипт `scripts/generate_golden.py` (новый, в корне `ruvox2`), запускающий legacy pipeline из `legacy/src/ruvox/tts_pipeline/` и записывающий фикстуры.
  - README `src-tauri/tests/fixtures/pipeline/README.md` — как добавить новый кейс, как перегенерировать.
- **Acceptance:**
  - `nix-shell --run "uv run --project legacy python scripts/generate_golden.py"` создаёт фикстуры без ошибок.
  - Каждая фикстура читаема и непуста.

#### F6. ttsd Python-subprocess (скелет + протокол)
- **Module:** ttsd
- **Branch:** `task/f6-ttsd-skeleton`
- **Blocked by:** F1
- **Deliverable:**
  - `ttsd/pyproject.toml` — с зависимостями `torch`, `numpy`, `scipy`, `omegaconf`, `torchaudio`. Name `ruvox-ttsd`.
  - `ttsd/ttsd/__init__.py`, `ttsd/ttsd/__main__.py` — входная точка: `python -m ttsd`.
  - `ttsd/ttsd/protocol.py` — определения request/response типов (pydantic v2 или typed dict).
  - `ttsd/ttsd/main.py` — главный цикл: read-line-from-stdin → dispatch → write-line-to-stdout.
  - Команды: `warmup`, `synthesize`, `shutdown`. На `warmup` — пока stub (печатает "warmup ok"), реальный load — в F7.
  - Логи через `logging` → stderr (не stdout!).
- **Acceptance:**
  - `nix-shell --run "cd ttsd && uv sync"` — ок.
  - `nix-shell --run "cd ttsd && echo '{\"cmd\":\"warmup\"}' | uv run python -m ttsd"` возвращает `{"ok": true}`.

#### F7. Silero wrapper + timestamps в ttsd
- **Module:** ttsd
- **Branch:** `task/f7-silero-wrapper`
- **Blocked by:** F6
- **Deliverable:**
  - `ttsd/ttsd/silero.py` — класс `SileroEngine`:
    - `load()` — `torch.hub.load('snakers4/silero-models', ...)`, скачивает модель при первом запуске.
    - `synthesize(text: str, speaker: str, sample_rate: int, out_wav: Path) -> SynthesizeResult` — делит текст на чанки (port из legacy `TTSRunnable._split_into_chunks`), синтезирует, конкатенирует, сохраняет WAV через `scipy.io.wavfile`.
    - `sanitize_for_silero(text)` — port из legacy (убирает newlines и т.д.).
  - `ttsd/ttsd/timestamps.py` — port из legacy `_estimate_timestamps_chunked` + `_extract_words_with_positions`. Принимает char_mapping из Rust (см. F8), но поскольку F8 может идти позже — на первом этапе работает без char_mapping (позиции в нормализованном тексте).
  - Протокол `synthesize` из F3 заполняется реально.
  - `ttsd/tests/test_silero.py` — smoke-тест: маленький текст, проверка что WAV создан и непуст.
- **Acceptance:** smoke-test проходит (под nix-shell).
- **Ссылка:** legacy/src/ruvox/ui/services/tts_worker.py.

#### F8. CLAUDE.md / AGENTS.md обновление для ruvox2
- **Module:** docs
- **Branch:** `task/f8-claude-md`
- **Blocked by:** —
- **Deliverable:**
  - `CLAUDE.md` — обновить: убрать упоминание PyQt как current stack, добавить «см. RewriteNotes.md для архитектуры ruvox2», сохранить ссылки на AGENTS.md.
  - `AGENTS.md` — обновить: раздел «Обзор проекта» переработать под Tauri + React + Rust, оставить ссылки на legacy/ как референс. Сохранить все правила относительно языка ответов, NikShell, Mantine 8-specific конвенций.
  - Не удалять `docs/` существующую — просто обновить `docs/index.md` с новой структурой.
- **Acceptance:** агент, прочитав CLAUDE.md + AGENTS.md, понимает что делать в ruvox2-контексте без обращения к legacy-инструкциям.

---

### Фаза 2 — Rust pipeline

#### R1. TrackedText + CharMapping в Rust
- **Module:** src-tauri/pipeline
- **Branch:** `task/r1-tracked-text`
- **Blocked by:** F2, F4
- **Deliverable:**
  - `src-tauri/src/pipeline/tracked_text.rs` — порт `TrackedText` и `CharMapping` из `legacy/src/ruvox/tts_pipeline/tracked_text.py`.
  - API: `TrackedText::new(text)`, `.replace(pattern, replacement)`, `.sub(regex, replacer)`, `.build_mapping() -> CharMapping`.
  - `CharMapping::get_original_range(trans_start, trans_end) -> (usize, usize)`.
  - Unit-тесты для основных операций (replace, sub, вложенные замены, build_mapping).
- **Acceptance:** `cargo test --package <pipeline>` проходит, все legacy-тесты `test_tracked_text.py` имеют Rust-эквивалент.

#### R2–R8. Нормалайзеры
Каждая задача — один нормалайзер.
- **Blocked by:** R1 (все).
- **Общий паттерн:**
  - Создать `src-tauri/src/pipeline/normalizers/<name>.rs`.
  - Порт логики из `legacy/src/ruvox/tts_pipeline/normalizers/<name>.py`.
  - Использовать `regex` и `aho-corasick` где уместно.
  - Unit-тесты.

| ID | Normalizer | Branch | Legacy-файл |
|----|-----------|--------|-------------|
| R2 | numbers | `task/r2-numbers-normalizer` | normalizers/numbers.py |
| R3 | english | `task/r3-english-normalizer` | normalizers/english.py |
| R4 | abbreviations | `task/r4-abbreviations-normalizer` | normalizers/abbreviations.py |
| R5 | code | `task/r5-code-normalizer` | normalizers/code.py |
| R6 | urls | `task/r6-urls-normalizer` | normalizers/urls.py |
| R7 | symbols | `task/r7-symbols-normalizer` | normalizers/symbols.py + constants.py |
| R8 | code_blocks | `task/r8-code-blocks` | часть pipeline.py + `CodeBlockHandler` логика |

#### R9. Pipeline integration
- **Module:** src-tauri/pipeline
- **Branch:** `task/r9-pipeline-integration`
- **Blocked by:** R1, R2, R3, R4, R5, R6, R7, R8
- **Deliverable:**
  - `src-tauri/src/pipeline/mod.rs` — `TTSPipeline::process(&str) -> String` и `::process_with_char_mapping(&str) -> (String, CharMapping)`.
  - Порядок нормалайзеров — как в `legacy/src/ruvox/tts_pipeline/pipeline.py::process_with_char_mapping`.
  - Интеграционные тесты, покрывающие полный pipeline на нескольких golden-фикстурах из F5.
- **Acceptance:**
  - `cargo test` проходит.
  - Все golden-фикстуры дают ровно такой же `normalized` output как Python-версия. Несовпадения допустимы только если осознанно задокументированы (по пробелам/etc, объяснение в PR-description задачи).

#### R10. Golden-test harness
- **Module:** src-tauri/tests
- **Branch:** `task/r10-golden-harness`
- **Blocked by:** R9, F5
- **Deliverable:**
  - `src-tauri/tests/golden.rs` (интеграционный тест) — читает фикстуры из F5, прогоняет через Rust pipeline, сравнивает.
  - При несовпадениях — diff-вывод.
- **Acceptance:** `cargo test --test golden` — 100% фикстур проходит.

---

### Фаза 3 — Core UI + Backend services

#### B1. Storage service в Rust
- **Module:** src-tauri/storage
- **Branch:** `task/b1-storage-service`
- **Blocked by:** F4
- **Deliverable:**
  - `src-tauri/src/storage/mod.rs` — `StorageService` с методами: `load`, `add_entry`, `get_entry`, `update_entry`, `delete_entry`, `delete_audio`, `get_all_entries`, `save_audio`, `save_timestamps`, `load_timestamps`, `get_audio_path`.
  - Путь кэша: `dirs::cache_dir() / "ruvox"` (совместимо с legacy: `~/.cache/ruvox/`).
  - Валидация статусов при загрузке (port из `_validate_entry_status`).
  - Unit-тесты, в т.ч. совместимость с legacy `history.json` (читать тестовый файл из legacy-структуры).
- **Acceptance:** `cargo test` + ручная проверка: создать entry, закрыть приложение, перезапустить, entry сохранён.

#### B2. TTS subprocess manager
- **Module:** src-tauri/tts
- **Branch:** `task/b2-tts-manager`
- **Blocked by:** F3, F7
- **Deliverable:**
  - `src-tauri/src/tts/subprocess.rs` — `TtsSubprocess`:
    - `spawn(config) -> Self` — запускает `uv run python -m ttsd` через `tokio::process::Command`.
    - Async-очередь запросов (`mpsc::channel`).
    - `synthesize(text, speaker, sample_rate) -> Result<SynthesisOutput>`.
    - `warmup() -> ()` — вызывается при старте.
    - `shutdown()` — штатное завершение.
    - Автоперезапуск subprocess при крэше (tracing::warn + respawn).
    - Логи stderr из ttsd → `tracing::info!` с prefix `ttsd:`.
  - Unit-тесты через mock (через подставляемый executable или feature-flag).
- **Acceptance:** `cargo test`, ручная проверка — subprocess стартует, warmup отрабатывает, synthesize маленького текста возвращает WAV.

#### B3. Player через tauri-plugin-mpv
- **Module:** src-tauri/player
- **Branch:** `task/b3-player`
- **Blocked by:** F2
- **Deliverable:**
  - Установить и сконфигурировать `tauri-plugin-mpv`.
  - `src-tauri/src/player/mod.rs` — обёртка над плагином:
    - `load(path)`, `play`, `pause`, `resume`, `stop`, `seek(sec)`, `set_speed(f32)`, `set_volume(f32)`.
    - Периодическая публикация позиции (`playback_position` event) — раз в 100 ms, пока играет.
    - scaletempo2 включён по умолчанию.
  - Проверка: проигрывает WAV с позитивным sample rate, работает seek, скорость 0.5x–2.0x не ломает качество.
- **Acceptance:** Ручной smoke-test через `tauri dev`.

#### B4. Tauri commands (IPC-handlers)
- **Module:** src-tauri/commands
- **Branch:** `task/b4-tauri-commands`
- **Blocked by:** B1, B2, B3, R9, F3
- **Deliverable:**
  - `src-tauri/src/commands/mod.rs` — все команды из F3, обёрнуты в `#[tauri::command]`.
  - Emit-логика для всех событий из F3 через `AppHandle`.
  - Обработка ошибок: типизированные `enum Error`, `#[serde(tag = "type")]`, фронт получает читаемые коды.
  - State-management через `tauri::State<AppState>`, где `AppState { storage, tts, player, pipeline }`.
- **Acceptance:** `cargo build` ок, команды вызываемы из фронта (smoke через `invoke()` из dev-tools).

#### B5. System tray
- **Module:** src-tauri/tray
- **Branch:** `task/b5-tray`
- **Blocked by:** F2
- **Deliverable:**
  - Иконка в трее (переиспользовать `legacy/src/ruvox/ui/resources/tray_icon.svg` — скопировать в `src-tauri/icons/`).
  - Меню: Show/Hide Window, Read Now, Read Later, Settings, Quit.
  - `Read Now`/`Read Later` читают clipboard и вызывают `add_clipboard_entry`.
  - ДвойнойКликом → показать окно.
- **Acceptance:** tray-иконка появляется при запуске, меню работает.

#### U1. App shell + MantineProvider + theme switcher
- **Module:** frontend
- **Branch:** `task/u1-app-shell`
- **Blocked by:** F2
- **Deliverable:**
  - `src/App.tsx` — `MantineProvider` (`defaultColorScheme="auto"`), `Notifications`, `ModalsProvider`.
  - `src/components/AppShell.tsx` — Mantine `AppShell` с `Header`. В header: Заголовок "RuVox" слева, `SegmentedControl` темы справа (Light/Dark/Auto).
  - Hook `useThemeSwitcher()` — возвращает `{scheme, setScheme}`, пишет в `localStorage` (через стандартный `colorSchemeManager` Mantine).
  - Layout готов принять: плеер сверху, left-panel (queue), right-panel (viewer). Заготовки пустые `<div>` с border-ом и подписью.
  - `src/lib/tauri.ts` — типизированные обёртки над `invoke()`/`listen()` для команд и событий из F3.
- **Acceptance:** `pnpm typecheck` + `pnpm build` ок. Ручной запуск: `pnpm tauri dev` — окно с header'ом и работающим переключателем темы.

#### U2. Queue list component
- **Module:** frontend
- **Branch:** `task/u2-queue-list`
- **Blocked by:** U1, B4
- **Deliverable:**
  - `src/components/QueueList.tsx` — список `TextEntry` из `get_entries()`.
  - Каждый item: preview (первые 60 символов), статус-бейдж, длительность, кнопки (Play/Delete).
  - Клик на item → выбрать в store (Zustand или Context) → фильтрует text_viewer.
  - Listen на event `entry_updated` → обновляет список.
  - Пустое состояние: «Скопируйте текст и нажмите Read Now».
- **Acceptance:** smoke — добавить запись через tray, список обновляется.

#### U3. Player component
- **Module:** frontend
- **Branch:** `task/u3-player`
- **Blocked by:** U1, B4
- **Deliverable:**
  - `src/components/Player.tsx` — Mantine `Group`: Play/Pause, Progress (`Slider`), Time (MM:SS / MM:SS), Speed-control (click-wheel: 0.5x–2.0x, шаг 0.1), Volume-control, Prev/Next-in-queue.
  - Listen на `playback_position` → обновляет progress slider.
  - Seek через перетаскивание slider'а.
  - Keyboard: Space → Play/Pause, Left/Right → seek ±5s.
- **Acceptance:** smoke — воспроизведение + seek работают.

#### U4. Text viewer (plain + markdown)
- **Module:** frontend
- **Branch:** `task/u4-text-viewer`
- **Blocked by:** U1
- **Deliverable:**
  - `src/components/TextViewer.tsx` — рендерит `selectedEntry.original_text`.
  - Режимы: plain и markdown. Переключатель в углу viewer'а (`SegmentedControl`).
  - Markdown рендерится через `markdown-it` с плагинами (`highlight.js` для кода). Позиции токенов сохраняются в DOM как `data-orig-start` / `data-orig-end` атрибутах на `<span>`.
  - Plain — просто `<pre>` с original_text.
- **Acceptance:** переключение между plain/markdown мгновенное, текст с кодом и заголовками рендерится.

#### U5. Word highlighting
- **Module:** frontend
- **Branch:** `task/u5-highlighting`
- **Blocked by:** U4, B3, B4
- **Deliverable:**
  - При старте воспроизведения — загрузить timestamps из `get_timestamps(entry_id)`.
  - Listen на `playback_position`. Бинарный поиск по timestamps → `WordTimestamp`.
  - Найти span в DOM по `data-orig-start`/`data-orig-end`, добавить CSS-класс `.word-highlight`.
  - Scroll в viewport, если слово вышло за пределы.
  - Тёмная/светлая тема — подхватывается через CSS vars.
- **Acceptance:** при воспроизведении слова подсвечиваются синхронно.

#### U6. Clipboard input flow
- **Module:** frontend
- **Branch:** `task/u6-clipboard-input`
- **Blocked by:** U1, B4
- **Deliverable:**
  - Кнопки «Read Now» и «Read Later» в header.
  - При клике: вызов `add_clipboard_entry(play_when_ready)`.
  - Уведомление через `notifications.show` при добавлении.
- **Acceptance:** клик → запись появляется в очереди.

---

### Фаза 4 — Enrichment

#### U7. Mermaid rendering
- **Module:** frontend
- **Branch:** `task/u7-mermaid`
- **Blocked by:** U4
- **Deliverable:**
  - В markdown-режиме блоки ```mermaid ...``` заменяются на `<div class="mermaid">`.
  - После рендера Markdown — вызов `mermaid.run()` (библиотека `mermaid` через npm).
  - Тема mermaid синхронизируется с Mantine-темой.
  - При клике на диаграмму — модалка с zoomable-версией.
- **Acceptance:** mermaid-блок из тестового Markdown рендерится без подгрузки Chromium.

#### U8. HTML format support
- **Module:** frontend + src-tauri
- **Branch:** `task/u8-html-format`
- **Blocked by:** U4
- **Deliverable:**
  - Детекция HTML в clipboard (`text/html` MIME).
  - Новый формат `html` в `TextViewer`.
  - Sanitize через DOMPurify, подсветка кода через shiki/highlight.js.
  - Pipeline: извлечение текста из HTML для TTS (библиотека в Rust или через `scraper`), сохранение mapping.
  - Исключение `<nav>`, `<footer>`, `<aside>` из озвучки.
- **Acceptance:** копирование статьи из браузера → читается как HTML, озвучивается только основной контент.

#### U9. Settings dialog
- **Module:** frontend
- **Branch:** `task/u9-settings`
- **Blocked by:** U1, B4
- **Deliverable:**
  - `src/dialogs/Settings.tsx` — Mantine `Modal` с формой (`@mantine/form`).
  - Поля: speaker (select), sample_rate (select), notify_on_ready (switch), notify_on_error (switch), max_cache_size_mb (number), auto_cleanup_days (number), hotkeys (placeholder пока).
  - Сохранение через `update_config`.
- **Acceptance:** изменения применяются и персистятся.

#### U10. Notifications integration
- **Module:** frontend
- **Branch:** `task/u10-notifications`
- **Blocked by:** U1
- **Deliverable:**
  - Listen на `tts_error`, `model_error` → `notifications.show` (цвет red).
  - Listen на `model_loading`/`model_loaded` → нотификация progress.
- **Acceptance:** ошибки и статусы видны пользователю.

---

### Фаза 5 — Advanced features

#### U11. Preview dialog (FF 1.1)
- **Module:** frontend + src-tauri
- **Branch:** `task/u11-preview`
- **Blocked by:** R9, U1, B4
- **Deliverable:** Диалог между `add_clipboard_entry` и TTS. Показ original + normalized side-by-side, кнопки Synthesize/Edit/Cancel, чекбокс «не показывать для коротких текстов» (порог в config).
- **Acceptance:** соответствует описанию в `FutureFeatures.md` раздел 1.1.

#### U12. Edit mode (FF 1.2)
- **Module:** frontend + src-tauri
- **Branch:** `task/u12-edit-mode`
- **Blocked by:** U4, B4
- **Deliverable:** Toggle readonly↔edit в TextViewer, сохранение edited-версии в `TextEntry.edited_text`, синтез использует edited или original.
- **Acceptance:** FF 1.2 работает.

---

### Фаза 6 — Polish

#### P1. Tauri bundling через Nix flake
- **Module:** nix + src-tauri
- **Branch:** `task/p1-bundling`
- **Blocked by:** F1 — и практически все UI/backend задачи завершены.
- **Deliverable:** Nix derivation для сборки финального бинаря с bundled ttsd (как sidecar или как deps-tree).
- **Acceptance:** `nix build .#ruvox` даёт запускаемый бинарь.

#### P2. Удаление legacy
- **Module:** repo
- **Branch:** (делается координатором напрямую на ruvox2, не через агента)
- **Blocked by:** подтверждение пользователя о фичи-паритете.
- **Deliverable:** `git rm -r legacy/` + обновление docs.

#### P3. Финальные тесты + документация
- **Module:** docs + tests
- **Branch:** `task/p3-final-docs`
- **Blocked by:** все предыдущие.
- **Deliverable:** README, onboarding-doc, changelog перехода.

---

## Граф зависимостей (визуализация)

```
F1 ── F2 ── R1 ──┬── R2 ──┐
                 ├── R3 ──┤
                 ├── R4 ──┤
                 ├── R5 ──┤
                 ├── R6 ──┼── R9 ── R10
                 ├── R7 ──┤
                 └── R8 ──┘

F1 ── F6 ── F7 ─┐
                │
F3 ─────────────┤
                ├── B2 ──┐
F4 ── F2 ── B1 ─┤        │
            │            │
            └── B3 ──────┤
                         ├── B4 ──┐
            R9 ──────────┘        │
                                  ├── U2
F2 ── U1 ────────────────────────┤
     │                            ├── U3
     ├── B5                       │
     │                            ├── U5 (+ U4)
     ├── U4 ───────┬── U7         │
     │             │              │
     │             └── U8 (+B4)   │
     │                            ├── U6
     │                            ├── U9
     │                            └── U10
     │
     └── U11 (+R9, +B4)
         U12 (+U4, +B4)

Все UI/B/R/P* задачи → P1 (bundling) → P2/P3
```

## Готовность к параллельному запуску

После первого коммита (движение в `legacy/`) и финализации этого файла — координатор может одновременно запустить:

- **Wave 1 (3 агента):** F1 (Nix flake), F3 (IPC doc), F5 (golden fixtures), F8 (CLAUDE.md update). Они независимы.
- **Wave 2 (ждёт F1):** F2 (Tauri init), F6 (ttsd skeleton).
- **Wave 3 (ждёт F2 + F6):** F4, F7.
- **Wave 4 (ждёт F4, F2):** R1, B1, B3, B5, U1.
- И так далее по графу.

Координатор держит до 7 агентов в работе одновременно, запускает новых как только зависимости разблокированы.