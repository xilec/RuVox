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
