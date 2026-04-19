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
- status: **needs_fix → fix in progress**
- branch: task/f1-nix-flake
- worker_commit: `cb4edb8 build(nix): shell.nix with Rust + Node + Python + Tauri deps`
- reviewer: autopilot (Opus), review_result: **needs_fix**
- **blocker:** воркер ошибочно решил, что `cargo-tauri` отсутствует в nixpkgs 25.11, и задокументировал workaround через `cargo install tauri-cli`. Ревьюер проверил: `pkgs.cargo-tauri` 2.9.4 **есть** в nixpkgs 25.11 (`pkgs/by-name/ca/cargo-tauri/package.nix`). Воркер, вероятно, искал `tauri-cli`/`tauri` вместо корректного имени.
- fix_agent: запущен на исправление (autopilot-sonnet). Задача: добавить `cargo-tauri` в `buildInputs`, убрать неверные комментарии и echo из shellHook.
- other_checks_ok:
  - Rust 1.91.1 + rustfmt + clippy + clippy-driver — ок.
  - nodejs_20 + pnpm 10.25.0 + uv 0.9.16 + python312 — ок.
  - webkitgtk_4_1 2.50.3 + libsoup_3 + mpv-unwrapped 0.40.0 + pkg-config — ок (статическая верификация через `nix-instantiate --eval`; sandbox блокирует live `nix-shell --run`, проверка на хост-машине остаётся за пользователем).
  - Style: mirror `legacy/shell.nix` pattern. Без Claude-упоминаний.
- minor: `openssl.dev` указан отдельно от `openssl` — избыточно, но не ошибка. Может быть почищено в fix-коммите.

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
