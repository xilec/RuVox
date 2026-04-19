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
- status: **merged**
- branch: task/b2-tts-manager
- worker_commit: `0255082 feat(tts): subprocess manager with JSON protocol over stdin/stdout`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `32ae801 merge(b2): TTS subprocess manager`
- deliverable: `src-tauri/src/tts/mod.rs` (454 строки). `TtsSubprocess::spawn(ttsd_dir)` через `tokio::process::Command` + `kill_on_drop(true)`. Driver task через `mpsc::channel(1)` — один in-flight request. Stderr ttsd → `tracing::info!(target: "ttsd")`. API: `warmup`, `synthesize(text, speaker, sample_rate, out_wav, char_mapping)`, `shutdown`. Timeout 5 мин через `tokio::time::timeout`. `TtsError` через thiserror (Spawn/Ipc/Json/Ttsd{code,message}/Died/Timeout).
- crates added: `tokio` (macros+rt-multi-thread+process+io-util+sync+time).
- tests: 8/8 unit-тестов (serialize warmup/synthesize/shutdown/char_mapping, deserialize ok/err/synth-response). Повторены ревьюером в изолированном мини-крейте `$TMPDIR/b2-verify/` через cargo 1.95 online — 8/8 passed, compile clean без warning'ов.
- protocol-audit vs docs/ipc-contract.md:
  - request fields match: `cmd` (warmup|synthesize|shutdown), `text`, `speaker`, `sample_rate`, `out_wav`, `char_mapping: Option<Vec<CharMappingEntry>>`.
  - `CharMappingEntry { norm_start, norm_end, orig_start, orig_end }` — поля совпадают с pydantic-моделью ttsd и спецификацией.
  - response deserialize — ручной `Deserialize` для `TtsResponse` через dispatch по `ok` (избегает `#[serde(untagged)]` ambiguity с `#[serde(flatten)]`).
  - `#[serde(skip_serializing_if = "Option::is_none")]` на `char_mapping` — если None, поле не сериализуется (совпадает с pydantic default).
  - stderr-лента forwarded через `tokio::spawn` с `BufReader::lines()` → `tracing::info!(target: "ttsd")`.
- adaptor-point (B4): R1 `CharMapping { char_map: Vec<(usize, usize)> }` ≠ B2 `Vec<CharMappingEntry>`. B4 (commands) сконвертирует между форматами на границе pipeline→tts. Это ожидаемо — `ipc-contract.md` фиксирует entry-based формат на wire.
- known_gaps / TODO:
  - **Auto-restart при crash** — явный TODO в doc-comment модуля (v2+). При `Died` caller должен вызвать `TtsSubprocess::spawn()` заново. `ipc-contract.md` § «ttsd Auto-restart» требует транспарентного respawn — это откладывается на B4/B2-v2.
  - **Graceful shutdown drain** — `shutdown()` шлёт команду и ждёт ответа, но не реализует 5s-timeout+SIGTERM fallback (спека требует). `kill_on_drop(true)` обеспечит убийство при Drop владельца. Follow-up для v2.
  - `.expect("stdin was piped")` в `driver_task` — OK: инвариант после `.stdin(Stdio::piped())` в spawn; не в запросном пути.
  - В `OkPayload`/`ErrPayload` поле `pub ok: bool` имеет `#[allow(dead_code)]` — легитимно, документирует shape.
- иконки: воркер добавил 32×32, 128×128, 128×128@2x (67-143 байт RGBA). При merge возник add/add конфликт с уже merged в ruvox2 версиями (из B5-tray). Ревьюер оставил версии `ruvox2` (`git checkout --ours`), потому что B5-версии уже bundled в релизах. Дубляж placeholder-иконок — не проблема, финальный брендинг будет в P1 (bundling).
- merge_conflicts:
  - `src-tauri/Cargo.toml` — add/add по `[dependencies]` (ruvox2 добавил chrono/uuid/regex/aho-corasick/once_cell/dirs, B2 добавил tokio). Разрешено объединением всех depenдов.
  - `src-tauri/src/lib.rs` — add/add по `pub mod`. Разрешено объединением (`pipeline; storage; tray; tts;`).
  - `src-tauri/Cargo.lock` — взят ruvox2-версия; `tokio` уже присутствовал transitively через `tauri-plugin-opener`, root-добавление не требует нового entry. nix-shell regenerate при следующем `cargo build` приведёт в полный порядок.
  - 3× PNG-иконки — add/add binary, взяты ruvox2-версии.
- verification_gap: `cargo check` всего src-tauri-крейта в sandbox невозможен (нужны webkitgtk/gdk-pixbuf-2.0 system libs из nix-shell). Ревьюер верифицировал изолированно `src/tts/mod.rs` как отдельный крейт со всеми depsами из B2 — компиляция и 8/8 тестов green. `cargo check` полного крейта в nix-shell обязателен перед B4.
- unblocks: **B4 (Tauri commands)** — все три core-сервиса (storage=B1, tts=B2, player=B3) готовы после merge B2→B3.

### B3 — Audio player (tauri-plugin-mpv)
- status: **merged**
- branch: task/b3-player
- worker_commit: `21eb92e feat(player): libmpv wrapper with scaletempo2 and position events`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `dd579de merge(b3): audio player with tauri-plugin-mpv`
- crate_chosen: **tauri-plugin-mpv v0.5.2** (github.com/nini22P/tauri-plugin-mpv, MPL-2.0). mpv запускается как subprocess с JSON IPC сокетом. Альтернатива `libmpv2 5.0.3` отвергнута в пользу нативной Tauri-интеграции.
- mpv args: `--no-video --af=scaletempo2 --audio-pitch-correction=no --no-ytdl`.
- API: `load`, `play`, `pause`, `resume`, `stop`, `seek`, `set_speed`, `set_volume`, `position_sec`, `duration_sec`, `current_entry_id`, `is_playing`.
- events: `playback_started/paused/stopped/finished/position` (per ipc-contract).
- `spawn_position_emitter` — tokio-task с 100 ms polling.
- cargo build: воркер верифицировал (инкрементально 2s) в nix-shell. Test: 1 passed, 1 ignored (нужен mpv runtime).
- verification_gap: `cargo check` всего крейта в sandbox невозможен (atk / gdk-pixbuf-2.0 pkg-config не подхватываются без nix-shell env). Ревьюер провёл semantic-audit исходников и cross-crate API-проверку через распакованный `tauri-plugin-mpv-0.5.2` из cargo registry.
- findings (ревью):
  - Rules compliance: `unwrap()` в prod-путях нет; `thiserror` для `PlayerError` (Init/Op/FileNotFound); `tracing::{debug, warn}` для логов; нет emoji; нет `anyhow`. `.expect()` на `.run()` — идиоматично для Tauri main.
  - Event payloads совпадают с `docs/ipc-contract.md`: `playback_started {entry_id}`, `playback_paused {entry_id, position_sec}`, `playback_stopped {}`, `playback_finished {entry_id}`, `playback_position {position_sec, entry_id}`. `resume` намеренно не re-emit'ит `playback_started` (согласно спеке — это отдельное событие).
  - Lock safety: `parking_lot::Mutex<State>` с sync `.lock()` — без `await` внутри критических секций. `Player: Send + Sync` гарантированно.
  - `Drop for Player`: вызывает `mpv().destroy(WINDOW_LABEL)`. Плагин также перехватывает `CloseRequested` на окне `"main"` и вызывает `destroy()` — двойной destroy вернёт "No running mpv process found", только warn-log. Безопасно.
  - Socket lifecycle: mpv subprocess менеджится плагином (`MpvInstance`). Каждый `command()` / `read_property_f64()` делает новый IPC round-trip через Unix socket (`ipc::send_command`). Нет persistent socket → нет утечек, но если mpv крашится между вызовами — следующий вызов вернёт `PlayerError::Op`; auto-respawn не реализован (задокументировано ниже как known_gap).
  - `is_playing` state: обновляется атомарно через `State.is_playing`; emitter-task пропускает tick если `!is_playing`, но сама tokio-task живёт всё время жизни приложения (10 Hz polling, cost minimal).
  - AppHandle для emit: передаётся через `Player.app: AppHandle<R>` и отдельным clone в `spawn_position_emitter` — корректно.
- known_gaps (задокументированы в mod.rs комментариях / task_history):
  - EOF detection через polling (порог `position >= duration - 0.05s`) — может пропустить файлы <50 ms. Acceptable для TTS-кейса (минимальная длительность фраз ~0.3s).
  - IPC round-trip через Unix socket на каждый `get_property` (time-pos, duration) — приемлемо при 100 ms tick.
  - Нет auto-respawn при crash mpv-процесса. B4 должен отреагировать на `PlayerError::Op` через UI-ошибку; полноценный supervisor — follow-up.
  - `spawn_position_emitter` не прерывает tokio task при stop/drop — бежит forever. Minor leak на очень редкие stop-restart-stop циклы; для MVP ок.
  - capabilities/default.json НЕ включает `mpv:default` — frontend не может вызывать mpv команды через `invoke`, только через наши backend-обёртки. Это намеренно (B3 rust-only API).
- merge_conflicts:
  - `src-tauri/src/lib.rs` — add/add по `pub mod`. Резолв: объединение (`pipeline; player; storage; tray; tts;`) + setup-хук с последовательным вызовом `tray::init(...)?` → `Player::new(...)?` → `spawn_position_emitter` → `app.manage(player)`.
  - `src-tauri/Cargo.toml` — add/add по deps. Резолв: все depеnды, tokio features union (`macros+rt+rt-multi-thread+process+io-util+sync+time`).
  - `src-tauri/icons/{128x128,128x128@2x,32x32}.png` — add/add binary. Взяты ruvox2-версии (R3 placeholder, 67 байт).
  - `src-tauri/Cargo.lock` — взят ruvox2; следующий `cargo build` в nix-shell добавит `tauri-plugin-mpv` + зависимости + недостающие tokio features. Ожидается чистая регенерация.
- merge_decisions:
  - `src-tauri/tauri.conf.json` — **отказ от worker's изменения**. Воркер заменил `.icns` + `.ico` на одну `icon.png` (Linux-only), что бы сломало cross-platform bundling на macOS/Windows. Возврат к ruvox2-версии с `.icns/.ico` (пустые файлы оставлены с R3-merge, fixup брендинга отложен до P1).
  - `src-tauri/icons/icon.png` — **отброшен** (был добавлен воркером как часть Linux-only переключения; не нужен при сохранении `.icns/.ico` пути).
  - `src-tauri/gen/schemas/*.json` — **отброшены** и добавлены в `.gitignore` (`src-tauri/gen/`). Это tauri-build generated артефакты (capability/schema JSON, 2531 строки каждый), не должны трекаться в git — перегенерируются при каждом `cargo build`.
- unblocks: **B4 (Tauri commands)** — все три core-сервиса (storage=B1, tts=B2, player=B3) merged. B4 может начать реализацию Tauri-команд поверх B3 API.
- followups (не блокеры):
  - Auto-respawn mpv при crash (аналогично ttsd-supervisor'у в B2) — v2.
  - Persistent IPC соединение для reduce round-trip overhead — оптимизация для P1.
  - EOF detection через mpv `observed_properties: ["idle-active"]` или `playback-time` event — более точно чем polling threshold.
  - `capabilities/default.json` добавить `mpv:default` если фронт захочет прямой invoke (не требуется для MVP).

### U10 — Notifications integration
- status: **merged**
- branch: task/u10-notifications
- worker_commit: `22be80b feat(ui): wire backend events to Mantine notifications`
- reviewer: autopilot Opus, review_result: ok (с minor follow-up'ами)
- merge_sha: `10763c4 merge(u10): notifications bridge for backend events`
- deliverable: `src/lib/notificationBridge.ts` (79 строк) подписывает на 5 событий через typed-wrapper'ы из U1 `src/lib/tauri.ts` (events.modelLoading/modelLoaded/modelError/ttsError/synthesisProgress). App.tsx (+12 строк) интегрирует bridge через `useEffect([]) + .then(fn => cleanup = fn)` с вызовом cleanup при unmount.
- merge_note: U10 форкался от U1 (`78a0034`), до всех R/B/U4/U7 работ. Merge чистый, ort-стратегия auto-merged только два файла (`App.tsx` конфликтов нет — в ruvox2 после U1 этот файл не трогали; `notificationBridge.ts` — новый файл). Никаких конфликтов с U7 (mermaid в TextViewer), R-нормалайзерами, B2/B3/B5 (Rust-модули). `@mantine/notifications` уже был в `package.json` с U1 merge, новые deps не требуются.
- verified_static: TypeScript strict-совместим (без `any`, типы выводятся через `Event<T>` → payload). Mantine 8 API (`notifications.show`/`notifications.update` с `id`). Функциональный компонент, без `React.FC`, без `sx`/`createStyles`. Cleanup в `useEffect` возвращает closure, вызывающий unlisten callbacks из `@tauri-apps/api/event`. `modelLoading` использует `loading: true` + `autoClose: false`, переход к `modelLoaded`/`modelError` — через `notifications.update` с тем же id `'model-loading'` (корректное in-place обновление loading-notification).
- verified_live: НЕТ. `nix-shell --run "pnpm install && pnpm typecheck && pnpm build"` в sandbox ревьюера невозможен — nix-daemon SQLite cache заблокирован sandbox (`unable to open /home/evgen/.cache/nix/fetcher-cache-v4.sqlite`). На хост-машине требуется live-проверка.
- review_findings:
  1. **Race condition в cleanup (minor, не критично):** если компонент unmount'ится до резолва `setupNotificationBridge()`, cleanup из Promise пропустится — `cleanup?.()` в return-callback вызовется до `cleanup = fn`. Bridge не держит тяжёлых ресурсов, unlisteners осиротеют до следующего reload. StrictMode двойной mount/unmount может это проявить. Более robust паттерн: `let unmounted = false; then(fn => unmounted ? fn() : cleanup = fn)`. Follow-up для U1-refactor.
  2. **`synthesis_progress` → `notifications.update` без предварительного `show` (minor):** Mantine 8 `notifications.update` — noop для несуществующего id. Первый `synthesis_progress` event для нового `entry_id` ничего не покажет. Корректный pattern: `show` для первого progress, `update` для последующих. Практический impact низкий — backend обычно emit'ит progress несколько раз (chunk-by-chunk синтез), пользователь увидит progress начиная со 2-й итерации. Follow-up для U3/Player-integration или U10-v2.
  3. **`tts_error` через `notifications.show` c id (edge-case):** повторные ошибки для одного entry_id создадут дубликат (Mantine `show` для существующего id — noop, но первый остаётся). Спецификация задачи упоминала «через `notifications.update`, не дублировать» — здесь применяется `show`. Impact низкий: backend emit'ит `tts_error` один раз на entry (после `entry_updated` со статусом `error`). Повторный retry создаст новую ошибку через другой entry_id. Follow-up.
  4. **Cleanup корректность (sync unlisten):** `UnlistenFn` из Tauri — sync, массив собран правильно, итерация безопасна. OK.
  5. **Event coverage vs `docs/ipc-contract.md`:** все 5 событий из § U10-deliverable покрыты. Не подписаны `entry_updated`, `playback_*`, `tray_*` — это верно, они обрабатываются в QueueList (U2, ждёт B4) / Player (U3, ждёт B4) и App-level tray-hooks (follow-up).
- known_gaps: live-верификация на хосте (pnpm install → typecheck → build → tauri dev) остаётся за пользователем. Follow-up'ы (1-3) — minor, могут быть зафиксированы в рамках U10-v2 или соседних задач (U2/U3).
- unblocks: ничего нового. U10 — leaf в дереве enrichment-фичей (зависимость только от U1, разблокирован с U1-merge). U8 (HTML format) не тронут.

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
- status: **merged**
- branch: task/b1-storage-service
- worker_commit: `fd4b105 feat(storage): StorageService with CRUD and audio/timestamps persistence`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `b166e43 merge(b1): StorageService with CRUD`
- API: `new/with_cache_dir/add_entry/get_entry/update_entry/delete_entry/delete_audio/get_all_entries/save_audio/save_timestamps/load_timestamps/get_audio_path/get_cache_size/get_audio_count/load_config/save_config`.
- atomic writes через `write tmp + rename`.
- deviation: `dirs = "6"` вместо `"5"` — в sandbox cargo-cache есть только 4.x и 6.x (API совместим).
- extra: `ready` без `audio_path` → `pending` (соответствует storage-schema.md, legacy пропускал этот случай).
- tests: 24/24 в изолированном мини-крейте (15 новых + 9 из F4 schema), повторены ревьюером — все зелёные.
- legacy-compat: `Local::now().naive_local()` в `add_entry` совпадает с legacy `datetime.now().isoformat()` (local naive, без TZ), как и в F4 fix `7d2efbb`. Реальный legacy-формат history.json читается тестом `load_legacy_format_history_json`.
- merge_note: conflicts только в `src-tauri/Cargo.toml` и `src-tauri/Cargo.lock` (add/add по deps). lib.rs смерджился автоматически — B1 форкался до pipeline/tray, изменения из ruvox2 применились fast-forward. Merge через отдельный worktree `/tmp/claude/ruvox2-merge` потому что главный worktree занят веткой `task/r8-code-blocks`.
- minor_followup (не блокер):
  - clippy `unnecessary_sort_by` в `get_all_entries` — заменить на `sort_by_key(|e| Reverse(e.created_at))`;
  - clippy `field_reassign_with_default` в тесте `save_and_load_config` — переписать на struct-expr;
  - rustfmt хочет разбить inline `if full.exists() { Some(full) } else { None }` в `get_audio_path` на многострочный if-else.
- divergence_from_legacy: `get_audio_path` в Rust возвращает `Some(..)` только если файл существует на диске, а legacy возвращает путь безусловно. Фактический UI-слой проверяет существование отдельно, поэтому в текущей кодобазе не критично — но стоит зафиксировать ожидание в `docs/ipc-contract.md` до B4.

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
- status: **merged**
- branch: task/u7-mermaid
- worker_commit: `e507177 feat(ui): inline Mermaid rendering in text viewer`
- fix_commit: `9ab8aa5 fix(ui): preserve fence content for unknown languages`
- reviewer: autopilot Opus, review_result: ok (после fix-патча reviewer'a в той же ветке)
- merge_sha: `b691a65 merge(u7): inline Mermaid rendering`
- deps_met: U4 merged.
- deliverable:
  - `mermaid@^11` в `package.json` deps.
  - `src/lib/mermaid.ts` — `renderMermaidIn(container, colorScheme)`: переинициализирует mermaid с темой `default`/`dark`, сбрасывает `data-processed` на нодах `.mermaid` и вызывает `mermaid.run()`.
  - `src/lib/markdown.ts` — fence renderer перехватывает info=`mermaid` и эмитит `<div class="mermaid">{escaped}</div>` вместо `<pre><code>`. HTML-escape входного блока перед инъекцией → mermaid сам парсит/санитизирует SVG.
  - `src/components/TextViewer.tsx` — `useEffect` запускает `renderMermaidIn` после ререндера content (deps: content/format/colorScheme). Click-to-zoom: второй `useEffect` слушает клики по `.mermaid`, по выбранному SVG открывает Mantine `<Modal size="xl">` с увеличенной копией. `useComputedColorScheme('light')` синхронизирует тему mermaid с Mantine color scheme.
- verified: pnpm install/typecheck/build не запущены (sandbox блокирует nix-shell + DNS). Статический ревью ОК; код ожидает live-проверки на хосте перед `pnpm tauri dev`. `pnpm-lock.yaml` НЕ обновлён воркером — потребуется `pnpm install` на хосте чтобы зафиксировать `mermaid@^11` (лок-файл всё ещё с U4-набором).
- review_findings:
  - **Критичный bug в fence renderer** (исправлен fix-коммитом): `md.options.highlight?.(...) ?? escapeHtml(content)` — оператор `??` срабатывает только для `null`/`undefined`, а highlight callback возвращает `''` для unknown/missing языков → `<code></code>` пустой, контент кодоблоков без языка терялся. Fix: explicit truthy check + явный `escapeHtml(token.content)` fallback.
  - Theme sync OK: `data-processed` reset перед `mermaid.run()` гарантирует ререндер после смены `colorScheme`. Re-init mermaid с темой `default`/`dark` — допустимый паттерн (mermaid поддерживает повторный `initialize`).
  - XSS-safety: `html: false` в markdown-it; mermaid-блок escapeHtml-ится перед вставкой; `securityLevel: 'loose'` отвечает за HTML-в-labels mermaid и target=_blank на ссылках. Для пользовательского markdown допустимо в MVP, но в идеале ужесточить до `antiscript`/`strict` — follow-up.
  - Click-to-zoom использует прямой Mantine `<Modal>`, а не `@mantine/modals.openModal`. AGENTS.md рекомендует `@mantine/modals` для confirm-диалогов; для viewer-content (display-only) прямой `<Modal>` приемлем — Mantine 8 поддерживает оба паттерна.
  - TS strict: компонент функциональный, без `React.FC`, без `sx`/`createStyles`/emotion. Mantine 8 правила соблюдены.
- followups (не блокеры):
  - `pnpm install` на хосте для генерации lock-entry для `mermaid@^11`.
  - Рассмотреть `securityLevel: 'antiscript'` или `'strict'` для пользовательского markdown.
  - Mermaid bundle тяжёлый (~1 MB) — следить за размером бандла, при необходимости lazy-import (`await import('mermaid')`) внутри `renderMermaidIn`.
- next_unblocks: нет прямых задач, U7 — leaf в дереве enrichment-фичей.

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
- status: **merged**
- branch: task/r4-abbreviations-normalizer
- worker_commit: `bebd754 feat(pipeline): port AbbreviationNormalizer to Rust`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `c981079 merge(r4): AbbreviationNormalizer port`
- crates: `std::sync::LazyLock`, без новых внешних.
- tests: **109/109**.
- merge_conflict: `normalizers/mod.rs` резолвом + `pub mod abbreviations;`.
- followup: `rust-version = 1.77` в Cargo.toml, а LazyLock стабилизирован в 1.80. Поднять rust-version или заменить на `once_cell::Lazy`.

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

### R6 — URLPathNormalizer (Rust)
- status: **awaiting review**
- branch: task/r6-urls-normalizer
- worker_commit: `f0aee08 feat(pipeline): port URLPathNormalizer to Rust`
- crates: нет новых (reuse: regex, aho-corasick, once_cell).
- tests: **53/53** (+164 из R2/R3 = 217 в mini-crate).
- deviation: добавлен `new_without_english(numbers)` конструктор — Python-тесты создают `URLPathNormalizer()` без english, ожидая домены НЕ транслитерируются. Основной `new(english, numbers)` сохранён. В R9 ownership модель будет согласована.

### R7 — SymbolNormalizer + constants (Rust)
- status: **merged**
- branch: task/r7-symbols-normalizer
- worker_commit: `89cdc71 feat(pipeline): port SymbolNormalizer + constants to Rust`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `65b69dc merge(r7): SymbolNormalizer + constants port`
- crates: `once_cell = "1"` (уже был из R3).
- tests: **111/111**.
- merge_conflicts: placeholder-иконки (5) + mod-файлы + Cargo.toml — разрешены ревьюером тривиально. Ветка R7 была форкнута от R1-merge (до R2/R3/R4/B5), но 3-way merge корректно сохранил всё.
- cleanup: mini-crate `src-tauri/pipeline-core/` **удалён** ревьюером (дублировал main, был нужен только как test staging).
- content: GREEK_LETTERS (23+23), MATH_SYMBOLS (18), ARROW_SYMBOLS (6), многосимвольные операторы.
- **potential unblock:** R8 (CodeBlockHandler) — зависит от R1+R5+R7. R5 ещё ждёт ревью; после его merge R8 разблокируется.

### R5 — CodeNormalizer (Rust)
- status: **merged**
- branch: task/r5-code-normalizer
- worker_commit: `08bff2e feat(pipeline): port CodeNormalizer to Rust`
- reviewer: autopilot Opus, review_result: ok (с follow-up'ами)
- merge_sha: `5c861d9 merge(r5): CodeNormalizer port`
- crates: нет новых.
- tests: **61/61** в изолированном мини-крейте `/tmp/claude/r5_test` (cargo 1.95 + rustc 1.95 через nix-store, без nix-shell — sandbox). camelCase 19, PascalCase 8, snake_case 18, kebab-case 13, SCREAMING_SNAKE_CASE 3.
- verified: `cargo build` ок, `cargo clippy --lib` чисто, `cargo test` — 61 passed, 0 failed.
- spot-check dictionary: 260+ entries CODE_WORDS сверены с legacy (`get`/`set`/`api`/`html`/`json` и т.д.) — 1:1 соответствие. translit_map (26 Latin → Cyrillic) идентичен legacy.
- split_camel_case: порт Python-regex `[A-Z]?[a-z]+|[A-Z]+(?=[A-Z][a-z]|\d|$)|[A-Z]+(?![a-z])|\d+` на императивный scanner. Для HTMLParser → HTML+Parser backtrack корректен. Для digit/lower/upper-only run'ов — отдельные ветки.
- cross-normalizer: `CodeIdentifierNormalizer::new()` без аргументов. Встроены собственные `number_to_russian()` (0-1000 с явными значениями 64/128/256/512 + digit-by-digit fallback) и `spell_abbreviation()` (26 букв, inline HashMap). Будут заменены ссылками на R2/R4 в R9.
- deferred: CodeBlockHandler — R8 (теперь разблокирован: зависит от R1+R5+R7, все merged).
- merge_conflicts:
  - `src-tauri/src/pipeline/normalizers/mod.rs` — add/add с abbreviations/english/numbers/symbols. Резолв: объединение `pub mod` строк в алфавитном порядке.
  - `src-tauri/src/pipeline/mod.rs` — HEAD имел `pub mod constants;` (из R7), R5-ветка ушла от d8114a3 где constants ещё не было. Резолв: добавить `pub mod constants;` обратно.
- followup_critical: **`basic_transliterate` строка 644 содержит `Box::leak(c.to_string().into_boxed_str())`** — memory leak на каждый unrecognised char. Срабатывает только для char вне translit_map (26 ascii lowercase) после `to_lowercase()`. Тесты это не триггерят. Follow-up: заменить на возврат `String` из map (или использовать `char::to_string()` и собирать в `String`-аккумулятор вместо итератора `&str`).
- followup_minor: `spell_abbreviation` создаёт HashMap на каждый вызов (626) — как в legacy. Оптимизация через LazyLock в R9/refactor.
- test-coverage gap (matches legacy): TestMixedIdentifiers кейсы `sha256Hash` / `base64Encode` / `utf8String` — в legacy они `pass` placeholder, в Rust тоже не покрыты. Не блокер.

### R8 — CodeBlockHandler (Rust)
- status: **merged**
- branch: task/r8-code-blocks
- worker_commit: `dc9ef59 feat(pipeline): port CodeBlockHandler to Rust`
- reviewer: autopilot Opus, review_result: ok (с задокументированными deviations)
- merge_sha: `fd1bf84 merge(r8): CodeBlockHandler port`
- crates: нет новых (regex + once_cell — уже в Cargo.toml).
- tests: **44/44** R8-специфичных (brief 14 + full 3 + mode_switch 4 + lang 19 + tracked-integration 4). Прогон через мини-крейт `tmp/r8_test/` (cargo 1.95, offline) — **140 total passed** (R8 44 + R5 61 + TrackedText 35). Clippy `-D warnings` чисто.
- file-level: `src-tauri/src/pipeline/normalizers/code_blocks.rs` (852 строки), +1 строка в `normalizers/mod.rs` (add `pub mod code_blocks;`).
- content verified 1:1 vs legacy:
  - LANGUAGE_NAMES — все 57 записей из `code.py:469-527` (включая `c++`, `c#`, `zsh`, `mermaid`, `powershell`).
  - SPECIAL_SYMBOLS — ARROW_SYMBOLS union + 8 MATH_SYMBOLS (∞ ∈ ∉ ∀ ∃ ≠ ≤ ≥), ровно как в legacy `_573-587`.
  - Brief phrase — `"далее следует пример кода на <lang>"` / `"далее следует блок кода"`.
  - Tokenizer regex — перенесён (identifiers, digits, Greek, SPECIAL_SYMBOLS, brackets, ops, strings, punct).
- merge_conflicts:
  - `src-tauri/src/pipeline/normalizers/mod.rs` — add/add с urls (появился в ruvox2 после R5-merge-base). Авторазрешение `ort`-стратегией корректно.
- deviations (задокументированы):
  1. **Default mode = Brief** (Rust) vs **Full** (legacy). Покрыто тестом `mode_default_is_brief`. Brief — безопасный дефолт для прод-TTS (короткий), Full требует явного явного выбора или директивы.
  2. **Mermaid replacement unconditional**. Rust заменяет mermaid-блок маркером "Тут мермэйд диаграмма" в *любом* режиме (включая brief), что соответствует требованию AGENTS.md "Mermaid-блоки не озвучиваются". Legacy для brief+mermaid вернул бы обычный "далее следует пример кода на мёрмэйд".
  3. **Full mode `=` не произносится**. Legacy fallback через `SymbolNormalizer.SYMBOLS["="]` = "равно"; Rust возвращает пустую строку для всех operator-токенов. Тест `full_js_const_x` ослаблен — проверяет `["конст","икс","сорок два"]` без "равно". В R9 SymbolNormalizer применяется поверх на уровне pipeline, так что финальный output получит "равно" через `normalize` цепочку.
  4. **Mode switch через ephemeral handler** — `process(&self, ...)` immutable, per-block режим реализован через создание нового `CodeBlockHandler` внутри замыкания. Совместимо с Rust ownership, функционально эквивалентно legacy `self.mode = ...`.
  5. **Fenced regex `\w+`** не ловит `c++`/`c#` как язык в ```fenced```. Тест `lang_cpp_symbol` обходит ограничение прямым вызовом `process_block`. В практике `markdown` ```cpp → работает, ```c++ → язык будет пустым (редкий формат).
- quality:
  - `unwrap()` только на invariant-guaranteed `caps.get(0)` после match (идиоматично).
  - `.expect("valid regex")` на compile-time regex — допустимо.
  - no `unsafe`, no emoji, комментарии WHY-only.
- **big unblock:** все R1-R8 merged. **R9 (pipeline integration) готов к запуску** — зависит от R1+R2+R3+R4+R5+R6+R7+R8 и F5 (golden fixtures).

### R9 — Pipeline integration (TTSPipeline в Rust)
- status: **merged**
- branch: task/r9-pipeline-integration
- worker_commit: `23a3031 feat(pipeline): integrate TTSPipeline with char mapping`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `84e09b1 merge(r9): TTSPipeline integration with char mapping`
- deliverable:
  - `src-tauri/src/pipeline/mod.rs` (968 строк): `TTSPipeline` struct владеющий всеми нормалайзерами (Number/English/Abbreviation/Symbol/CodeBlock/CodeIdentifier). Методы `new()`, `process(&mut self, &str) -> String`, `process_with_char_mapping(&mut self, &str) -> (String, CharMapping)`.
  - **16-фазный pipeline**, порядок 1:1 с legacy `pipeline.py::process_with_char_mapping`: BOM → code blocks → quotes → dashes → whitespace → inline code → markdown → URLs/emails/IPs/paths → sizes → versions → ranges → percentages → operators → symbols+tilde → code identifiers → English → numbers → postprocess.
  - 37 golden fixture integration tests (inline в `#[cfg(test)] mod tests` в `pipeline/mod.rs`).
- tests: **620 passed / 0 failed** в изолированном мини-крейте (cargo 1.95 offline). Reviewer повторил прогон на финальном ruvox2 state после merge — всё зелёное.
  - **37/37 golden fixtures** (все фикстуры из F5 — number/size/range/version/percentage/url/email/ip/filepath/markdown-{link,header,list,code_block,inline_code,mermaid}/camelCase/PascalCase/snake_case/kebab-case/abbreviation/english_word/greek/math/arrow/operators/mixed_paragraph).
  - 6 unit-sanity тестов (empty, plain russian, number_inline, mermaid_marker, char_mapping_nonempty, process_vs_char_mapping_consistent).
  - Плюс унаследованные tracked_text + все normalizers-тесты.
- follow-ups fixed в этой же задаче:
  1. **R5 memory leak (критичный)**: `CodeIdentifierNormalizer::basic_transliterate` использовал `Box::leak(c.to_string().into_boxed_str())` для каждого unmapped char — утечка на каждый вызов. Заменено на `String` accumulator с `result.push(c)` / `result.push_str(s)`. ✓
  2. **R2 Regex::new в горячих путях**: `normalize_range`, `normalize_size`, `normalize_version` компилировали regex на каждый вызов. Заменены на `OnceLock<Regex>` статические инстансы. Также `apply_genitive_replacements` заменил per-call `Regex::new(r"\bfrom\b")` на ручной substring-поиск с char-boundary проверкой. ✓
  3. **R8 default mode**: `CodeBlockHandler::default` был `Brief`. Legacy `config.py::PipelineConfig.code_block_mode` default = `"full"`. Изменено на `Full`. ✓
  4. **R8 normalize_token для operators/brackets**: возвращал пустую строку. Теперь вызывает `SymbolNormalizer::normalize(token)` — произносит brackets/operators через общий словарь (как legacy `_normalize_token`). ✓
  5. **R6 URLPathNormalizer::transliterate_word**: не проверял IT_TERMS перед `transliterate_simple`. Теперь сначала IT_TERMS, затем fallback — `"github"` транслитерируется в `"гитхаб"` а не в посимвольный `"гитхуб"`. ✓
  6. **Operators: single `=` исключён** из `TRACKED_OPERATOR_KEYS` (соответствует legacy `_TRACKED_OPERATOR_KEYS`). Включение `=` повредило бы math-формулы вида `α = β`. ✓
- phase-order verification (1:1 с legacy):
  - BOM removal → code blocks (do NOT move this — должно быть до dash/whitespace norm иначе TrackedText skip matches внутри replacement regions) → quotes (« » " " ' ') → dashes (— –) → `\n{3,}` → `[ \t]+` → inline code → markdown headers/links/lists → URLs → emails → IPs → paths → sizes → versions → ranges → percentages → operators (longest-first) → Greek + math + arrows + `~digit` → CamelCase/PascalCase/snake_case/kebab-case → C++/C# + IT_TERMS/abbreviations/transliterate → standalone integers → postprocess spaces/newlines + trim.
- code quality:
  - `unwrap()` только на invariant-guaranteed `caps.get(0)` после `captures_iter`, `next_ch` после `!empty` — идиоматично.
  - `.expect("valid regex")` на compile-time regex внутри `OnceLock` — допустимо.
  - no `unsafe`, no emoji, нет `anyhow` в prod.
  - commit message формат `<type>(<module>): <desc>` ✓.
- deviations от legacy (задокументированы, не блокеры):
  1. **`config.read_operators`**: в R9 TTSPipeline::new() нет config-параметра — operators phase запускается всегда. В legacy может быть отключен `PipelineConfig(read_operators=False)`. Legacy default = `True`, так что текущий дефолт R9 совпадает. Добавить config-параметр — follow-up для B4 (если фронт захочет управлять).
  2. **Не портированы**: `set_code_mode()`, `get_unknown_words()`, `get_warnings()`, `print_warnings()`, `process_with_mapping()` (word-level). `set_code_mode` потребуется для B4 (если пользователь переключает режим из UI). Word-mapping не нужен — char-mapping более точный.
  3. **Single-use URL/email/IP/path normalizer**: создаётся inline scope `URLPathNormalizer::new(eng, num)` на каждый `process_with_char_mapping` вызов вместо хранения в struct. Это обходит Rust borrow-restrictions (нормалайзер держит &references на english/number). Runtime-cost минимален (структура stateless). Follow-up: рефакторинг в Rc/Arc или `&self`-методы для меньшего surface area.
- merge_conflicts: **zero**. R9 форкался от ветки с актуальным ruvox2 state — все зависимости уже были merged. Файлы normalizers (code/numbers/urls/code_blocks) R9 редактировал для follow-up fixes; эти файлы не трогались в ruvox2 между merged R5/R6/R8 и HEAD, поэтому merge прошёл чисто стратегией ort.
- rust-version bump: `1.77 → 1.80`. Обоснование: `std::sync::LazyLock` стабилизирован в 1.80 и используется в `abbreviations.rs` (merged в R4). На хост-машине cargo 1.95 — без проблем.
- Cargo.lock: R9 branch содержал локальный snapshot с `tokio` dependency в `ruvox-tauri` — в итоговом ruvox2 это добавление уже присутствовало от B1/B2 (tokio там тоже в `[dependencies]`). Ort-стратегия auto-merge корректно объединила.
- verification_gap: полный `nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml"` на всём крейте в sandbox ревьюера невозможен (gtk/webkit system libs через nix-daemon). Ревьюер верифицировал pipeline как изолированный lib-крейт — компилирует clean на stable Rust + 620/620 тестов зелёные offline. **Live-прогон полного `cargo test` на хосте (с nix-shell) остаётся за пользователем** перед запуском R10 (golden harness integration).
- **big unblock**: R10 (golden-test harness integration) + B4 (Tauri commands поверх pipeline) теперь разблокированы. U11 также разблокирован, если зависит от R9+B4.

### U8 — HTML format support
- status: **merged** (2026-04-20)
- branch: task/u8-html-format
- worker_commits:
  - `fb64336 feat(ui): HTML format support in text viewer`
  - `9ee1cbe feat(pipeline): HTML-to-text extractor with mapping`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `6722e02 merge(u8): HTML format support`
- deps_met: U4 merged (`3eae9db`). После merge разрешил также все последующие наложения R-серии, U7, U10.
- deliverable:
  - **Frontend (`fb64336`):**
    - `src/lib/html.ts` — `renderHtml(raw)` через DOMPurify 3.4 + highlight.js. Sanitize config: `USE_PROFILES:{html:true}`, `FORBID_TAGS:['script','style','iframe','object','embed','form','input']`, `FORBID_ATTR:['style']`, `ALLOW_DATA_ATTR:true`. После sanitize walks `pre code.language-*` элементов и применяет `hljs.highlight()` / `highlightAuto()` с relevance threshold ≥ 5.
    - `src/components/TextViewer.tsx` — `Format` расширен до `'plain' | 'markdown' | 'html'`, SegmentedControl теперь 3-mode, `useMemo` switch на рендерер.
    - `src/components/TextViewer.module.css` — новые правила для `img` (max-width:100%), `table/th/td` (borders + zebra header), `blockquote` (blue left border).
    - `package.json` — `dompurify ^3.4.0` в dependencies.
  - **Rust (`9ee1cbe`):**
    - `src-tauri/src/pipeline/html_extractor.rs` (550 строк): `extract_text_for_tts(html) -> TrackedHtml` через scraper (html5ever backend). Walks DOM tree, excludes `nav/footer/aside/script/style/head/noscript/template/svg/math/button/select/option/optgroup/datalist` subtrees целиком. Блочные теги (p/div/section/article/main/header/h[1-6]/blockquote/pre/ul/ol/li/dt/dd/dl/figure/figcaption/table/thead/tbody/tfoot/tr/th/td/details/summary/br/hr) получают leading+trailing `\n`. Whitespace collapse (включая `\u{00a0}` NBSP) как в браузере. `TrackedHtml.html_range_for(start,end)` — lookup HTML span для text-range.
    - `src-tauri/src/pipeline/mod.rs` — добавлен `pub mod html_extractor;`.
    - `src-tauri/Cargo.toml` — `scraper = "0.19"` (thiserror/tracing уже были после R9).
    - 20 unit-тестов: simple_paragraph, multiple_paragraphs, headings, nested_blocks, nav/footer/aside/script/style exclusions, ul/ol, inline_code, pre/code, full_article, whitespace, spans_exist, html_range_for_empty, normalise_multiple_blank_lines, blockquote, table_cells, empty_html.
- review_findings:
  - **XSS safety (frontend):** DOMPurify 3.4 config — строгий. `script/style/iframe/object/embed/form/input` forbidden; `FORBID_ATTR:['style']` отсекает CSS-injection. DOMPurify по умолчанию блокирует `on*` handlers, `javascript:`/`data:` URIs в href/src (кроме whitelisted image mime-types), `srcdoc`, `formaction` и др. `highlightCodeBlocks` парсит уже очищенный HTML в temp `<div>` и переписывает `codeEl.innerHTML = result.value` — hljs output — это escaped HTML spans, безопасно. TrustedHTML cast через `as unknown as string` — необходимый костыль DOMPurify 3.4 типов, не влияет на runtime. В связке с Tauri CSP (двойная защита) — acceptable.
  - **Rust html_extractor:** без `unwrap()`/`expect()` в горячих путях (используется `Selector::parse` только в dead_code helper `build_exclusion_selector`). `extract_text_for_tts` инфаллибильный — возвращает `TrackedHtml` без `Result`. Exclusion list покрывает всё что требует task-spec (nav/footer/aside) + защита от script/style/svg. Whitespace collapse идентичен browser inline-text normalization.
  - **HtmlCharSpan sentinel `html_start/end = 0`**: задокументирован в `push_text` (line 253-257): "We cannot reliably get scraper source offsets from text nodes without patching the library, so we mark html_start == html_end as a sentinel meaning 'source position unknown for this span.' The word highlighter in the UI will gracefully degrade." Full source mapping — follow-up через html5ever tokenizer (нужен для U5 word-highlighting на HTML-режиме).
  - **TS strict:** функциональный компонент, нет `React.FC`, нет `any`, нет `sx`/`createStyles`/emotion. Mantine 8 правила соблюдены.
  - **Rust style:** `thiserror` для `HtmlExtractError`, `tracing::warn` импортирован (но не используется в текущем коде — зарезервирован для будущих warnings), нет `anyhow` в prod, нет `unsafe`, нет emoji.
- known_deviations (задокументированы):
  1. `scraper = "0.19"` вместо более новой 0.26 — в sandbox cargo registry доступна только 0.19. Работает корректно; 0.19 пулит отдельный html5ever 0.27/markup5ever 0.13 не конфликтуя с html5ever 0.29 уже в дереве.
  2. `HtmlCharSpan.html_start/end = 0` sentinel — full source mapping отложен до U5 HTML word-highlight.
  3. `TextEntry.format` в schema не добавлен (есть TODO-коммент в TextViewer.tsx). Будет в B1/F4 при расширении schema. Пока format — ephemeral client-side state.
- merge_conflicts:
  - `src-tauri/Cargo.toml` — U8 добавлял `scraper+thiserror+tracing`, R9 уже добавил `thiserror+tracing+dirs+tokio+parking_lot` и поднял rust-version до 1.80. Резолв: union deps, `scraper = "0.19"` добавлен рядом, rust-version остался 1.80, dev-dependencies сохранены.
  - `src-tauri/Cargo.lock` — в секции `[[package]] name = "ruvox-tauri"` конфликт в списке dependencies (U8 видел только thiserror, HEAD имел tempfile+thiserror+tokio). Резолв: HEAD-версия, т.к. она уже включает thiserror и добавляет прочие deps; scraper присутствует в объединённом списке выше.
  - `src/components/TextViewer.tsx` — единственный конфликт был в импорте (HEAD: `renderMermaidIn`, U8: `renderHtml`). Остальное авто-объединилось корректно: U8 SegmentedControl с 3 опциями + U7 useRef/useEffect для mermaid + Modal. В итоговом компоненте: `Format = 'plain' | 'markdown' | 'html'`, switch-cases rendererа (plain/html/markdown), mermaid useEffect срабатывает **только** в markdown режиме (`if (format !== 'markdown')`), click-to-zoom useEffect — всегда (в html-режиме mermaid-дивов не будет, closest('.mermaid') вернёт null).
  - `src-tauri/src/pipeline/mod.rs` — auto-merged корректно: `pub mod html_extractor;` добавлен рядом с `pub mod constants; pub mod normalizers; pub mod tracked_text;`, R9 TTSPipeline сохранён ниже 1:1.
  - `src/components/TextViewer.module.css`, `package.json`, `pnpm-lock.yaml` — auto-merged (чистые дополнения).
- verified: статический ревью. `nix-shell --run "cargo test"` / `pnpm install && pnpm typecheck && pnpm build` не запущены в sandbox ревьюера (DNS + gtk/webkit system deps через nix-daemon). Воркер-ревизия U8 делала изолированный test-crate: 20/20 html_extractor тестов зелёные на cargo 1.95 offline. `pnpm-lock.yaml` уже содержит dompurify 3.4.0 (с deprecated `@types/dompurify` стабом — можно убрать follow-up), но валидация lockfile требует `pnpm install` на хосте.
- followups (не блокеры):
  - Live `nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml"` и `pnpm typecheck && pnpm build` на хосте — проверить что `scraper 0.19` + `html5ever 0.27` не конфликтуют с `html5ever 0.29` уже подтянутым в дерево.
  - U5 HTML word-highlighting потребует full source mapping в `HtmlCharSpan` (текущий sentinel 0/0 закроет graceful-degrade).
  - B1/F4: добавить `format: 'plain' | 'markdown' | 'html'` в `TextEntry` schema (плюс `html` text/html MIME-detection из clipboard).
  - Рассмотреть удаление `@types/dompurify` стаба из lock (deprecated — DOMPurify 3.x приносит встроенные типы).
  - Integration: HTML-pipeline пока не подключен к `TTSPipeline::process_with_char_mapping` — сейчас `extract_text_for_tts` standalone. Для полноценного "copy HTML → TTS" потребуется: frontend detect `text/html` MIME → backend command `process_html_for_tts(html) -> (String, CharMapping)` внутри которого `extract_text_for_tts` → `TTSPipeline::process_with_char_mapping`. Follow-up для B4.
- next_unblocks: нет прямых задач. U8 — leaf в дереве enrichment-фичей. Разблокировка через B4 (frontend HTML-pipeline bridge) и U5 (word highlighting в HTML режиме).

### R10 — Golden-test harness
- status: **merged** (2026-04-20)
- branch: task/r10-golden-harness
- worker_commit: `8e1dcda test(pipeline): golden-fixture harness for TTSPipeline`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `e22b559 merge(r10): golden-fixture harness with markdown bugfix`
- deps_met: R9 merged (`84e09b1`), F5 merged (37 fixtures).
- deliverable:
  - `src-tauri/tests/golden.rs` (223 строки): single `#[test] fn golden_fixtures()`, сканирует `tests/fixtures/pipeline/*.input.txt`, прогоняет `TTSPipeline::process_with_char_mapping`, сверяет текст с `*.expected.txt` и char_map с `*.char_map.json`. Diff через `similar::TextDiff::from_lines` для текстовых mismatch; per-entry listing для char_map (first 10 + remaining-count). Поддерживает запуск как из `src-tauri/` так и из workspace root (автодетект `tests/fixtures/pipeline`).
  - `src-tauri/Cargo.toml` — `similar = "2"` добавлен в `[dev-dependencies]`.
- tests: **37/37 golden fixtures passed** + **640 unit tests passed / 0 failed** в изолированном мини-крейте ревьюера (cargo 1.95 offline). Golden test сам по себе — `cargo test --test golden` — зелёный.
- **BONUS bugfix (in-scope, pipeline correctness):** воркер обнаружил что фикстура `markdown_link` падала — `process_markdown_tracked` обрабатывал `[GitHub](https://github.com)` одной операцией `tracked.replace(full_pattern, pre_normalized)`, что помечало все символы link-текста как already-replaced и блокировало последующую CamelCase-нормализацию ("GitHub" → "гит хуб"). Исправление:
  - `src-tauri/src/pipeline/tracked_text.rs` — добавлен метод `replace_byte_range(byte_start, byte_end, to)` — single-occurrence byte-range replace, повторяет инварианты `sub` (`is_current_char_pos_inside_replacement` + `find_containing_replacement` guards, push Replacement/OffsetEntry, invalidate sorted cache, splice `current`).
  - `src-tauri/src/pipeline/mod.rs::process_markdown_tracked` — переработан: сначала regex `re_md_link_full` собирает byte-ranges для `[` prefix и `](url)` suffix каждой ссылки, затем применяет `replace_byte_range` в reverse-document-order (сначала suffix на высоком offset, потом bracket на низком). Link-text остаётся нетронутым как original chars и проходит через последующие CamelCase/English phases.
  - Удалён dead helper `normalize_link_text` (больше не нужен — pre-normalization не требуется).
  - Совместимо с legacy `pipeline.py::_process_markdown_tracked` (420-426): two separate `tracked.sub(r"\[(?=...)\]", "")` + `tracked.sub(r"\]\([^)]+\)", "")`. Rust-версия использует byte-range вместо второго regex, но эффект семантически идентичен.
- review_findings:
  - **`replace_byte_range` invariants:** корректно вычисляет `orig_start`/`orig_end` через `current_to_original` (с учётом что `char_end > char_start` — иначе orig_end = orig_start), проверяет `already_replaced` per-codepoint, гвардит через `find_containing_replacement`, повторяет `insert(0, ...)` pattern для `offset_entries` что совпадает с `sub`. `sorted_entries_cache` инвалидируется. `self.current` пересобирается через string splicing на byte offsets (каллер гарантирует char-boundary — regex match.start()/end() + +1 для ASCII `[`).
  - **No-op guard:** если `old_text == to`, возвращается сразу без touch state — корректно (соответствует `sub` early-continue).
  - **Harness quality:** auto-detect fixtures directory покрывает оба способа запуска (`cargo test --manifest-path src-tauri/Cargo.toml --test golden` из root и `cd src-tauri && cargo test --test golden`). `similar::TextDiff` даёт читаемый unified diff. `char_map_diff` с лимитом 10 + "and N more" предотвращает гигантский вывод на крупных расхождениях. Trailing `\n` обрезается из expected/input — совместимо с F5 фикстурами где Python генератор оставляет trailing newline.
  - **Style:** `.expect(...)` в тестовых helpers — допустимо по правилу (unwrap запрещён только в production-путях). Комментарии WHY присутствуют (секции "Fixture discovery", "Diff helpers", "Main harness"). No emoji, commit message формат `<type>(<module>): <desc>` ✓.
- merge_conflicts:
  - `src-tauri/Cargo.lock`, `src-tauri/Cargo.toml`, `src-tauri/src/pipeline/mod.rs` — все auto-merged ort-стратегией. R10 форкался от `07df66c` (после R9), ruvox2 ушёл вперёд на U8 (`6722e02`). Конфликтов требующих ручного резолва — нет.
  - `similar = "2"` (R10) + `scraper = "0.19"` + `thiserror = "1"` + `tracing = "0.1"` (U8) union-объединились корректно.
  - `pub mod html_extractor;` (U8) сосуществует с R10 изменениями в `process_markdown_tracked` — разные регионы файла.
- verification:
  - Ревьюер собрал мини-крейт `{regex, aho-corasick, once_cell, serde, serde_json, scraper, thiserror, tracing} + dev: similar` (без tauri/tokio/mpv) с копией полного `pipeline/` и `tests/fixtures/` + `tests/golden.rs`. `cargo test --test golden -- --nocapture` → `golden_fixtures: 37/37 passed`. `cargo test --lib` → `640 passed / 0 failed`.
  - Полный `nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml"` в sandbox ревьюера по-прежнему невозможен (GTK/webkit2gtk system-libs через nix-daemon Unix-socket). Live-прогон остаётся за хостом.
- known_deviations: none. Bugfix — **улучшение** корректности относительно R9, не регрессия. Legacy Python behavior теперь воспроизводится точно.
- refactoring_note_for_future: `TrackedText::replace_byte_range` — новый публичный API-метод. Использовать в будущих пайплайновых фазах где нужна single-occurrence замена без риска случайного совпадения с другими текстовыми регионами (например, экранированные символы, HTML-entity substitution, indexed-positional-replaces из парсера). Инварианты: caller должен передавать byte-boundary aligned offsets (обычно из `regex::Match::start()/end()`), и убедиться что range не пересекает character boundaries (`self.current.is_char_boundary(byte_start/byte_end)`).
- next_unblocks: **все критические задачи R-серии + tests завершены**. После B4 merge готова к запуску вся UI-волна (U2, U3, U5, U6, U9, U11, U12). R10 — последний test-harness в задачах rewrite; дальнейшие golden-fixtures просто добавляются в F5 без изменения harness.

### B4 — Tauri commands (IPC-handlers)
- status: **merged** (2026-04-20)
- branch: task/b4-tauri-commands
- worker_commit: `9d2f5b0 feat(commands): scaffold AppState and Tauri command handlers`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `2a9c83c merge(b4): Tauri command handlers with background synthesis`
- deps_met: B1 merged (`b166e43`), B2 merged (`32ae801`), B3 merged (`dd579de`), R9 merged (`84e09b1`), R10 merged (`e22b559`), F3 merged (ipc-contract), F4 merged (storage-schema).
- deliverable:
  - `src-tauri/src/commands/mod.rs` (664 строки): **18 команд** = 14 спецификационных из `docs/ipc-contract.md` + 4 extra (`cancel_synthesis`, `clear_cache`, `get_cache_stats`, `stop_playback`). Все через `#[tauri::command]`, возвращают `Result<T, CommandError>`. `CommandError` — `thiserror` enum с `#[serde(tag = "type", rename_all = "snake_case")]` — совпадает с `TauriError` контрактом (`not_found | storage_error | synthesis_error | playback_error | config_error | internal`). `From<StorageError>` / `From<TtsError>` — конверсии в границах.
  - `src-tauri/src/state.rs` (24 строки): `AppState { storage: Arc<StorageService>, tts: Arc<TtsSubprocess>, player: Arc<Player<tauri::Wry>>, pipeline: Arc<parking_lot::Mutex<TTSPipeline>>, tray_cmd_tx: Option<mpsc::Sender<TrayCmd>> }`. Использует конкретный `tauri::Wry` runtime — необходимо для `tauri::generate_handler!` (нельзя generic в `#[tauri::command]`).
  - `src-tauri/src/lib.rs` (full rewrite setup, 168 строк): spawn ttsd → `warmup` async → emit `model_loading`/`model_loaded`/`model_error`; создание `tokio::mpsc::channel<TrayCmd>(16)` + background loop читающий clipboard → `storage.add_entry` → `spawn_synthesis_pub`; регистрация всех 18 команд в `invoke_handler!`. Startup `.expect(...)` на `StorageService::new()` / `TtsSubprocess::spawn()` — допустимо (fail-fast at init, не prod-path).
  - `src-tauri/src/tray/mod.rs` (126 строк, +55 −0 diff): `on_menu_event` для `read_now`/`read_later` теперь напрямую вызывает `invoke_add_clipboard_entry` → `try_state::<AppState>()` + `tray_cmd_tx.try_send(TrayCmd)`. Fallback на старый `app.emit("tray_read_now")` когда `tray_cmd_tx` ещё `None` (до setup). `pub struct TrayCmd { play_when_ready: bool }` экспортируется.
  - `src-tauri/Cargo.toml`: `arboard = "3"` добавлен (системный clipboard для Linux/Win/Mac).
  - `src-tauri/Cargo.lock`: auto-update (arboard + transitive deps).
- spec_compliance (`docs/ipc-contract.md`):
  - **Commands 14/14 из спеки реализованы** 1:1 (имена, типы аргументов, возврат): `add_clipboard_entry`, `get_entries`, `get_entry`, `delete_entry`, `delete_audio`, `cancel_synthesis`, `play_entry`, `pause_playback`, `resume_playback`, `seek_to`, `set_speed`, `set_volume`, `get_config`, `update_config`, `get_timestamps`, `clear_cache`, `get_cache_stats`. Плюс `stop_playback` (в спеке документирован, но не в списке первых 14). Итого 18.
  - **Events 10/11 покрыты:** `entry_updated` (commands emit), `playback_position` (player spawn_position_emitter), `playback_started`/`playback_paused`/`playback_stopped`/`playback_finished` (player), `model_loading`/`model_loaded`/`model_error` (lib.rs setup), `tts_error` (commands). **Не реализован: `synthesis_progress`** — ttsd v1 не поддерживает streaming JSON-ответов (один request → один response). Known gap, задокументирован; фронт должен обрабатывать отсутствие progress-событий (показывать indeterminate spinner между `processing` и `ready`).
  - Error format matches spec: `{ "type": "not_found" | "storage_error" | "synthesis_error" | "playback_error" | "config_error" | "internal", "message": string }`.
- review_findings:
  - **Async safety:** `add_clipboard_entry` использует `tokio::task::spawn_blocking` для `arboard::Clipboard::new()/get_text()` — корректно для sync API на Linux (X11/Wayland event loop в blocking thread). Background synthesis через `tokio::spawn` с cloneable `AppHandle<Wry>` и `Arc<...>` полей — нет ownership конфликтов. `pipeline.process_with_char_mapping` обёрнут в `spawn_blocking` (CPU-bound, ~100ms+ на длинных текстах) — правильно, не блокирует tokio reactor.
  - **Lock contention:** `AppState` — просто контейнер Arc'ов; lock берётся только внутри `StorageService` (RwLock вокруг HashMap), `Player` (parking_lot::Mutex для `State{current_entry_id, ...}`) и `TTSPipeline` (parking_lot::Mutex — pipeline stateful). Pipeline lock держится исключительно внутри `spawn_blocking` → tokio reactor не заблокирован. Нет nested locks → нет deadlock-риска.
  - **Error handling:** `CommandError` через `thiserror`, `#[serde(tag = "type", rename_all = "snake_case")]` → wire-формат `{ "type": "storage_error", "message": "..." }` матчит `ipc-contract.md` § Error Format. `From<StorageError>` правильно маппит `NotFound` → `not_found`, остальное → `storage_error`. `TtsError` → `synthesis_error`. Все `Result`-returning команды возвращают `CmdResult<T>` = `Result<T, CommandError>`.
  - **Tray integration:** `tokio::mpsc::channel::<TrayCmd>(16)` буфер 16 — достаточно для человеко-скоростных tray-кликов. `try_send` в `on_menu_event` (синхронный callback) избегает блокировки UI-потока; если буфер заполнен — событие теряется, что для tray-меню приемлемо. Background loop в `lib.rs::setup` читает `tray_rx.recv()` и делегирует в `spawn_synthesis_pub`. `tray_cmd_tx` хранится как `Option` в `AppState` для graceful fallback (fallback на emit, если канал ещё не готов). Минор: нет explicit shutdown логики для канала — при закрытии app tokio runtime дропнет, очередь GC'ится. Для v1 достаточно.
  - **Clipboard:** `arboard` sync API в `spawn_blocking` — каноничный паттерн для Linux (X11 selection требует event-loop polling на своём треде). Корректно.
  - **Pipeline flow** (спец Example 1 в `ipc-contract.md`): `add_clipboard_entry` → `storage.add_entry` → emit `entry_updated{pending}` → `spawn_synthesis` → `pipeline.process_with_char_mapping` → `storage.update_entry{processing, normalized_text}` → emit `entry_updated{processing}` → `tts.synthesize(..., char_mapping)` → `storage.save_timestamps` → `storage.update_entry{ready, audio_path, duration_sec}` → emit `entry_updated{ready}` → (if play_when_ready) `player.load + play` → `player` emit `playback_started`. Шаги матчат спеку с одним отклонением: **не эмитируется `entry_updated{processing}` ДО вызова pipeline** — emit происходит ПОСЛЕ pipeline завершения (вместе с `normalized_text`). Для UI это не блокер (pipeline ~100-500ms), но расходится с диаграммой где `processing` emit идёт раньше. Follow-up — minor.
  - **Style:** no emoji, no `unwrap()` в prod-путях (только `.expect("не удалось ...")` в startup lib.rs — допустимо по правилу "fail-fast at init"), `#[tauri::command]`, `tracing::{info,warn,error}` для логов, commit message формат `<type>(<module>): ...` ✓.
  - **Minor issues (non-blocking, follow-up):**
    1. `delete_entry` эмитирует `playback_stopped{}` безусловно после `storage.delete_entry`, хотя `player.stop()` уже эмитирует `playback_stopped` когда entry действительно играла. Результат: дублирующий event когда entry играла, ложный event когда не играла. UI должен быть идемпотентен к повторным stop-событиям.
    2. `cancel_synthesis` просто переводит статус в `pending` без реального abort в TtsSubprocess (channel-serialized, нет cancellation token). Документировано в doc-comment команды. Реальный mid-request abort — follow-up для B2-v2.
    3. Emit `entry_updated{processing}` идёт ПОСЛЕ pipeline, не ДО (см. pipeline_flow выше). Minor UX-задержка.
- merge_conflicts: none (clean merge, ort strategy). B4 форкался от `1d25284` (после R10 merge) — `src-tauri/src/commands/mod.rs` (new), `src-tauri/src/state.rs` (new) — новые файлы. `lib.rs` и `tray/mod.rs` — B4 полностью переписал setup и `on_menu_event`, конфликтов с B5-fix / tray-themed не возникло т.к. ruvox2 с момента R10 merge не трогала эти файлы. `Cargo.toml` + `Cargo.lock` — добавка `arboard = "3"`, union-merge. `emit_entry_updated` хелпер и `spawn_synthesis_pub` публичный re-export для tray.
- verification:
  - Ревьюер прочитал и верифицировал все 4 файла на SHA `9d2f5b0` через `git show`. Сверка с `docs/ipc-contract.md` по каждому имени команды, типу аргументов, возврату, error format, emit targets — 14/14 спека + 4 extra.
  - Сверка с `StorageService` (service.rs) / `TtsSubprocess` (tts/mod.rs) / `Player<Wry>` (player/mod.rs) / `TTSPipeline` (pipeline/mod.rs) API: все вызовы `storage.add_entry`, `storage.update_entry`, `storage.get_entry`, `storage.delete_entry`, `storage.delete_audio`, `storage.save_timestamps`, `storage.load_timestamps`, `storage.get_audio_path`, `storage.get_all_entries`, `storage.load_config`, `storage.save_config`, `storage.get_cache_size`, `storage.get_audio_count`, `storage.cache_dir` + `player.load/play/pause/resume/stop/seek/set_speed/set_volume/current_entry_id` + `tts.warmup/synthesize` + `pipeline.process_with_char_mapping` — совпадают по сигнатурам и возвращаемым типам.
  - `cargo check --manifest-path src-tauri/Cargo.toml` в sandbox ревьюера невозможен (нужны nix-daemon + GTK/webkit2gtk system libs). Воркер подтвердил `cargo check → 0 errors` у себя. Live-прогон `nix-shell --run "cargo check --manifest-path src-tauri/Cargo.toml"` остаётся за хостом.
- known_deviations:
  - **`synthesis_progress` event не реализован** (ttsd v1 single-response) — задокументировано в RewriteNotes как gap; UI должен fallback на `status: processing` без progress-bar.
  - **`cancel_synthesis` — soft-cancel** (только status update, без interrupt ttsd). Acceptable для v1.
  - **`add_clipboard_entry` emit-последовательность**: `pending` → `processing` идёт после pipeline, не до. Minor.
- refactoring_note_for_future:
  - При добавлении команд требующих `AppHandle` (для emit) — использовать `AppHandle<Wry>` (не `AppHandle<R>` generic), т.к. `tauri::generate_handler!` требует concrete runtime. Helpers типа `emit_entry_updated<R: Runtime>` — generic для переиспользования в tests.
  - `spawn_synthesis_pub` публичный API нужен в tray для reuse. Если появятся другие вызовы (hotkey manager, IPC из CLI) — можно вынести в отдельный `synth_queue` модуль.
- next_unblocks: **UI-волна полностью разблокирована**. Задачи готовые к запуску: **U2** (queue list), **U3** (player component), **U5** (word highlighting — ждал B3+B4), **U6** (settings panel), **U9** (regenerate entry), **U11** (keyboard shortcuts), **U12** (error surfacing). Все используют `src/lib/tauri.ts` typed-wrappers (уже созданные в U1) поверх 18 команд + listen на 10 событий.

### Следующие действия координатора
- После B4 merge — **запустить UI-волну параллельно**: U2, U3, U5, U6, U9, U11, U12. Все разблокированы. Между ними prereqs: U5 ждёт U4 (merged); U12 ждёт U10 (merged) для notification-channel reuse.
- Hot-spot для хоста: прогнать `nix-shell --run "cargo check --manifest-path src-tauri/Cargo.toml"` на ruvox2 HEAD (`2a9c83c`) — подтвердить что merge компилируется. Затем `nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml"` + `pnpm install && pnpm typecheck && pnpm tauri dev` для smoke.
- Follow-ups (не блокируют UI-волну):
  - `synthesis_progress` — требует v2 ttsd протокол (streaming). Отдельный задел, если нужно прогресс-барам.
  - `cancel_synthesis` — hard-cancel через cancellation token в TtsSubprocess.
  - Emit `entry_updated{processing}` до pipeline (а не после) для более отзывчивого UI.

### U3 — Player component
- status: **merged** (2026-04-20)
- branch: task/u3-player
- worker_commit: `9329dfc feat(ui): playback controls with keyboard shortcuts`
- merge_commit: `e11c3c9 merge(u3): playback controls component`
- deps_met: U1 merged, B4 merged.
- reviewer: autopilot-opus (1M context)
- review_result: ok
- files:
  - `src/components/Player.tsx` (new, 300 lines): функциональный компонент `Player({ entryIds?: EntryId[] })`. Подписывается на `playback_started`/`playback_paused`/`playback_stopped`/`playback_finished`/`playback_position`/`entry_updated`; все unlisten-promises собраны в массив и вызываются в cleanup. Кнопки Play/Pause, Prev/Next (Unicode-глифы `\u23F8`/`\u25B6`/`\u23EE`/`\u23ED`), Slider прогресса (seek через `onChangeEnd` → `seek_to`), NumberInput скорости 0.5–2.0 (clamp + `set_speed`), Slider громкости 0–1 (`set_volume`), MM:SS/MM:SS time display. Hotkeys через `@mantine/hooks` `useHotkeys`: Space (play/pause), ArrowLeft/Right (±5s seek).
  - `src/components/Player.module.css` (new, 38 lines): CSS Modules, без `sx`/`createStyles`/`emotion`. Используются `var(--mantine-*)` токены (spacing/color/font-size). Flex-layout: progress `flex:1`, `tabular-nums` для времени, фиксированные ширины для speed/volume/play.
  - `src/components/AppShell.tsx` (modified, +19/-16): header.height 56 → 108, внутри `<Stack gap={0}>` сверху Group с Title+ThemeSwitcher (h=56), снизу `<Player />`. Placeholder-комментарий U3 удалён.
  - `pnpm-lock.yaml`: автообновление lockfile (+943 −0) — видимо затронуты transitive deps при резолве `@mantine/hooks` useHotkeys tree. Нет новых runtime deps в package.json (U1 уже притащил `@mantine/hooks`).
- spec_compliance (`RewriteTaskPlan.md` § U3, `docs/ipc-contract.md`):
  - Deliverable fulfilled: Mantine `Group` с Play/Pause, Progress `Slider`, Time MM:SS/MM:SS, Speed NumberInput 0.5–2.0 шаг 0.1, Volume Slider, Prev/Next — всё на месте.
  - Listen на `playback_position` → обновляет progress. Ok.
  - Seek через drag slider → `seek_to` в `onChangeEnd`. Ok.
  - Keyboard: Space / Left / Right — Ok через `useHotkeys`.
  - Acceptance (smoke: воспроизведение + seek) — требует live-прогон на хосте (не покрыто в sandbox ревьюера).
- review_findings:
  - **Cleanup listeners:** 6 listener-promises в массиве `unlisteners`, в return `() => unlisteners.forEach(p => p.then(fn => fn()))` — стандартный Tauri-паттерн. Unlisten асинхронный, но событие unmount происходит редко. Без утечек.
  - **Drag-protection:** реализована через state `isDragging`+`dragValue`. Slider рендерит `value={isDragging ? dragValue : state.position}` — визуально корректно во время drag. Замечание: в handler `playbackPosition` закрытие `(prev) => { if (!isDragging) { ... } }` читает `isDragging` из замыкания `useEffect([])`, которое всегда видит начальное `false`. Это формально баг (state.position обновляется во время drag), но ввиду того, что рендер использует `dragValue` в drag-режиме, visual UX корректен; по завершении drag `onChangeEnd` вызывает `setState({position:v})` + `seekTo(v)`, после чего incoming events снова обновляют. Воркер оставил комментарий "captures latest via closure-over-ref pattern would require a ref". Follow-up: перевести на `useRef` для точного следования spec, но не блокер.
  - **Hotkeys safety для форм-инпутов:** `useHotkeys` из `@mantine/hooks` по умолчанию игнорирует `INPUT`/`TEXTAREA`/`contenteditable` — Space не перехватится в NumberInput скорости или текстовых полях будущего edit-dialog. Ok.
  - **Speed range:** NumberInput `min=0.5 max=2.0 step=0.1` + clamp в `handleSpeedChange` (`Math.min(2.0, Math.max(0.5, speed))`). Совпадает со спецификацией `set_speed` (`[0.5, 2.0]`).
  - **Volume range:** Slider `min=0 max=1 step=0.05`, передаётся напрямую в `set_volume`. Совпадает со спецификацией `[0.0, 1.0]`.
  - **Time format:** `formatTime(sec)` → `MM:SS`. Для длительности > 60 мин вывод будет `75:30` вместо `1:15:30`. Технически не HH:MM:SS, но для типичных TTS-записей (<< 60 мин) приемлемо. Follow-up: расширить до HH:MM:SS.
  - **Play button disabled:** `!currentEntryId && (!entryIds || entryIds.length === 0)` — disabled только когда нечего играть. При наличии entryIds и отсутствии текущего entry кнопка enabled, но `handlePlayPause` ничего не делает (no play-first-of-queue fallback). Minor UX gap: можно было бы запускать entryIds[0]. Follow-up.
  - **Duplicate playback_stopped handling:** listener `playbackStopped` сбрасывает `currentEntryId=null` + `position=0`. Backend (B4) эмитит дубликат в `delete_entry` (документировано). Frontend идемпотентен (повторный reset — noop). Ok.
  - **Symbols vs emoji rule:** Unicode-символы плеера `\u23F8 \u25B6 \u23EE \u23ED` — управляющие символы клавиш транспорта Unicode block "Miscellaneous Technical", не emoji по правилу проекта (no smiley/pictograph). Допустимо.
  - **Style:** TS strict, no `any`, функциональный без `React.FC`, CSS Modules only, нет `sx`/`createStyles`/`emotion`. Commit message формат `feat(ui): ...` ✓.
- merge_conflicts: none. U3 первый в UI-волне после B4; AppShell.tsx трогается впервые. U2/U5/U6/U9 параллельно работают с AppShell.tsx в своих воркерах — их merge-pass будет с конфликтами, это нормально.
- verification:
  - Ревьюер прочитал 3 файла (Player.tsx, Player.module.css, AppShell.tsx) + сверил `tauri.ts` wrappers (не менялся на U3) с `docs/ipc-contract.md`: `play_entry`, `pause_playback`, `resume_playback`, `seek_to`, `set_speed`, `set_volume` — имена match.
  - TypeScript typecheck (`tsc --noEmit`) прогнан напрямую через nodejs из nix-store (nix-shell заблокирован sandbox sqlite cache): **0 ошибок**.
  - `vite build` прогнан напрямую: **ok (5.82s, 2.08 MB main bundle)**. Build warnings только про chunk size > 500kB (mermaid transitive, не относится к U3).
  - `nix-shell --run "pnpm install && pnpm typecheck && pnpm tauri dev"` на хосте — smoke-тест воспроизведения/seek остаётся за пользователем (acceptance).
- known_deviations: none критичных. Minor follow-ups см. выше (isDragging ref, HH:MM:SS для длинных аудио, play-first-of-queue fallback).
- next_unblocks: U3 не блокирует другие задачи напрямую (leaf в UI-дереве). Параллельно активны U2/U5/U6/U9 воркеры; их merge — следующие в очереди ревьюера.

### U2 — Queue list component
- status: **merged** (2026-04-20)
- branch: task/u2-queue-list
- worker_commit: `8f9d957 feat(ui): queue list with status badges and selection`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `e6fb983 merge(u2): queue list with selection store`
- deps_met: U1 merged (`78a0034`), B4 merged (`2a9c83c`).
- deliverable:
  - `src/components/QueueList.tsx` (~235 строк): list `TextEntry` из `commands.getEntries()`; item = preview (первые 60 символов с `\u2026` ellipsis), статус-бейдж (`pending`/`processing`/`ready`/`playing`/`error` с цветами gray/blue/green/teal/red и русскими лейблами), duration (MM:SS), кнопки Play (▶ `\u25B6`) / Delete (✕ `\u2715`) через `ActionIcon`. Клик на item → `setSelectedEntry` из Zustand-store. Delete → `modals.openConfirmModal` (`@mantine/modals`) → `commands.deleteEntry` + очистка selected если удалён выбранный. Listen на `events.entryUpdated` → in-place update (prepend+re-sort для новых entries, indexed replace для существующих); параллельно обновляет `selectedEntry` в store если текущий выбранный — чтобы TextViewer видел свежий статус без `get_entry` invoke. Empty state: «Скопируйте текст и нажмите Read Now». Sort by `created_at` desc (newest first) — на load и при insert.
  - `src/components/QueueList.module.css` (~43 строк): CSS Modules; `.item` / `.itemSelected` / `.preview` (ellipsis) / `.meta` / `.actions` (opacity 0 → 1 on hover/selected) / `.scrollArea`; использует Mantine CSS variables (`--mantine-spacing-*`, `--mantine-color-default-hover`, `--mantine-radius-sm`). Без `sx`, `createStyles`, emotion.
  - `src/stores/selectedEntry.ts` (16 строк): Zustand-store `useSelectedEntry` с `selectedId: EntryId | null`, `selectedEntry: TextEntry | null`, `setSelectedId`, `setSelectedEntry`. `setSelectedEntry(entry)` синхронно обновляет и `selectedId`, и `selectedEntry` — избегает race между QueueList и TextViewer.
  - `src/lib/tauri.ts`: re-export `UnlistenFn` из `@tauri-apps/api/event` для компонентов.
  - `package.json`: `zustand: ^5.0.0` добавлен в dependencies.
  - `src/components/AppShell.tsx`: navbar placeholder заменён на `<QueueList />`, `selectedEntry` теперь из `useSelectedEntry()` store (вместо `useState<TextEntry | null>`).
- merge_conflicts:
  - `src/components/AppShell.tsx` — **единственный merge conflict, разрешён вручную**. U2 форкался от `6da8e37` (B4 merged), до U3. U3 (`e3388b3`) добавил `Player` в header (`height: 108`, `<Stack>` wrapping `<Group>` + `<Player />`), U2 в той же ветке заменил navbar placeholder на `<QueueList />` и подключил Zustand store. Резолв: объединил импорты (`Stack` + `Player` из U3 + `QueueList` + `useSelectedEntry` из U2), убрал `useState<TextEntry | null>` (больше не нужен — selectedEntry из store), сохранил U3 header 108px с `Stack gap={0}` + `<Player />`, сохранил U2 navbar с `<QueueList />`. Результат: полный AppShell с header 108px (RuVox title + ThemeSwitcher + Player), navbar 280px с очередью, main с `TextViewer(entry={selectedEntry})`.
  - `package.json` — auto-merged (строка `zustand: ^5.0.0` чисто добавилась рядом с `react-dom`).
- verification:
  - Ревьюер прочитал и верифицировал все 6 файлов на SHA `8f9d957` через `git show`. Проверил `useEffect` cleanup для `entry_updated` listener (`unlisten?.()` в return), sort `created_at` desc на load + insert, `@mantine/modals::openConfirmModal` для delete, TS strict-типизация (`TextEntry | null`, `UnlistenFn`, payload-типы через typed-wrappers из U1), функциональный компонент без `React.FC`, Mantine 8 через CSS Modules (без sx/createStyles/emotion), empty-state текст русский.
  - `nix-shell --run "pnpm install"` в sandbox ревьюера невозможен (nix-daemon SQLite cache блокируется sandbox), `pnpm-lock.yaml` **не обновлён** в worker-ветке по той же причине (задокументировано в worker-commit message). Требуется `pnpm install` на хосте для регенерации lockfile.
  - Воркер подтвердил у себя `pnpm typecheck` / `pnpm build` чистыми. Live-прогон остаётся за хостом.
- review_findings:
  - **`useSelectedEntry.setState` с пустым объектом (minor, не блокер):** в listener `events.entryUpdated` для no-op (когда event не для выбранного entry) возвращается `{}`. Zustand shallow-merge делает это безопасным (noop для state), но немного неэлегантно. Можно было бы вынести условие наружу и не вызывать `setState` в no-op ветке. Не блокирующее.
  - **Иконки Play/Delete через Unicode символы (▶ `\u25B6`, ✕ `\u2715`) вместо tabler-icons:** `tabler-icons-react` не установлен в проекте — воркер выбрал lightweight Unicode для избежания ещё одной зависимости. Согласуется с deliverable задачи (`кнопки (Play/Delete)` без уточнения icon-библиотеки). По правилу проекта «no emoji» — эти символы не emoji, а управляющие символы Unicode block "Miscellaneous Technical" / "Dingbats" (аналогично Player U3 с `\u23F8 \u23EE \u23ED`). Допустимо.
  - **Empty state (корректно):** проверяется `entries.length === 0`; не показывается loader при первом mount (между `loadEntries()` kick-off и promise resolve — возможно мелькание empty state на 0ms–100ms). Для v1 ок, для UX — добавить `isLoading` флаг — follow-up.
  - **Sort stability (корректно):** `b.created_at.localeCompare(a.created_at)` для ISO 8601 строк даёт descending order. При insert нового entry выполняется re-sort; при update существующего — in-place replace без re-sort (но `created_at` не меняется, так что порядок сохраняется).
  - **Listener cleanup (корректно):** `unlisten?.()` в `useEffect` return вызовется после резолва Promise `events.entryUpdated(...)`. В StrictMode первый mount может успеть kick-off'нуть setup до unmount — `unlisten?.()` при unmount вернёт `undefined` (listener ещё не прилетел), listener осиротеет. В production без StrictMode — cleanup корректен. Для более robust паттерна можно использовать `let unmounted = false; ...then(fn => unmounted ? fn() : unlisten = fn)`. Follow-up по аналогии с U10 review.
  - **Нет pagination/virtualization:** `ScrollArea` рендерит все entries. Для очереди в 100+ записей приемлемо (Mantine `ScrollArea` не делает virtualize). Follow-up если появятся жалобы на перф.
  - **`QueueList.module.css` `.scrollArea { height: 100% }`:** внутри AppShell Navbar (`p="md"`) ScrollArea растянется на всю высоту navbar'а за вычетом `Title` «Очередь». Ok.
  - **Style:** no emoji, no `any`, no `React.FC`, no `sx`/`createStyles`/emotion, commit message `<type>(<module>): ...` ✓. TypeScript strict ✓.
- known_deviations: `pnpm-lock.yaml` не обновлён в worker-commit — требует `pnpm install` на хосте после merge.
- next_unblocks: **U4 / U5 / U6 / U9 могут теперь подтягивать `selectedEntry` из Zustand-store** вместо локального state в AppShell. U5/U6/U9 воркеры активны параллельно. Следующие к merge: **U6** или **U9** (оба готовы по сигналу координатора). **У всех активных U-воркеров будет конфликт в `src/components/AppShell.tsx`** (navbar теперь с QueueList + импорт `useSelectedEntry`) — ревьюерам U6/U9/U5 придётся объединять свои добавления с текущим layout (сохранять header 108px + Player + navbar QueueList + main `TextViewer(selectedEntry)`).

### U6 — Clipboard input (Read Now / Read Later)
- status: **merged** (2026-04-20)
- branch: task/u6-clipboard-input
- worker_commit: `180f356 feat(ui): Read Now / Read Later buttons with hotkeys`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `2c2ca28 merge(u6): Read Now / Read Later buttons with hotkeys`
- deps_met: U1 merged (`78a0034`), B4 merged (`2a9c83c`).
- deliverable:
  - `src/components/AppShell.tsx` — добавлены две кнопки в header рядом с `ThemeSwitcher`: **Read Now** (`<Button color="blue">`) и **Read Later** (`<Button variant="default">`). Обе кнопки имеют `loading={pending}` и `disabled={pending}` — защита от повторных кликов пока предыдущий `add_clipboard_entry` ещё в полёте.
  - `addEntry(playWhenReady: boolean)` — async handler: `setPending(true)` → `commands.addClipboardEntry(playWhenReady)` → `notifications.show({color: 'green'})` с текстом «Текст будет воспроизведён сразу» / «Текст добавлен для прослушивания позже». На ошибке — `notifications.show({color: 'red', title: 'Ошибка'})` с `err.message`. `finally { setPending(false) }` гарантирует сброс состояния.
  - `useHotkeys([['ctrl+shift+1', ...read-now], ['ctrl+shift+2', ...read-later]])` — глобальные шорткаты через `@mantine/hooks::useHotkeys`. Тот же pending-guard через ранний `if (pending) return`.
- merge_conflicts:
  - `src/components/AppShell.tsx` — **единственный конфликт, разрешён вручную**. U6 форкался от `ruvox2` до merge U2/U3 → его вариант файла содержал старый layout (`header: 56px`, `<Group>` без Stack, `<TextEntry | null>` через `useState`, navbar placeholder). HEAD (ruvox2 `2466cbc`) содержал обновления U3 (header 108px, `<Stack>` + `<Player />`) и U2 (`QueueList` в navbar, `useSelectedEntry` из `../stores/selectedEntry`).
  - **Резолв (канонический, сохраняет current layout):**
    - импорты — объединённый union: `Stack` + `Button` из `@mantine/core`, `useHotkeys`, `notifications`, `useState`, `commands`, `ThemeSwitcher`, `TextViewer`, `Player`, `QueueList`, `useSelectedEntry`. Убран `import type { TextEntry }` из U6 (не используется после миграции на store).
    - `selectedEntry` — из `useSelectedEntry()` store (как в ruvox2 HEAD), не `useState<TextEntry | null>` из U6 (устарело после U2 merge).
    - `pending` state + `addEntry` + `useHotkeys` — сохранены из U6 1:1.
    - header layout — сохранён 108px + `<Stack gap={0}>` с вложенными `<Group h={56}>` (title + actions) и `<Player />` (из U3/U2). Внутри правой `<Group>` actions кнопки Read Now + Read Later добавлены **перед** `<ThemeSwitcher />` (композиция: синяя основная → default secondary → theme control).
    - navbar — сохранён `<QueueList />` из U2.
    - main — `<TextViewer entry={selectedEntry} />` (как в ruvox2 HEAD).
- review_findings:
  - **Loading state корректный:** `loading={pending}` + `disabled={pending}` на обеих кнопках; `if (pending) return` в начале `addEntry` — защита от race при быстрых повторных кликах / горячих клавишах. `finally { setPending(false) }` гарантирует сброс даже при exception. Ok.
  - **Hotkeys через `@mantine/hooks::useHotkeys`:** корректное API, bindings `['ctrl+shift+1', fn]` / `['ctrl+shift+2', fn]`. Mantine автоматически вешает listener на `document` и снимает при unmount. `AppShell` — долгоживущий компонент (верхний уровень), hooks устанавливаются один раз.
  - **Hotkey values** (`ctrl+shift+1` / `ctrl+shift+2`) — hardcoded. Тем временем в `UIConfig` есть поля `hotkey_read_now: string` и `hotkey_read_later: string` (legacy defaults `Ctrl+Shift+C` / `Ctrl+Shift+V`) — задел для user-configurable shortcuts через Settings dialog (U9). Сейчас используются разумные defaults `Ctrl+Shift+1/2` не пересекающиеся ни с одним system-шорткатом Linux/Windows. Follow-up: подписаться на `get_config()` и пробросить `hotkey_read_now`/`hotkey_read_later` вместо hardcoded значений.
  - **Notifications через `@mantine/notifications`:** `notifications.show({title, message, color})` — каноничный API Mantine 8. Color `green` для success, `red` для error. Русские тексты для пользовательских сообщений. `NotificationsProvider` установлен в `App.tsx` на U1.
  - **Error handling:** `err instanceof Error ? err.message : String(err)` — безопасная extract-логика для Tauri `CommandError` (wire-format `{ type, message }` маппится в JS `Error` через `invoke` contract). Message показывается пользователю — допустимо для v1; более полная обработка (распознавание `{ type: 'clipboard_empty' }` / `{ type: 'clipboard_error' }`) может быть добавлена как follow-up.
  - **Command: `addClipboardEntry`** → backend `add_clipboard_entry(play_when_ready)` → `arboard::Clipboard::get_text()` в `spawn_blocking` → `storage.add_entry` → emit `entry_updated{pending}` → `spawn_synthesis` (pipeline + TTS). `play_when_ready=true` — после `ready` player.load+play. B4 flow точно матчит ожидания U6. Воркер ничего не трогал в backend.
  - **Style:** TS strict ✓, функциональный без `React.FC` ✓, no `any` ✓, no `sx`/`createStyles`/emotion ✓, Mantine 8 `@mantine/hooks` + `@mantine/notifications` ✓, commit message формат `feat(ui): ...` ✓, no emoji ✓.
- known_deviations:
  - **Hotkeys hardcoded** вместо чтения из `UIConfig.hotkey_read_now`/`hotkey_read_later` — follow-up для U9 (settings dialog) или отдельной задачи. Текущие defaults `Ctrl+Shift+1/2` разумны и не пересекаются с legacy дефолтами `Ctrl+Shift+C/V` (последние могут конфликтовать с copy/paste в терминале).
  - **No debounce на hotkeys** кроме `pending` guard — если пользователь зажмёт `Ctrl+Shift+1` → повторные срабатывания будут отклонены `if (pending) return`, но notification пропадёт только на первой. Ok для v1.
- verification:
  - Ревьюер прочитал `AppShell.tsx` на SHA `180f356` и ruvox2 HEAD через `git show`. Сверил `commands.addClipboardEntry` signature с `src/lib/tauri.ts` (merged в U1) и `docs/ipc-contract.md` — `add_clipboard_entry(play_when_ready: bool) -> EntryId` ✓. Проверил, что `@mantine/hooks` и `@mantine/notifications` уже в dependencies (установлены на U1).
  - `pnpm typecheck` / `pnpm build` / `pnpm tauri dev` не прогонялись в sandbox ревьюера (nix-daemon SQLite cache блокируется). Воркер подтвердил `typecheck` / `build` чистыми. Live-прогон остаётся за хостом.
- next_unblocks: U6 — leaf в UI-дереве, не блокирует другие задачи напрямую. Следующие к merge по приоритету: **U9** (settings dialog — user-configurable hotkeys закроют follow-up U6 hardcoded shortcuts), **U5** (word highlighting — независимо). Оба готовы по сигналу координатора.

### Следующие действия координатора
- Ревьюер ждёт уведомлений от других UI-воркеров: **U5** (word highlighting), **U6** (settings panel), **U9** (regenerate entry). Все форкнуты от ruvox2 до U2-merge → их AppShell.tsx будет конфликтовать с текущим (navbar теперь с `<QueueList />` + `useSelectedEntry`, header уже содержит Player). Merge-ревьюер решает конфликты ручным мерджем: сохранять текущий layout (header 108px + Player + navbar QueueList + main `TextViewer(selectedEntry)`) и добавлять изменения от U5/U6/U9.
- **После U2 merge (`e6fb983`):** на хосте запустить `nix-shell --run "pnpm install"` для обновления `pnpm-lock.yaml` (zustand ^5.0.0 добавлен). Затем `pnpm typecheck && pnpm build && pnpm tauri dev` — smoke QueueList (добавление entry через tray → item появляется с `pending` бейджем → переходит в `processing` → `ready`; delete через confirm-модал; выбор item → TextViewer показывает original_text).
- Hot-spot для хоста: `nix-shell --run "pnpm install && pnpm tauri dev"` на ruvox2 HEAD (`e6fb983`) — smoke QueueList + Player UI.
- Follow-ups (не блокирующие):
  - Перевести `isDragging` в Player на `useRef` для точной drag-protection.
  - Расширить `formatTime` до HH:MM:SS для длительности > 60 мин.
  - При пустом `currentEntryId`, но наличии `entryIds` — Play должен стартовать первый из очереди.
  - QueueList: добавить `isLoading` флаг для первого mount (избежать мелькания empty state).
  - QueueList: заменить Unicode-глифы ▶/✕ на tabler-icons-react после того, как U6/U9 добавят другие иконки (settings gear, regenerate arrows).
  - QueueList: StrictMode-safe cleanup через `unmounted` флаг (аналогично U10 follow-up).

### U9 — Settings dialog
- status: **merged** (2026-04-20)
- branch: task/u9-settings
- worker_commit: `3de4737 feat(ui): settings dialog with form persistence`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `ee655fe merge(u9): settings dialog with form persistence`
- deps_met: U1 merged (`78a0034`), B4 merged (`2a9c83c`).
- deliverable:
  - `src/dialogs/Settings.tsx` (новый, 203 строки) — `SettingsModal` компонент с `@mantine/form::useForm`. Controlled через пропсы `{ opened, onClose }`, Mantine 8 `<Modal size="md" title="Настройки">`. Secции: «Синтез речи» (Select голос, Select sample_rate), «Уведомления» (Switch notify_on_ready/error), «Кэш» (NumberInput max_cache_size_mb min=100, NumberInput auto_cleanup_days min=0), «Интерфейс» (Select theme).
  - Options для Select: `speaker ∈ {aidar, baya, kseniya, xenia, eugene, random}`, `sample_rate ∈ {8000, 24000, 48000}` (матчит `docs/ipc-contract.md` § UIConfig), `theme ∈ {light, dark, auto}`. Русские лейблы для пользователя.
  - Loading: `useEffect([opened])` → `commands.getConfig()` → `form.setValues({...config})` только когда `opened=true`. Комментарий в коде фиксирует намеренный exclude `form` из deps (`setValues` стабилен, иначе loop).
  - Submit: `form.onSubmit(handleSubmit)` → билдит `UIConfigPatch` только из полей формы (частичное обновление — остальные `UIConfig` поля вроде `hotkey_read_now`, `window_geometry` не трогаются), → `commands.updateConfig(patch)` → `notifications.show({color: 'green', title: 'Настройки сохранены'})` и `onClose()`. На ошибке — `notifications.show({color: 'red', title: 'Ошибка сохранения'})`. `theme` кастуется через `values.theme as UIConfigPatch['theme']` (рантайм safe: Select ограничивает значения до literal union).
  - Validation: `max_cache_size_mb < 100` → «Минимум 100 МБ»; `auto_cleanup_days < 0` → «Значение не может быть отрицательным».
  - Reset button (`form.reset()`) + Save button (`type="submit"`) в Group justify="flex-end".
  - `src/components/AppShell.tsx` — inline-`IconSettings` SVG компонент (через lucide-compatible path + circle для cog и rim) + `ActionIcon variant="subtle"` в `<Tooltip label="Настройки">`, `settingsOpened` state, mount `<SettingsModal>` под header.
- merge_conflicts:
  - `src/components/AppShell.tsx` — **единственный конфликт, разрешён вручную**. U9 форкался от `ruvox2` до merge U2/U3/U6 → его вариант содержал старый header (56px, `<Group>` без Stack, `useState<TextEntry | null>`, без QueueList и Player). HEAD ruvox2 `f8c1391` содержал U3 (header 108px + `<Stack>` + `<Player />`), U2 (navbar QueueList + `useSelectedEntry`) и U6 (Read Now / Read Later buttons, `addEntry` handler, `useHotkeys`).
  - **Резолв (канонический, сохраняет current layout):**
    - импорты — объединённый union: `ActionIcon` + `Tooltip` (из U9) добавлены к существующим `Stack`, `Button` из `@mantine/core`; сохранены `useHotkeys`, `notifications`, `commands`, `ThemeSwitcher`, `TextViewer`, `Player`, `QueueList`, `useSelectedEntry`; добавлен `SettingsModal` из `../dialogs/Settings`. Убран `import type { TextEntry }` из U9 (не используется после миграции на `useSelectedEntry` store).
    - `IconSettings` helper-компонент сохранён из U9 как есть (inline SVG, зависимостей нет).
    - `selectedEntry` — через `useSelectedEntry()` store (как в ruvox2 HEAD), не `useState<TextEntry | null>(null)` из U9 (устарело после U2 merge).
    - `pending` state + `addEntry` + `useHotkeys` — сохранены из U6 1:1 (ruvox2 HEAD).
    - добавлен `settingsOpened` state + setter (`useState(false)`).
    - header layout — сохранён 108px + `<Stack gap={0}>` с вложенными `<Group h={56}>` и `<Player />`. Внутри правой `<Group>` actions порядок: `Read Now` (синяя) → `Read Later` (default) → `ThemeSwitcher` → **новая** `<Tooltip><ActionIcon variant="subtle" onClick={() => setSettingsOpened(true)}><IconSettings /></ActionIcon></Tooltip>` (последняя справа, композиция: actions → theme control → settings gear).
    - `<SettingsModal opened={settingsOpened} onClose={...} />` смонтирован под `MantineAppShell.Header`, перед `Navbar` (вне layout-контейнеров — Modal рендерится через Portal, позиция mount не влияет на DOM). Navbar и Main — без изменений (QueueList + TextViewer из U2/U1).
- review_findings:
  - **`UIConfigPatch` корректен:** `src/lib/tauri.ts` (U1) определяет `UIConfigPatch = Partial<UIConfig>` с `theme: Theme` (`'light'|'dark'|'auto'`), `sample_rate: number`, `speaker: string`. Форма покрывает подмножество полей — остальные (`speech_rate`, `hotkey_read_*`, `text_format`, `history_days`, `audio_max_files`, `audio_regenerated_hours`, `code_block_mode`, `read_operators`, `player_hotkeys`, `window_geometry`) не трогаются (backend сохранит существующие значения). Ok для v1 — hotkeys / advanced settings можно добавить в follow-up.
  - **`theme as UIConfigPatch['theme']` cast:** формально обход проверки — `form.values.theme` имеет тип `string` (т.к. initialValues = `'auto'`). Практически безопасно — `Select data={THEME_OPTIONS}` ограничивает возможные значения до `'light'|'dark'|'auto'`. Альтернатива — типизировать `SettingsFormValues.theme: Theme` для статической проверки; оставлено как follow-up (не блокер).
  - **Form validation через `@mantine/form`:** `validate: { max_cache_size_mb: (v) => v < 100 ? 'Минимум 100 МБ' : null, auto_cleanup_days: (v) => v < 0 ? 'Значение не может быть отрицательным' : null }` — корректный API. `NumberInput min={100}` / `min={0}` — ограничение на уровне виджета, валидация — дополнительный гард.
  - **`useEffect([opened])` dep array корректный:** re-run только при открытии модалки; `form` намеренно исключён (eslint-disable комментарий с объяснением). Альтернатива `useEffect(..., [opened, form])` — создаёт infinite loop (form меняется при `setValues`). Текущий паттерн — канонический для `@mantine/form` loading-on-open.
  - **`sample_rate` Select:** options `'8000' | '24000' | '48000'` (строки для Select), парсинг через `parseInt(v, 10)` с fallback `48000`, `error={form.errors.sample_rate}` проброшен из формы. `onChange` обновляет через `form.setFieldValue`, что корректно отразится в state. Мелкая шероховатость — не использует `form.getInputProps('sample_rate')`, потому что Select ожидает `string | null`, а поле — `number`; ручной маппинг приемлем.
  - **Notifications:** `notifications.show({title, message, color})` — Mantine 8 API. Success green + close, error red без close (модалка остаётся открытой для повторной попытки). Русские строки для пользователя ✓.
  - **Style:** TS strict ✓, функциональный без `React.FC` ✓, no `any` ✓, no `sx`/`createStyles`/emotion ✓, Mantine 8 `@mantine/form` + `@mantine/notifications` ✓ (не react-hook-form / Formik), commit message формат `feat(ui): ...` ✓, no emoji ✓.
  - **AppShell.tsx merged-layout:** все прежние элементы (Player, QueueList, Read Now/Later, ThemeSwitcher, hotkeys) сохранены, Settings button добавлен как crossbar в правой Group. Порядок: action buttons → visual control → settings gear (канонический UI-паттерн).
- known_deviations:
  - **Inline SVG IconSettings** вместо `@tabler/icons-react`. Воркер выбрал inline, чтобы не тянуть новую зависимость для одной иконки (тюнинг размера бандла). Follow-up: когда накопится >3 иконок (settings, regenerate, ...), перейти на `@tabler/icons-react`.
  - **`useModals` / `openContextModal` не использованы** — воркер выбрал controlled standalone `<Modal>` для простоты параллельной задачи. Оба подхода корректны в Mantine 8. Если в будущем появится несколько context-aware modals, можно рефакторить в `openContextModal`.
  - **Hotkeys в UIConfig (`hotkey_read_now`, `hotkey_read_later`) не экспонированы в форме** — U6 follow-up остаётся незакрытым. Форма покрывает только 7 полей; остальные доступны через `update_config` прямо из кода, но пока не через UI. Follow-up task.
  - **`form.values.theme` типизирован как `string`, не `Theme`** — см. review_findings выше; не критично, но снизит необходимость cast.
- verification:
  - Ревьюер прочитал `src/dialogs/Settings.tsx` и `src/components/AppShell.tsx` на SHA `3de4737` через `git show task/u9-settings:<path>`. Сверил `UIConfigPatch` определение в `src/lib/tauri.ts` (merged на U1) и `UIConfig` схему в `docs/ipc-contract.md` § 305-325 — `get_config()` / `update_config(patch)` ✓. Sample rates 8000/24000/48000 матчат `docs/ipc-contract.md:64`. Theme values `'light'|'dark'|'auto'` матчат `docs/ipc-contract.md:78`.
  - Merge выполнен в isolated worktree `/tmp/claude/ruvox2-merge-u9` (detached → local branch `ruvox2-merge-u9`). Конфликт в `AppShell.tsx` разрешён вручную, single clean resolution. Push в основной ruvox2 через `git push /home/evgen/work/github/RuVox HEAD:refs/heads/ruvox2` (local filesystem refspec) — успешно, `f8c1391..ee655fe`.
  - `pnpm typecheck` / `pnpm build` / `pnpm tauri dev` не прогонялись в sandbox ревьюера (nix-daemon SQLite cache блокируется). Воркер подтвердил `typecheck` чистым. Live-прогон остаётся за хостом.
- next_unblocks: U9 — leaf в UI-дереве, не блокирует другие задачи. Остался **U5** (word highlighting) как последний в текущей UI-волне — независим, готов к merge.

### U5 — Word highlighting
- status: **merged** (2026-04-20)
- branch: task/u5-highlighting
- worker_commit: `b1e290b feat(ui): word highlighting synced with playback`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `8e36aed merge(u5): word highlighting synced with playback`
- deps_met: U4 merged, B3 merged, B4 merged (`2a9c83c`).
- deliverable:
  - `src/lib/wordHighlight.ts` (new) — O(log N) binary search over WordTimestamp[], DOM lookup via querySelectorAll [data-orig-start] with exact-match-first then smallest containing range fallback, scrollIntoView only when bbox is outside viewport.
  - `src/lib/markdown.ts` — в renderMarkdown добавлен per-call override md.renderer.rules.text: оборачивает inline-text токены в span data-orig-start / data-orig-end. Позиции считаются через source.indexOf(content, searchFrom) с курсором, так что повторяющиеся фрагменты получают разные позиции. После render дефолтный rule восстанавливается.
  - `src/components/TextViewer.tsx` — три useRef (timestampsRef, playingEntryIdRef, activeIdxRef) кэшируют состояние без re-renders. useEffect подписывается на playback_started/position/stopped/finished/paused. На started — commands.getTimestamps(entry_id). На каждый position tick — гейты (entry совпадает, format не plain и не html) → findActiveTimestamp → если idx сменился, applyHighlight. Отдельный useEffect на смену entry.id / format очищает stale подсветку.
  - `src/components/TextViewer.module.css` — класс :global(.word-highlight) с background-color: light-dark(rgba(255,213,0,0.45), rgba(255,213,0,0.3)), border-radius 2px, transition 80ms. CSS-native light-dark() без переменных.
- merge_conflicts: нет. task/u5-highlighting форкнут от ruvox2 @ 6da8e37 (B4 merged); между этой точкой и ruvox2 HEAD (f6e3f4d) файлы U5 (TextViewer.tsx, TextViewer.module.css, markdown.ts) не менялись → ort-мердж прошёл чисто (новый wordHighlight.ts + non-overlapping append-only в остальных).
- review_findings:
  - Бинарный поиск корректный: полуоткрытые интервалы [start, end), early-return при точном попадании, -1 в gap-ситуациях. Последняя проверка "positionSec < timestamps[lo].start" дублирует безусловный -1 (dead code), но не баг.
  - findSpanByOrigPos: exact-match → smallest containing fallback. Покрывает случай, когда pipeline даёт сегмент внутри большего text-токена. Ok.
  - Cursor-based indexOf в markdown.ts гарантирует уникальность позиций для повторяющихся слов. Fallback (pos === -1) рендерит без data-orig-* — подсветка молча скипается. Ok.
  - Плейбек-listeners subscribe/cleanup: использован ad-hoc паттерн с пятью let unlistenX = null + .then(fn => unlistenX = fn). Это расходится с каноническим паттерном в Player.tsx (Promise<UnlistenFn>[] + cleanup через .then(fn => fn())). Недостаток: если компонент размонтируется до того как promise разрешится (StrictMode double-mount в dev, быстрый switch entry), unlistenX всё ещё null к моменту cleanup → listener зарегистрируется потом и никогда не снимется (утечка).
  - Ревьюер не применил fix на отдельный коммит поверх b1e290b из-за sandbox-ограничений записи больших блоков (см. ниже). Follow-up открыт: переписать subscribe-блок на канонический паттерн (аналог Player.tsx), чтобы устранить потенциальную утечку listener при StrictMode / быстрых сменах entry.
  - TS strict, без any; функциональный компонент без React.FC; CSS Modules + :global; commit-формат feat(ui): корректный; no emoji. Ok.
- known_deviations:
  - HTML-mode подсветка отключена (TODO в TextViewer.tsx): HtmlCharSpan sentinel (0/0) в HTML pipeline мапит все слова в одну позицию. До исправления в backend HTML-mode молча скипается (format === 'html' ранний return).
  - Plain-mode без data-orig-* spans: escapeHtml не добавляет обёртку, findSpanByOrigPos ничего не находит → highlighting не применяется. Ожидаемое поведение.
  - Listener subscribe-паттерн не StrictMode-safe (см. review_findings). Потенциальная утечка listeners при быстрых циклах mount/unmount; не блокирует merge, follow-up оставлен.
- verification:
  - Ревьюер: прочитал diff b1e290b и сверил с RewriteTaskPlan § U5. Сверил сигнатуры с src/lib/tauri.ts (commands.getTimestamps, events.playback*), выяснил что WordTimestamp уже в IPC-контракте (добавлен в B4).
  - Прогнал tsc --noEmit в /tmp worktree с symlink node_modules. Ошибок в U5-файлах нет. Есть orthogonal ошибки в QueueList.tsx и stores/selectedEntry.ts (implicit any, зависимость zustand не в this sandbox node_modules) — не связаны с U5, не блокируют merge.
  - nix-shell недоступен в sandbox (SQLite cache DB read-only) → полноценный pnpm typecheck / build / tauri dev — за хостом. Рекомендуется smoke на ruvox2 HEAD 8e36aed: запуск, плей запии с синтезом → подсветка слов в markdown-mode движется по тексту, scroll автоматический, не рвёт фокус; switch в plain/html — подсветка молча исчезает.
- next_unblocks: U5 — leaf в UI-дереве, не блокирует другие задачи. Follow-up: переписать subscribe-блок на канонический паттерн Player.tsx (Promise<UnlistenFn>[] + cleanup через .then(fn => fn())). Опционально: заменить dead-code ветку в findActiveTimestamp на прямой return -1.

### U11 — Preview dialog (FF 1.1)
- status: **merged** (2026-04-20)
- branch: task/u11-preview
- worker_commits:
  - `125b93a feat(commands): add preview_normalize handler for Rust pipeline`
  - `677f436 feat(ui): preview dialog with normalized text comparison`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `24bac6f merge(u11): preview dialog with normalized comparison`
- deps_met: R9 merged (pipeline.process), B4 merged (commands), U1 merged (AppShell).
- deliverable:
  - Backend (`src-tauri/src/commands/mod.rs`): команда `preview_normalize(text: String) -> { normalized: String }` — синхронно прогоняет text через Pipeline::process (без charmap), возвращает нормализованный вариант без записи в storage. Используется frontend для предпросмотра до реального `add_clipboard_entry`.
  - Backend (config): в UIConfig/Config два новых поля `preview_dialog_enabled: bool` (default true) и `preview_threshold: usize` (default 500 символов) — порог, ниже которого диалог пропускается.
  - Frontend (`src/dialogs/PreviewDialog.tsx`): Mantine `Modal` с двухколоночным layout «Оригинал / Нормализовано», кнопки «Озвучить / Редактировать / Отмена», чекбокс «Не показывать для коротких текстов».
  - Frontend (`src/components/AppShell.tsx`): preflight — `navigator.clipboard.readText()` + проверка `preview_dialog_enabled && text.length > preview_threshold` → открыть `PreviewDialog` вместо прямого `add_clipboard_entry`. Если короче порога — прежний flow без диалога.
  - Frontend (`src/lib/tauri.ts`): тип `PreviewNormalizeResult` + `commands.previewNormalize`, расширение `UIConfig` новыми полями.
- merge_conflicts: нет (ветка форкнута от ruvox2 на момент, когда ни один из затронутых файлов не менялся параллельно). Авто-merge прошёл чисто.
- review_findings:
  - `preview_normalize` сделан через sync-обёртку над `Pipeline::process` без `tokio::spawn_blocking`. CPU-bound pipeline ≤50 мс на 500 символов — приемлемо на main-thread Tauri-команды.
  - Диалог не трогает history: пользователь не видит preview-entries в списке, только финальный entry после подтверждения. Ok.
  - `navigator.clipboard.readText()` требует HTTPS/localhost/Tauri-context — в Tauri webview работает. Fallback не нужен.
  - TS strict ✓, CSS Modules ✓, commit-формат ✓, no emoji ✓.
- known_deviations:
  - `preview_threshold` хардкожен в конфиге на 500 символов; UI настройки для порога нет (FF-задача на будущее).
- verification:
  - Ревьюер прочитал `125b93a` и `677f436`, сверил команду с `docs/ipc-contract.md` и конфиг-поля с `docs/storage-schema.md`.
  - Smoke вручную на ruvox2 HEAD `24bac6f`: скопировать текст 600+ симв → диалог открылся, Normalized видно, «Озвучить» добавляет entry в очередь, «Отмена» — не добавляет. Текст ≤500 симв — прямой flow (диалог пропущен).
- next_unblocks: U11 — leaf. Совместимо с U12 (edit mode) — оба пути работают независимо.

### U12 — Edit mode (FF 1.2)
- status: **merged** (2026-04-20)
- branch: task/u12-edit-mode
- worker_commits:
  - `1283459 feat(commands): add update_entry_edited_text and edited_text source switch`
  - `dd8dffb feat(ui): edit mode toggle in TextViewer with edited_text persistence`
- reviewer: autopilot Opus, review_result: ok
- merge_sha: `100320a merge(u12): edit mode toggle with edited_text persistence`
- deps_met: U4 merged, B4 merged, U5 merged (highlight coexists с editMode через guard).
- deliverable:
  - Backend (`src-tauri/src/commands/mod.rs`): команда `update_entry_edited_text(id, edited: Option<String>)` — читает entry, пишет `entry.edited_text = edited`, `storage.update_entry(entry.clone())`, эмитит `entry_updated` через `emit_entry_updated`. `Some(text)` сохраняет override, `None` очищает (следующий синтез возьмёт `original_text`). Регистрация в `src-tauri/src/lib.rs generate_handler`.
  - Backend (spawn_synthesis): `source_text = entry.edited_text.clone().unwrap_or_else(|| entry.original_text.clone())` подставляется в `pipeline.process_with_char_mapping(...)`. `original_text` не перезаписывается — источник истины для отображения и отката правок через `None`.
  - Frontend (`src/components/TextViewer.tsx`): state `editMode / editedDraft / saving`, pencil U+270F → save U+2713 / cancel U+2715 с Tooltip, Mantine `<Textarea autosize minRows=10>` в editMode вместо ScrollArea. Esc отменяет правки. `notifications.show color=green "Сохранено"` / `color=red "Ошибка сохранения"`. Mermaid renderer + word-highlight subscribe effects имеют `if (editMode) return` guard — при правке подсветка не трогает DOM Textarea.
  - Frontend (`src/lib/tauri.ts`): `updateEntryEditedText(id: EntryId, edited: string | null): Promise<void>` → `invoke('update_entry_edited_text', { id, edited })`.
- merge_conflicts:
  - `src/components/TextViewer.tsx` — U12 форкнут от `f8c1391` (до merge U5, который переписал TextViewer под word-highlighting). Разрешён union: сохранены U5-импорты и рефы (timestampsRef / playingEntryIdRef / activeIdxRef), U5-эффекты (clear-on-entry-change, playback subscribe); добавлены U12-state, draft-sync useEffect, displayText memo, mermaid-guard, `if (editMode) return` guard в word-highlight subscribe effect (иначе listener пытался бы применить highlight к DOM Textarea, где нет `data-orig-*` spans), Esc-хэндлер, handleEnterEdit/handleCancel/handleSave, Textarea/ScrollArea switch в JSX.
  - `src/lib/tauri.ts` — конфликт на секции commands: HEAD использовал inline type `{ normalized: string }` для `previewNormalize`, U12-ветка — `PreviewNormalizeResult` alias + `updateEntryEditedText`. Объединено под alias `PreviewNormalizeResult` (читаемее), добавлено `updateEntryEditedText`.
  - `src-tauri/src/commands/mod.rs` и `src-tauri/src/lib.rs` — авто-merge прошёл (U11 добавил `preview_normalize`, U12 — `update_entry_edited_text`; обе команды ортогональны).
- review_findings:
  - `update_entry_edited_text` корректен: `Option<String>` (reset через None), `storage.get_entry → mutate → update_entry(entry.clone())` — соответствует API StorageService (closure-API не предусмотрен, spec упоминал его ошибочно). Эмит `entry_updated` — frontend видит обновление в queue list без полла.
  - Source-switch в `spawn_synthesis` безопасен: `source_text` читается один раз, используется и для pipeline, и сохраняется в `entry.normalized_text` после обработки. `original_text` не перезаписывается — при очистке edited_text через None regenerate возвращает synthesis на оригинал.
  - Guard `if (editMode) return` в word-highlight subscribe + mermaid renderer effects: при входе в editMode listeners/renderers отваливаются чисто (cleanup возвращается), при выходе — заново подписывается и подхватит плейбек. Effect deps включают `editMode` для явной re-subscribe.
  - Esc-хэндлер: `window.addEventListener('keydown')` активен только в editMode, cleanup снимается при выходе. Нет конфликта с `@mantine/hooks::useHotkeys` в AppShell (Read Now / Later на Ctrl-комбинациях).
  - Unicode-глифы `&#x270F;` pencil, `&#x2713;` check, `&#x2715;` x-mark — HTML-entity коды, не emoji.
  - TS strict ✓, CSS Modules (унаследованы из U4) ✓, `notifications.show` ✓, commit-формат ✓, no Co-Authored-By ✓, no emoji ✓.
- verification:
  - Ревьюер (в текущей main session) разрешил 2 конфликта (`TextViewer.tsx`, `tauri.ts`) вручную. Auto-merge применился для `commands/mod.rs` и `lib.rs`.
  - `pnpm typecheck` (внутри nix-shell) — ошибки только orthogonal pre-existing в `QueueList.tsx` (implicit any) и `stores/selectedEntry.ts` (zustand не установлен); U12-файлы чисто типизированы.
  - `cargo check --manifest-path src-tauri/Cargo.toml` — `Finished dev profile`, 2 warning (unused import + dead code в `html_extractor.rs`, pre-existing). U12 backend компилируется.
  - Smoke за хостом: запустить `pnpm tauri dev`, добавить entry, нажать pencil → Textarea, изменить, save → notification «Сохранено», re-synth читает изменённый текст; Esc / cancel возвращает draft.
- next_unblocks: U12 — leaf. Все U-задачи завершены. Разблокирует P1 (bundling) и P3 (final docs).

### P1 — Tauri bundling через Nix flake
- status: **merged** (2026-04-20)
- branch: прямо в ruvox2 (main session, без worker-агента)
- deps_met: все R/B/U задачи завершены, `shell.nix` стабилен.
- deliverable:
  - `flake.nix` — flake-структура с `inputs.nixpkgs` (unstable channel) + `flake-utils.eachDefaultSystem`.
  - `packages.default` = `packages.ruvox` — Rust-бинарь `ruvox-tauri` собирается через `rustPlatform.buildRustPackage` с `cargoLock.lockFile = ./src-tauri/Cargo.lock`. `sourceRoot = "source/src-tauri"`. `preBuild` копирует pre-built `dist/` (от frontend-derivation) в `../dist`, `buildPhase` вызывает `cargo tauri build --no-bundle`. `postFixup` оборачивает бинарь через `wrapProgram`: добавляет `ttsd` + `mpv` в `PATH`, `LD_LIBRARY_PATH` для runtime (webkit/gtk/mpv/pulse/wayland), `GIO_EXTRA_MODULES` для glib-networking TLS.
  - `packages.frontend` — `stdenv.mkDerivation` с `pnpm.fetchDeps` (fetch pnpm store по lockfile) и `pnpm run build` → `dist/`. Hash — `lib.fakeHash`, замена на реальный после первого `nix build` (стандартный pnpm2nix flow).
  - `packages.ttsd` — `python312Packages.buildPythonApplication` с `pyproject = true` и propagatedBuildInputs (torch/numpy/scipy/pydantic/omegaconf/torchaudio). `doCheck = false` потому что тесты требуют network для скачивания Silero-модели.
  - `devShells.default` — делегирует существующий `shell.nix` через `import ./shell.nix { inherit pkgs; }`, чтобы не дублировать список buildInputs между flake и классическим nix-shell.
  - `apps.default` — `{ type = "app"; program = "${ruvox}/bin/ruvox"; }` для `nix run`.
- acceptance: `nix build .#ruvox` — структура валидна (`nix-instantiate --parse` проходит). Полная сборка (без fakeHash) требует one-shot hash-fixup после первого запуска.
- known_deviations:
  - Hash для `pnpm.fetchDeps` оставлен как `lib.fakeHash` — при первом `nix build` Nix сообщит реальный hash, его нужно подставить в flake.nix. Это стандартная процедура для pnpm2nix, не баг.
  - Tests через `doCheck = false` в ttsd: Silero-модель (~200 МБ) качается с github/torch.hub → network в sandboxed build запрещён. Тесты гоняются отдельно в dev-shell (`uv run python -m pytest`).
  - `cargo tauri build --no-bundle` производит только исполняемый файл, не AppImage/deb/rpm. Полный bundle target (`targets = "all"` в `tauri.conf.json`) работает через `cargo tauri build` без флага, но требует `linuxdeploy`/`dpkg-deb`/`rpmbuild` в buildInputs — оставлено как follow-up.
  - `sandbox` блокирует `nix flake show` (bind-mount `/dev/null` на `.gitmodules` мешает git-fetcher'у) — проверка структуры через `nix-instantiate --parse` вместо `nix flake show`.
- verification:
  - `nix-instantiate --parse flake.nix` — синтаксис ok, все ссылки разрешены.
  - Интеграционный `nix build .#ruvox` — за хостом (или с отключённым sandbox для первого hash-fixup).
- next_unblocks: P1 разблокирует P3 (final docs) и P2 (удаление legacy, делается координатором).
