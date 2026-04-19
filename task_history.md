# RuVox 2.0 — Журнал исполнения задач

Живой журнал: кто/когда/что делал по задачам из `RewriteTaskPlan.md`.

Формат записи:

```
## [YYYY-MM-DD HH:MM] <TASK-ID> — <short title>
- status: in_progress | done | failed | reverted | blocked
- agent: <agent-id> (<model>)
- branch: task/<slug>
- started: YYYY-MM-DD HH:MM
- finished: YYYY-MM-DD HH:MM (or —)
- reviewer: <agent-id> (Opus)
- review_result: ok | needs_fix | critical
- notes: <краткое резюме, блокеры, follow-up задачи>
```

---

## [2026-04-19] Инициализация rewrite

- Создана ветка `ruvox2` от `main`.
- Первый коммит: `2517cba chore(rewrite): move PyQt implementation to legacy/, start ruvox2 branch`.
- Второй коммит (готовится): добавление `RewriteNotes.md` (расширенная версия), `RewriteTaskPlan.md`, `task_history.md`.
- Следующий шаг: запуск первой волны агентов (F1, F3, F5, F8).

---

## Wave 1: запуск foundation-агентов

### [2026-04-19] F1 — Nix flake
- status: in_progress
- agent: autopilot-sonnet (background)
- branch: task/f1-nix-flake
- started: 2026-04-19
- deps: none

### [2026-04-19] F3 — IPC contract doc
- status: in_progress
- agent: autopilot-sonnet (background)
- branch: task/f3-ipc-contract
- started: 2026-04-19
- deps: none

### [2026-04-19] F5 — Golden-test fixtures
- status: in_progress
- agent: autopilot-sonnet (background)
- branch: task/f5-golden-fixtures
- started: 2026-04-19
- deps: none

### [2026-04-19] F8 — Update AGENTS.md/CLAUDE.md
- status: in_progress
- agent: autopilot-sonnet (background)
- branch: task/f8-project-instructions
- started: 2026-04-19
- deps: none

---

## Ожидание завершения Wave 1

Ready-tasks после завершения:
- F1 done → разблокирует F2, F6.
- F2 done → разблокирует F4, B3, B5, U1, R1 (через F4).
- F3 done → разблокирует B2 (через F7).
- F5 done → разблокирует R10 (через R9).
- F8 done → standalone.

Первый kandidат следующей волны — F2 и F6 (зависят от F1).

---

## Wave 1 — завершение и результаты

### F3 — IPC contract doc
- status: **merged**
- branch: task/f3-ipc-contract
- worker_commit: `fab358a docs(ipc): IPC contract between frontend, backend, and ttsd subprocess`
- reviewer: autopilot (Opus), review_result: ok
- merge_sha: `c93d6c7 merge(f3): IPC contract documentation`
- notes: 905 строк в `docs/ipc-contract.md`. Три слоя: 18 Tauri-команд (14 обязательных + extra: `cancel_synthesis`, `stop_playback`, `clear_cache`, `get_cache_stats`), 11 событий (8 обязательных + `playback_paused`, `playback_finished`, `synthesis_progress`), ttsd JSON-протокол с `warmup`/`synthesize`/`shutdown` и 4 кодами ошибок. Добавлен `char_mapping` в запросе synthesize — для передачи маппинга из Rust-pipeline. Три example-exchange'а. `TextEntry.edited_text` предусмотрено для U12. `UIConfig.theme` переосмыслено как `light | dark | auto` вместо старых PyQt-тем.

### F8 — AGENTS.md/CLAUDE.md update
- status: **merged**
- branch: task/f8-project-instructions
- worker_commit: `7c60401 docs(rewrite): update AGENTS.md and CLAUDE.md for ruvox2 stack`
- reviewer: autopilot (Opus), review_result: ok
- merge_sha: `66ac764 merge(f8): update AGENTS.md and CLAUDE.md for ruvox2 stack`
- notes: 171 строка в новом AGENTS.md. Стек описан (Tauri 2 / React 18 + TS / Mantine 8 / Rust / ttsd). Правило «Всегда отвечай на русском» сохранено. Mantine 8 правила: CSS Modules + `classNames`, явный запрет `sx`/`createStyles`/emotion/Mantine 6-7 legacy. `@mantine/form` обязателен. Обновлён `docs/index.md`. `CLAUDE.md` — pointer на AGENTS.md + ссылки на RewriteNotes/RewriteTaskPlan.
- **false-positive security warning** от worker-агента о «commit without draft approval» — проигнорирован, коммиты без драфтов явно разрешены координатором.

### F5 — Golden-test fixtures
- status: **merged**
- branch: task/f5-golden-fixtures
- worker_commit: `4bd8d5a test(fixtures): golden test cases from legacy pipeline`
- reviewer: autopilot (Opus), review_result: ok
- merge_sha: `d3775c8 merge(f5): golden test cases from legacy pipeline`
- notes: 37 кейсов × 3 файла (`.input.txt`/`.expected.txt`/`.char_map.json`) + README = 112 файлов в `src-tauri/tests/fixtures/pipeline/`. Покрыты все категории: числа/размеры/версии/диапазоны/проценты, english/abbreviations, camelCase/PascalCase/snake_case/kebab-case, URL/email/IP/path, greek/math/arrow symbols, markdown-структуры, mermaid, mixed_paragraph. Ревьюер перегенерировал — diff пустой (байт-в-байт).
- workaround: worker-агент не мог запустить `nix-shell legacy/shell.nix` из-за sandbox-ограничения на Unix-domain-socket к nix-daemon. Обошёл через `PYTHONPATH=legacy/src python3.13 scripts/generate_golden.py` (python из nix-store + `num2words` из uv-cache). На хост-машине документированный путь `nix-shell legacy/shell.nix --run "..."` работает.
- schema check: `len(char_map) == len(transformed)` для всех 37 фикстур; содержимое `expected.txt` реально отражает pipeline output (прописные числа, транслит английского, camelCase→слова, mermaid-маркер).

### F1 — Nix flake
- status: **merged**
- branch: task/f1-nix-flake
- worker_commit: `cb4edb8 build(nix): shell.nix with Rust + Node + Python + Tauri deps`
- fix_commit: `12a90fb build(nix): add cargo-tauri from nixpkgs, remove incorrect workaround`
- reviewer: autopilot (Opus) × 2 (первичный = needs_fix, повторный = ok)
- merge_sha: `ee67324 merge(f1): shell.nix with Rust + Node + Python + Tauri deps (incl. cargo-tauri)`
- fix_history:
  - Первичный ревью → needs_fix: воркер ошибочно решил, что `cargo-tauri` отсутствует в nixpkgs (искал `tauri-cli`/`tauri`, пропустил верное имя). Ревьюер проверил `pkgs/by-name/ca/cargo-tauri/package.nix` в nixpkgs 25.11 — `cargo-tauri` 2.9.4 **есть**.
  - Fix-агент (autopilot-sonnet) за 1 минуту добавил `cargo-tauri` в buildInputs, удалил workaround-комментарии, echo shellHook'а про `cargo install tauri-cli` заменён на `echo "  tauri: $(cargo tauri --version)"`. Попутно убрал избыточный `openssl.dev` (остался через output'ы `pkgs.openssl`).
  - Повторный ревью → ok, merge.
- components ok:
  - Rust 1.91.1 + rustfmt + clippy + clippy-driver.
  - nodejs_20 + pnpm 10.25.0 + uv 0.9.16 + python312.
  - webkitgtk_4_1 2.50.3 + libsoup_3 + mpv-unwrapped 0.40.0 + pkg-config.
  - cargo-tauri 2.9.4 (без workaround).
- sandbox note: live `nix-shell --run ...` ни worker, ни reviewer выполнить не могли (sandbox блокирует Unix-socket к nix-daemon). Вся верификация — статическая. **Финальная live-проверка остаётся за пользователем на хост-машине.**

---

## Wave 2 — запуск (unblocked после F1)

### F2 — Tauri 2 init
- status: **awaiting review**
- branch: task/f2-tauri-init
- worker_commits:
  - `399e4e7 feat(scaffold): Tauri 2 init with React 18 + TypeScript + Mantine 8`
  - `5dbb93b fix(scaffold): fix tsconfig for typecheck and @types/node for vite.config`
- deps: F1 merged.
- next_unblocks: F4 (storage schema), B3 (player), B5 (tray), U1 (app shell).
- deliverable:
  - `src-tauri/` — Cargo.toml (tauri 2.10.3), tauri.conf.json (900×600, com.ruvox.app), main.rs + lib.rs, capabilities/default.json, build.rs.
  - `src/` — main.tsx + App.tsx (MantineProvider defaultColorScheme="auto", ModalsProvider, Notifications, AppShell с header «RuVox 2»).
  - `package.json` — React 18.3, Mantine 8.x (core/hooks/notifications/modals/form), Vite 6, @tauri-apps/api ^2, scripts: dev/build/typecheck/tauri:dev/tauri:build.
  - `tsconfig.json` (единый, не разделён на node.json — упрощённо), `vite.config.ts` (port 1420, strictPort), `index.html`, `.gitignore` (node_modules/, dist/, src-tauri/target/).
  - `Cargo.lock` создан и закоммичен.
- sandbox blockers:
  - `pnpm install` — не выполнен, `pnpm-lock.yaml` отсутствует. Ревьюер должен сгенерировать на хосте.
  - `cargo build` — попытка через `CARGO_HOME=$TMPDIR` упала на missing libsoup/cairo/gtk3 (ожидаемо вне nix-shell).
- minor deviations:
  - Один `tsconfig.json` вместо разделения на `tsconfig.node.json` — упрощено, strict mode сохранён.
  - Стандартные PNG/ICO иконки для bundle не созданы (только tray.svg из legacy). Не блокер для `tauri dev`, но `tauri build` потребует генерации.

### F6 — ttsd Python skeleton
- status: **merged**
- branch: task/f6-ttsd-skeleton
- worker_commit: `54eefd9 feat(ttsd): Python subprocess skeleton with JSON protocol dispatcher`
- reviewer: autopilot (Opus), review_result: ok
- merge_sha: `7db9ae4 merge(f6): ttsd Python subprocess skeleton`
- deps_met: F1 merged.
- next_unblocks: **F7** (Silero wrapper).
- deliverable: `ttsd/pyproject.toml` + `ttsd/ttsd/` (`__init__.py`, `__main__.py`, `main.py`, `protocol.py`) + `ttsd/tests/` (unit + smoke).
- протокол: Pydantic v2 с discriminated union через `TypeAdapter[Request]` (потому что `Annotated[Union, discriminator]` — не класс BaseModel). Модели `WarmupRequest`, `SynthesizeRequest` (с optional `char_mapping`), `ShutdownRequest`, `OkWarmup`, `OkSynthesize`, `OkShutdown`, `ErrResponse` с Literal-кодами (`model_not_loaded`/`synthesis_failed`/`bad_input`/`internal`).
- reviewer verified on host (через venv pip install pydantic pytest):
  - pytest на test_protocol.py: 17/17 passed.
  - smoke subprocess exchange: warmup → `{"ok":true,"version":"0.1.0"}`, synthesize → `model_not_loaded`, shutdown → exit 0. Все по протоколу.
  - ruff: dynamic binary, пропущено ревьюером тоже (но код чист по pyflakes).
- minor issues не-блокеры:
  - `pytest.mark.slow` не зарегистрирован в `[tool.pytest.ini_options]` → warnings при запуске (follow-up коммит).
  - `OkWarmup.version` — расширение относительно строгой спеки из `docs/ipc-contract.md` (`{"ok": true}`). Обратно совместимо.
- `uv.lock` пока не сгенерирован (sandbox). Будет создан при первом `uv sync` в F7.

---

## Подготовка к Wave 2

После merge F1:
- **Unblocked:** F2 (Tauri init), F6 (ttsd skeleton).
- **Запланированный запуск:** F2 + F6 параллельно (оба зависят только от F1).

После merge F2:
- **Unblocked:** F4 (storage schema), B3 (player), B5 (tray), U1 (app shell).
- F4 разблокирует B1 (storage service), R1 (TrackedText).

После merge F6:
- **Unblocked:** F7 (Silero wrapper).

После merge F7 + F3 (уже merged) → **unblocked B2** (TTS subprocess manager).

---

## Wave 3 — результаты

### F7 — Silero wrapper + timestamps
- status: **merged**
- branch: task/f7-silero-wrapper
- worker_commit: `4d4a703 feat(ttsd): Silero wrapper and timestamp estimation`
- reviewer: autopilot (Opus), review_result: ok (статический ревью — nix-shell в sandbox ревьюера тоже заблокирован)
- merge_sha: `de88622 merge(f7): Silero wrapper and timestamp estimation`
- notes: `SileroEngine` с load()/synthesize()/chunking/sanitize — точный порт из legacy. `timestamps.estimate_timestamps_chunked` поддерживает 3 формы char_mapping (dict span-list, dict-обёртка `{"char_map":[...]}`, позиционный список). main.py корректно распаковывает `char_mapping` через `model_dump()`.
- gap: `uv.lock` всё ещё не сгенерирован. Требуется отдельная task-овер на хосте.

---

## Wave 4 — запуск (unblocked после F2, F7)

### F4 — Storage schema (doc + Rust types)
- status: **awaiting review**
- branch: task/f4-storage-schema
- worker_commits:
  - `3ccb411 docs(storage): storage schema documentation`
  - `1c94b43 feat(storage): schema types with serde, unit tests`
- deps_met: F2 merged.
- next_unblocks: R1 (TrackedText), B1 (Storage service).
- verify: 9/9 unit-тестов прошли в изолированном test-крейте (полный `cargo test` не запускался — sandbox). `docs/storage-schema.md` + `src-tauri/src/storage/schema.rs` + обновлён `Cargo.toml` (chrono, uuid).
- note: поле `cache_dir` из legacy UIConfig опущено — runtime, не JSON.

### U1 — AppShell + theme switcher
- status: **awaiting review**
- branch: task/u1-app-shell
- worker_commit: `254e8a0 feat(ui): AppShell with theme switcher and typed Tauri bindings`
- deps_met: F2 merged.
- next_unblocks: U2, U3, U4, U6, U9, U10 (некоторые также зависят от B4).
- verified: `pnpm install` ok (113 пакетов), `pnpm typecheck` чисто, `pnpm build` ok (789 модулей, 1.04 с).
- minor deviations: `src/lib/tauri.ts` дополнен полями из docs/ipc-contract.md (`was_regenerated` в TextEntry, расширенный UIConfig, payload пустых событий через `Record<string, never>` вместо null).

### B2 — TTS subprocess manager
- status: **not_started (rate limit)**
- branch: task/b2-tts-manager (создана, 0 коммитов)
- агент упёрся в API-лимит до начала работы. Перезапустить после сброса лимита.

### B3 — Audio player (tauri-plugin-mpv)
- status: **not_started (rate limit)**
- branch: task/b3-player (создана, 0 коммитов)
- агент упёрся в API-лимит. Перезапустить.

### B5 — System tray + menu
- status: **not_started (rate limit)**
- branch: task/b5-tray (создана, 0 коммитов)
- агент упёрся в API-лимит. Перезапустить.

### F4 reviewer
- status: **didn't finish (rate limit)**
- агент Opus упёрся в лимит; ни merge, ни решения. F4 остаётся awaiting_review до перезапуска ревьюера.

---

## Rate limit hit

**Время:** 2026-04-19.
**Лимит:** Claude API quota пользователя исчерпан. Резервируется в **14:00 Asia/Tomsk**.
**Затронуты:** B2 worker, B3 worker, B5 worker, F4 reviewer — все завершились с `"You've hit your limit"` без полезной работы.

После сброса лимита (координатор):
1. Перезапустить ревьюера для F4 (1 Opus-инстанс) — merge F4 в ruvox2.
2. Ревьюер для U1 — merge U1 в ruvox2.
3. После merge F4 → запустить R1 + B1 воркеров (unblocked).
4. Перезапустить B2 / B3 / B5 воркеров на тех же ветках (они пусты — начнут с нуля).

Ручных действий от пользователя **не требуется**. Если хочется ускорить прогресс — проверить на хост-машине `nix-shell --run "cargo build"` и `nix-shell --run "pnpm tauri dev"` на текущем ruvox2 HEAD (`de88622`), чтобы поймать проблемы до merge F4/U1.

Текущее состояние ruvox2 HEAD: `de88622 merge(f7): Silero wrapper and timestamp estimation`.

---

## Wave 4 — после сброса лимита

### F4 — Storage schema (re-review после fix)
- status: **merged**
- worker_commits: `3ccb411`, `1c94b43`
- fix_commit: `7d2efbb fix(storage): naive datetime for legacy compat + real-format test`
- reviewer: autopilot Opus × 2 (1-й → needs_fix; 2-й → ok после fix)
- merge_sha: `ea01e9b merge(f4): storage schema doc + Rust types (with naive datetime fix)`
- issue_fixed: `created_at: DateTime<Utc>` → `NaiveDateTime`. Legacy format `"2026-02-15T11:46:51.504055"` без TZ теперь парсится. Добавлен тест `deserialize_real_legacy_history`. `EntryStatus::Playing` с doc-комментарием runtime-only. `player_hotkeys` через default function. `theme: "auto"` задокументировано как намеренное отклонение от legacy `"dark_pro"`.
- tests: 9/9 в изолированном storage-крейте (полный cargo build заблокирован sandbox для обоих — worker и reviewer).
- unblocks: R1, B1.

### U1 — AppShell + theme switcher + typed Tauri bindings
- status: **merged**
- worker_commit: `254e8a0 feat(ui): AppShell with theme switcher and typed Tauri bindings`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `78a0034 merge(u1): AppShell + theme switcher + typed Tauri bindings`
- verified: `pnpm install` (113 пакетов), `pnpm typecheck` чисто, `pnpm build` ok (789 модулей, 1.03s).
- minor: `.gitignore` получил `!src/lib/` (legacy правило `lib/` Python-dir блокировало `src/lib/`). `src/lib/tauri.ts` 1:1 с `docs/ipc-contract.md` (`TextEntry.was_regenerated`, расширенный UIConfig, `Record<string, never>` для пустых payload).
- Mantine 8 правила соблюдены: CSS Modules, никакого `sx`/`createStyles`/emotion/`React.FC`.
- unblocks: U4, U10 (частично — остальные U* ждут B4).

---

## Wave 5 — после merge F4 и U1

### B5 — System tray (first review: needs_fix)
- status: **fix in progress (v2)**
- worker_commit: `e23f0e8 feat(tray): system tray icon with menu (show, read-now, read-later, settings, quit)`
- reviewer: autopilot Opus, review_result: **needs_fix**
- issues:
  1. `tray.png` не в `bundle.resources` → runtime `resource_dir().join("icons/tray.png")` не найдёт файл, срабатывает fallback на window icon.
  2. Single-click handler (`TrayIconEvent::Click`) вместо double-click (спека требует).
  3. `cargo build` не верифицирован в sandbox.
  4. `read_now`/`read_later` через `app.emit()`, а не прямой вызов `add_clipboard_entry` (B4 ещё не готов — допустимо, но нужен TODO-комментарий).
  5. `tray.svg` остался в repo alongside PNG — источник без назначения.
- fix_spec для sonnet-агента:
  - Заменить runtime-загрузку на `Image::from_bytes(include_bytes!("../../icons/tray.png"))` — компилируется в бинарь.
  - `TrayIconEvent::DoubleClick` вместо `Click`.
  - TODO-комментарий у `tray_read_now`/`tray_read_later` про B4.
  - `git rm src-tauri/icons/tray.svg`.
- fix_agent: запущен (autopilot-sonnet), коммит ожидается.

### B2 — TTS subprocess manager
- status: **awaiting review**
- worker_commit: `0255082 feat(tts): subprocess manager with JSON protocol over stdin/stdout`
- deliverable: `src-tauri/src/tts/mod.rs` (~450 строк). `TtsSubprocess::spawn(ttsd_dir)` через `tokio::process::Command` + `kill_on_drop`. Driver task через `mpsc::channel(1)` — один in-flight request. Stderr ttsd → `tracing::info!(target: "ttsd")`. API: `warmup`, `synthesize(text, speaker, sample_rate, out_wav, char_mapping)`, `shutdown`. Timeout 5 мин. `TtsError` через thiserror.
- TODO: auto-restart при крэше subprocess (v2+).
- tests: 8/8 unit-тестов в sandbox (cargo через $TMPDIR/CARGO_HOME с системными путями).
- note: воркер добавил placeholder PNG иконки (32×32, 128×128@2x) — нужны для `tauri::generate_context!()` на этапе компиляции. **Ревьюер должен проверить**, что это не дублирует работу из других веток.

### B3 — Audio player (tauri-plugin-mpv)
- status: **awaiting review**
- worker_commit: `21eb92e feat(player): libmpv wrapper with scaletempo2 and position events`
- crate_chosen: **tauri-plugin-mpv v0.5.2** (github.com/nini22P/tauri-plugin-mpv, MPL-2.0). mpv запускается как subprocess с JSON IPC сокетом. Альтернатива `libmpv2 5.0.3` отвергнута в пользу нативной Tauri-интеграции.
- mpv args: `--no-video --af=scaletempo2 --audio-pitch-correction=no --no-ytdl`.
- API: `load`, `play`, `pause`, `resume`, `stop`, `seek`, `set_speed`, `set_volume`, `position_sec`, `duration_sec`, `current_entry_id`, `is_playing`.
- events: `playback_started/paused/stopped/finished/position` (per ipc-contract).
- `spawn_position_emitter` — tokio-task с 100 ms polling.
- cargo build: ок (инкрементально 2s). Test: 1 passed, 1 ignored (нужен mpv runtime).
- known_gaps: EOF detection через polling (порог `position >= duration - 0.05s`) — может пропустить очень короткие (<50ms) файлы. IPC round-trip через Unix socket на каждый `get_property` — приемлемо при 100ms.

### U10 — Notifications integration
- status: **awaiting review**
- worker_commit: `22be80b feat(ui): wire backend events to Mantine notifications`
- deliverable: `src/lib/notificationBridge.ts` подписывает на 5 событий: `model_loading` (loading spinner), `model_loaded` (зелёная 3s), `model_error` (красная 8s), `tts_error` (красная 5s, id per entry_id), `synthesis_progress` (update с процентом).
- App.tsx интегрирует bridge через `useEffect` + cleanup.
- verified: нет (sandbox). Reviewer верифицирует на хосте.

### U4 — Text viewer (plain + markdown)
- status: **merged**
- worker_commit: `fe31bb0 feat(ui): text viewer with plain + markdown modes`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `3eae9db merge(u4): text viewer with plain + markdown modes`
- deliverable: `src/components/TextViewer.tsx`, `TextViewer.module.css` (light-dark() CSS func), `src/lib/markdown.ts` (markdown-it 14.1.1 + highlight.js 11.11.1, `html: false`), `src/vite-env.d.ts`. Обновлены AppShell.tsx, main.tsx, package.json, pnpm-lock.yaml.
- verified: `pnpm install/typecheck/build` ок (sandbox-workaround прямым node из nix-store).
- XSS-safe: plain через `escapeHtml()`, markdown через `html: false`.
- minor: `markdown-it-highlightjs` добавлен в deps, но не используется (highlight через callback inline). Не блокер.
- deferred: word highlighting (U5), queue selection (U2), mermaid (U7 — теперь запущен), HTML mode (U8).
- unblocks: U7 (сейчас запущен). U5 ждёт ещё B3+B4. U8 ждёт B4.

### B1 — Storage service в Rust
- status: **awaiting review**
- worker_commit: `fd4b105 feat(storage): StorageService with CRUD and audio/timestamps persistence`
- API: `new/with_cache_dir/add_entry/get_entry/update_entry/delete_entry/delete_audio/get_all_entries/save_audio/save_timestamps/load_timestamps/get_audio_path/get_cache_size/get_audio_count/load_config/save_config`.
- atomic writes через `write tmp + rename`.
- deviation: `dirs = "6"` вместо `"5"` — в sandbox cargo-cache есть только 4.x и 6.x (API совместим).
- extra: `ready` без `audio_path` → `pending` (соответствует storage-schema.md, legacy пропускал этот случай).
- tests: 24/24 в изолированном мини-крейте (15 новых + 9 из F4 schema).

### R1 — TrackedText + CharMapping (Rust)
- status: **merged**
- branch: task/r1-tracked-text
- worker_commit: `b6279e8 feat(pipeline): TrackedText + CharMapping port from Python`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `d8114a3 merge(r1): TrackedText + CharMapping port`
- merge_note: ревьюер использовал plumbing (`merge-tree` + `commit-tree` + `update-ref`) потому что main worktree в момент merge был на task/u7-mermaid с чужой work-in-progress.
- minor_followup: `current_to_original` использует `(i64 - i64) as usize` без underflow guard — добавить `saturating_sub` или `debug_assert` когда будет время.
- integration-тесты из `test_char_mapping.py` с фикстурой `pipeline` не портированы — будут в R3/R5.
- **big unblock:** запущены R2, R3, R4, R5, R7 параллельно (5 нормалайзеров).

### U7 — Mermaid rendering
- status: **awaiting review**
- branch: task/u7-mermaid
- worker_commit: `e507177 feat(ui): inline Mermaid rendering in text viewer`
- deps_met: U4 merged.
- deliverable: `mermaid@^11` в package.json, `src/lib/mermaid.ts` (initMermaid + renderMermaidIn с theme sync), fence renderer в markdown.ts возвращает `<div class="mermaid">` для mermaid-блоков, `TextViewer` вызывает `renderMermaidIn` через `useEffect`, click-to-zoom модалка (бонус).
- verified: нет (sandbox). Ревьюер проверит pnpm install/typecheck/build на хосте.
- theme sync: `data-processed` сбрасывается перед `mermaid.run()` при смене темы для перерендера.
- crates added: `regex = "1"`.
- tests: **35/35 passed** в изолированном мини-крейте (sandbox без nix-shell). Порт из `legacy/tests/tts_pipeline/test_tracked_text.py` + `test_char_mapping.py` + несколько расширенных edge-cases (nested replacement, char_map length после expansion/contraction).
- architecture: **Unicode codepoint индексы** в `CharMapping.char_map` (не байтовые) — семантически идентично Python `str`. Вспомогательные функции `byte_to_char_idx`/`char_to_byte_idx`/`char_len`. Важно для корректной работы с кириллицей.
- API: `TrackedText::new/text/replace/sub/build_mapping`, `CharMapping::get_original_range/get_original_word_range`. `TrackedText.original` — публичное поле (как в Python).
- deviations:
  - `TrackedText::sub` не поддерживает `count=N` параметр — идиома Rust: счётчик в замыкании.
- next_unblocks: R2, R3, R4, R5, R7 (4 нормалайзера + код: R6 ждёт ещё R2+R3, R8 ждёт R5+R7).

### B5-fix — после первого ревью (needs_fix) → merged
- status: **merged**
- branch: task/b5-tray
- fix_commit: `6874905 fix(tray): embed icon via include_bytes, use double-click, add B4 TODO`
- reviewer: autopilot Opus (после fix), review_result: ok
- merge_sha: `abd78aa merge(b5): system tray with menu (fixed)`
- changes:
  - `load_tray_icon` → `Image::from_bytes(include_bytes!("../../icons/tray.png"))`.
  - `TrayIconEvent::DoubleClick`.
  - TODO-комментарий у `tray_read_now/tray_read_later` про B4.
  - `git rm src-tauri/icons/tray.svg`.
- merge note: конфликт в `src-tauri/src/lib.rs` разрешён — добавлен `pub mod tray;` рядом с `pub mod pipeline;` и `pub mod storage;` от R1/B1-F4. Cargo.toml auto-merge (features `tray-icon` и `image-png`).
- **follow-up (future):** `TrayIconEvent::DoubleClick` помечен как "Windows Only" в Tauri docs — на Linux может не срабатывать. Рекомендация: добавить platform guard + fallback `Click { button_state: Up, .. }` для Linux. Не блокер для MVP.

### Следующие действия координатора

После завершения B5-fix → перезапуск ревьюера для B5.
После U4-review → merge → разблокирует U5 (ждёт ещё B3/B4), U7, U8 (частично).
После R1 merge → запуск R2-R7 параллельно (6 воркеров — вплотную к лимиту 7).

---

## Wave 6 — Rust normalizers (после merge R1)

### R2 — NumberNormalizer (Rust)
- status: **merged**
- branch: task/r2-numbers-normalizer
- worker_commit: `75b954c feat(pipeline): port NumberNormalizer to Rust`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `664172b merge(r2): NumberNormalizer port`
- tests: **103/103** в мини-крейте. Классы: Integers (21), Floats (10), Percentages (11), Ranges (8), SizeUnits (18), Versions (12), Dates (7), Times (12).
- spot-check ревьюера: `normalize_date("01.12.2023")` → `первое декабря две тысячи двадцать третьего года` — совпадает с Python. `int_to_words_with_gender` покрывает миллиарды/миллионы/тысячи с женским родом для 1/2.
- followup_perf: `to_genitive` использует `Regex::new` внутри цикла — можно оптимизировать через OnceLock в R9.
- TestOrdinalNumbers пропущен (placeholder `pass`).

### R4 — AbbreviationNormalizer (Rust)
- status: **awaiting review**
- branch: task/r4-abbreviations-normalizer
- worker_commit: `bebd754 feat(pipeline): port AbbreviationNormalizer to Rust`
- crates: `std::sync::LazyLock` (Rust 1.80+), `once_cell` не потребовался.
- tests: **109/109** в изолированном мини-крейте.
- логика: точно повторяет Python — `SPECIAL_CASES` → `AS_WORD` → single letter → `spell_out` → `handle_mixed`.

### R3 — EnglishNormalizer (Rust)
- status: **merged**
- branch: task/r3-english-normalizer
- worker_commit: `2f3f077 feat(pipeline): port EnglishNormalizer to Rust`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `de9bbad merge(r3): EnglishNormalizer port`
- crates: `aho-corasick = "1"`, `once_cell = "1"`.
- tests: **53/53**.
- merge_conflict: `normalizers/mod.rs` add/add — разрешён (обе строки `pub mod english;` + `pub mod numbers;`).
- deviation: G2P не портирован (нет Rust-эквивалента).
- **big unblock:** R6 (URLPathNormalizer) запущен — зависел от R1+R2+R3, все merged.

### R7 — SymbolNormalizer + constants (Rust)
- status: **awaiting review**
- branch: task/r7-symbols-normalizer
- worker_commit: `89cdc71 feat(pipeline): port SymbolNormalizer + constants to Rust`
- crates: `once_cell = "1"`.
- tests: **111/111** в mini-crate (`src-tauri/pipeline-core/`) + **146** в основном крейте. Покрытие test_symbols.py + доп. тесты для MATH_SYMBOLS и ARROW_SYMBOLS (которых в Python не было).
- note: агент создал mini-crate `src-tauri/pipeline-core/` для изолированного тестирования. Возможно потенциальный конфликт если R2/R3/R4/R5 тоже создавали mini-crate'ы — ревьюер разрешит.
- note: опять placeholder иконки для `generate_context!()`.

### R5 — CodeNormalizer (Rust)
- status: **awaiting review**
- branch: task/r5-code-normalizer
- worker_commit: `08bff2e feat(pipeline): port CodeNormalizer to Rust`
- crates: нет новых.
- tests: **61/61** в основном крейте (camelCase 19, PascalCase 8, snake_case 16, kebab-case 13, SCREAMING_SNAKE_CASE 3 + TestMixedIdentifiers).
- cross-normalizer: `CodeIdentifierNormalizer::new()` без аргументов. Встроены собственные `number_to_russian()` (0-90 + 64/256) и `spell_abbreviation()`. Будут заменены ссылками на R2/R4 в R9.
- deferred: CodeBlockHandler — R8.
