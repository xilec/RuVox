# RuVox

**RuVox** — desktop-приложение для озвучивания технических текстов на русском языке через Silero TTS. Нормализует английские термины, аббревиатуры, код, числа, URL до передачи в TTS, чтобы синтезатор корректно читал материал, для которого не предназначен.

## Проблема → Решение

Silero TTS не умеет корректно произносить:
- Английские слова и IT-термины (`feature` → молчание или искажение)
- Аббревиатуры (`API`, `HTTP`, `JSON`)
- URL, email, IP-адреса, пути
- Идентификаторы кода (`getUserData`, `my_variable`)
- Спецсимволы и операторы (`->`, `>=`, `!=`)

```
"Вызови getUserData() через API" → "Вызови гет юзер дата через эй пи ай"
```

## Возможности

- **Кнопка Add** — копируете текст в буфер, нажимаете Add → запись попадает в очередь и синтезируется.
- **Preview-диалог** — для длинных текстов отдельный floating-window показывает оригинал и нормализованную версию side-by-side; можно отредактировать оригинал перед синтезом.
- **Edit mode** — правка `original_text` прямо в viewer, изменения сохраняются на entry.
- **Очередь** — список всех записей с бейджами статуса (`pending` / `processing` / `ready` / `playing` / `error`).
- **Подсветка слов** — синхронная подсветка читаемого слова в markdown-режиме через бинарный поиск по `WordTimestamp`.
- **Mermaid** — диаграммы рендерятся в UI; для TTS заменяются маркером «тут мермэйд диаграмма».
- **Системный трей** — close-to-tray, тёплый mpv re-init при показе окна.

## Стек

| Слой | Технология |
|------|------------|
| Shell | [Tauri 2](https://tauri.app/) (Rust + нативный webview) |
| Frontend | React 18 + TypeScript 5 + [Mantine 8](https://mantine.dev/) |
| Backend | Rust (pipeline нормализации, storage, TTS-менеджер, обёртка плеера) |
| TTS | Python 3.12 subprocess `ttsd`, обёртка над [Silero TTS](https://github.com/snakers4/silero-models) |
| Аудио | [`tauri-plugin-mpv`](https://crates.io/crates/tauri-plugin-mpv) (libmpv с `scaletempo2`) |
| Сборочное окружение | Nix (`shell.nix` + `flake.nix`) |

## Документация

### Архитектура и история

- [RewriteNotes.md](../RewriteNotes.md) — архитектурные решения и обоснования выбора стека.
- [RewriteTaskPlan.md](../RewriteTaskPlan.md) — детальный план задач переписывания, граф зависимостей.
- [task_history.md](../task_history.md) — журнал исполнения задач.
- [CHANGELOG.md](../CHANGELOG.md) — хронология версий.

### Справка

- [IPC-контракт](ipc-contract.md) — Tauri-команды, события, JSON-протокол ttsd.
- [Storage-схема](storage-schema.md) — `history.json`, `config.json`, `{uuid}.timestamps.json`, `{uuid}.wav`.
- [Pipeline нормализации](pipeline.md) — этапы обработки, нормалайзеры, golden-тесты.
- [UI-компоненты](ui.md) — структура React-приложения, компоненты, стилизация.
- [Preview-диалог (FF 1.1)](preview-dialog.md) — поведение, настройки, флоу взаимодействия.

### Сценарии и разработка

- [Сценарии использования](use-cases.md) — пользовательские сценарии: добавление текста, plain/markdown-режимы, mermaid, подсветка слов.
- [Разработка](development.md) — окружение, команды, отладка.
- [Contributing](contributing.md) — как добавить термин в словарь, правила коммитов и стиля.

## Лицензия

GPL-3.0 — см. [LICENSE.md](../LICENSE.md).
