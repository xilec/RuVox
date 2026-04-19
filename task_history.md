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
