# Preview-диалог (FF 1.1)

Floating non-modal окно для предпросмотра нормализованного текста перед синтезом. Реализует FF 1.1 из roadmap'а.

**Источник:** `src/dialogs/PreviewDialog.tsx`, `src/dialogs/PreviewDialog.module.css`. Backend-команда: `preview_normalize` в `src-tauri/src/commands/mod.rs`.

## Зачем

Pipeline нормализации детерминистичен, но иногда даёт результат, который пользователь хочет скорректировать вручную: исправить транслитерацию термина, вырезать лишний код, изменить структуру предложения. Запускать TTS на каждом таком тексте, потом удалять и переcинтезировать — расход батареи и времени.

Preview-диалог показывает оригинал и нормализованную версию side-by-side, даёт отредактировать оригинал и видеть нормализацию вживую (debounce 1 секунда), и только потом запускает синтез.

## Поведение

### Открытие

Диалог открывается из `AppShell` после нажатия кнопки **Add**:

1. Прочитать буфер обмена (`tauri-plugin-clipboard-manager::readText`).
2. Прочитать `UIConfig` (один раз на mount).
3. Если `config.preview_dialog_enabled === true` — открыть `<PreviewDialog>` с `text = clipboardText`.
4. Если `false` — пропустить диалог, сразу `commands.addTextEntry(text, true)`.

По умолчанию `preview_dialog_enabled = true` (`storage::schema::UIConfig::default`).

### Окно

- **Floating, non-modal.** Реализован через `react-rnd` внутри Mantine `Portal`. Не блокирует UI под собой.
- **Drag** — за header («Предпросмотр нормализации»).
- **Resize** — за любой край / угол. Минимум `560 × 380`, начальный размер `900 × 620`, центрируется при каждом открытии.
- **ESC** — закрывает диалог (как Cancel).
- **Anchor:** `Rnd` положен внутрь viewport-sized fixed-контейнера (`viewportContainer` CSS), чтобы координаты `(x, y)` соответствовали viewport-у независимо от document-scroll.

### Содержимое

```
┌─ Header ──────────────────────────────────────────┐
│ Предпросмотр нормализации                  [✕]    │
├─ Body ────────────────────────────────────────────┤
│ ┌──────────────────┬────────────────────────────┐ │
│ │ Оригинал         │ После нормализации         │ │
│ │ ┌──────────────┐ │ ┌────────────────────────┐ │ │
│ │ │ Текст /      │ │ │ Нормализованный        │ │ │
│ │ │ Textarea     │ │ │ результат / Loader     │ │ │
│ │ │ (edit mode)  │ │ │                        │ │ │
│ │ └──────────────┘ │ └────────────────────────┘ │ │
│ └──────────────────┴────────────────────────────┘ │
├─ Footer ──────────────────────────────────────────┤
│ [☐] Больше не показывать  [☐] Синхр. скроллинг    │
│              [Read Now ✓] [Отмена] [Редакт.] [Синт.]│
└────────────────────────────────────────────────────┘
```

**Левая панель — Оригинал:**
- По умолчанию `<pre>` read-only с прокруткой.
- После клика «Редактировать» — `<Textarea>` с текущим значением (`editedText`).
- При вводе → перенормализация с debounce 1000 мс.

**Правая панель — После нормализации:**
- `<pre>` с результатом `commands.previewNormalize(text)`.
- Во время `loading` — `<Loader>`.
- На ошибке — текст `"(ошибка нормализации: ...)"`.

### Footer

| Контрол | Назначение |
|---------|-----------|
| **Больше не показывать этот диалог** (Checkbox) | При синтезе сбросит `preview_dialog_enabled` в `false` через `commands.updateConfig`. |
| **Синхронный скроллинг** (Checkbox) | Зеркальный scroll левой и правой панелей по относительной позиции. Имеет защиту от ping-pong через `syncingRef`. |
| **Read Now** (Switch, default ON) | Передаётся в `addTextEntry(text, playWhenReady)`. ON — воспроизводить сразу после `ready`. OFF — добавить в очередь, не играть. |
| **Отмена** | Закрывает диалог, ничего не добавляет. |
| **Редактировать** | Переключает левую панель в edit-mode (показывается только когда mode = read-only). |
| **Синтезировать** | Закрывает диалог + `addTextEntry`. |

### Синтез

При нажатии **Синтезировать** (`handleSynthesize`):

1. Если `editMode` — взять `editedText.trim()`; иначе — исходный `text`.
2. Если `skipShortTexts` — `commands.updateConfig({ preview_dialog_enabled: false })`.
3. `addTextEntry(finalText, playWhenReady)` (где `playWhenReady` — состояние Read Now switch).

Имя checkbox'а «Больше не показывать» исторически соответствовало «не показывать для коротких текстов» (порог `preview_threshold` в UIConfig), но фича порога [была удалена](../task_history.md) в `ee90518 chore(config): drop unused preview_threshold` — сейчас checkbox **глобально отключает диалог**, без порога.

## Backend: `preview_normalize`

```rust
#[tauri::command]
pub async fn preview_normalize(
    state: State<'_, AppState>,
    text: String,
) -> CmdResult<PreviewNormalizeResult> {
    let pipeline = Arc::clone(&state.pipeline);
    let result = tokio::task::spawn_blocking(move || {
        let mut p = pipeline.lock();
        p.process_with_char_mapping(&text)
    })
    .await
    .map_err(|e| CommandError::Internal {
        message: format!("pipeline task panicked: {e}"),
    })?;

    let (normalized, _char_mapping) = result;
    Ok(PreviewNormalizeResult { normalized })
}
```

Особенности:

- `spawn_blocking` — pipeline CPU-bound, чтобы не блокировать tokio reactor (хотя на 500 символах — ≤ 50 мс, но при больших входах может растянуться).
- `char_mapping` отбрасывается — preview-диалогу нужен только результат-строка.
- Storage **не трогается** — preview не создаёт `TextEntry`. Пользователь не видит preview-черновики в очереди.

## Конфигурация

Поле в `UIConfig`:

```typescript
preview_dialog_enabled: boolean   // default true
```

- **Установка:** Settings dialog → переключатель «Показывать preview-диалог».
- **Сброс через checkbox в самом диалоге:** при отметке «Больше не показывать» → `commands.updateConfig({ preview_dialog_enabled: false })`.
- **Возврат:** через Settings dialog → toggle обратно в `true`.

## TypeScript типы

```typescript
// src/lib/tauri.ts
export interface PreviewNormalizeResult {
  normalized: string;
}

commands.previewNormalize: (text: string) => Promise<PreviewNormalizeResult>
```

## Когда применять правки

Preview-диалог — единственное место в текущем UI, где пользователь может править исходный текст **до** синтеза. После «Синтезировать» текст уходит в `addTextEntry` как готовый и записывается в storage как `original_text`; пере-редактирование уже синтезированной записи не предусмотрено — нужно удалить запись и добавить заново.

> Ранее существовавший edit mode в `TextViewer` (FF 1.2) удалён в этой ветке — фича сохраняла правки в `edited_text`, но re-synth не запускала, поэтому воспроизведение продолжало играть старый WAV. См. соответствующий коммит в `task_history.md`.
