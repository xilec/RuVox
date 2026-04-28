# Preview dialog (FF 1.1)

A floating non-modal window for previewing the normalized text before synthesis. Implements FF 1.1 from the roadmap.

**Source:** `src/dialogs/PreviewDialog.tsx`, `src/dialogs/PreviewDialog.module.css`. Backend command: `preview_normalize` in `src-tauri/src/commands/mod.rs`.

## Why

The normalization pipeline is deterministic, but sometimes produces a result the user wants to adjust manually: fix the transliteration of a term, cut out extraneous code, change the structure of a sentence. Running TTS on every such text and then deleting and re-synthesizing it wastes battery and time.

The preview dialog shows the original and normalized version side by side, lets you edit the original and see normalization live (1-second debounce), and only then triggers synthesis.

## Behavior

### Opening

The dialog opens from `AppShell` after the **Add** button is pressed:

1. Read the clipboard (`tauri-plugin-clipboard-manager::readText`).
2. Read `UIConfig` (once per mount).
3. If `config.preview_dialog_enabled === true` — open `<PreviewDialog>` with `text = clipboardText`.
4. If `false` — skip the dialog and call `commands.addTextEntry(text, true)` directly.

By default `preview_dialog_enabled = true` (`storage::schema::UIConfig::default`).

### Window

- **Floating, non-modal.** Implemented via `react-rnd` inside a Mantine `Portal`. Doesn't block the UI underneath.
- **Drag** — by the header ("Предпросмотр нормализации").
- **Resize** — by any edge / corner. Minimum `560 × 380`, initial size `900 × 620`, centered on every open.
- **ESC** — closes the dialog (acts as Cancel).
- **Anchor:** `Rnd` is placed inside a viewport-sized fixed container (`viewportContainer` CSS) so that the `(x, y)` coordinates correspond to the viewport regardless of document scroll.

### Contents

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

**Left panel — Original:**
- By default a read-only `<pre>` with scrolling.
- After clicking "Редактировать" — `<Textarea>` with the current value (`editedText`).
- On input → re-normalization with a 1000 ms debounce.

**Right panel — After normalization:**
- A `<pre>` with the result of `commands.previewNormalize(text)`.
- During `loading` — a `<Loader>`.
- On error — the text `"(ошибка нормализации: ...)"`.

### Footer

| Control | Purpose |
|---------|---------|
| **Больше не показывать этот диалог** (Checkbox) | On synthesis, sets `preview_dialog_enabled` to `false` via `commands.updateConfig`. |
| **Синхронный скроллинг** (Checkbox) | Mirrors the scroll of the left and right panels by relative position. Has ping-pong protection via `syncingRef`. |
| **Read Now** (Switch, default ON) | Passed to `addTextEntry(text, playWhenReady)`. ON — play immediately after `ready`. OFF — add to the queue, don't play. |
| **Отмена** | Closes the dialog, adds nothing. |
| **Редактировать** | Switches the left panel to edit mode (only shown while mode = read-only). |
| **Синтезировать** | Closes the dialog + `addTextEntry`. |

### Synthesis

When **Синтезировать** is pressed (`handleSynthesize`):

1. If `editMode` — take `editedText.trim()`; otherwise — the original `text`.
2. If `skipShortTexts` — `commands.updateConfig({ preview_dialog_enabled: false })`.
3. `addTextEntry(finalText, playWhenReady)` (where `playWhenReady` is the state of the Read Now switch).

The "Больше не показывать" checkbox name historically corresponded to "don't show for short texts" (the `preview_threshold` field in UIConfig), but the threshold feature [was removed](../task_history.md) in `ee90518 chore(config): drop unused preview_threshold` — currently the checkbox **disables the dialog globally**, with no threshold.

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

Notes:

- `spawn_blocking` — the pipeline is CPU-bound, so we don't block the tokio reactor (although on 500 characters it's ≤ 50 ms; on large inputs it can stretch).
- `char_mapping` is discarded — the preview dialog only needs the result string.
- Storage **is not touched** — preview doesn't create a `TextEntry`. The user doesn't see preview drafts in the queue.

## Configuration

Field in `UIConfig`:

```typescript
preview_dialog_enabled: boolean   // default true
```

- **Setting:** Settings dialog → "Показывать preview-диалог" toggle.
- **Reset via the in-dialog checkbox:** ticking "Больше не показывать" → `commands.updateConfig({ preview_dialog_enabled: false })`.
- **Restore:** through the Settings dialog → toggle back to `true`.

## TypeScript types

```typescript
// src/lib/tauri.ts
export interface PreviewNormalizeResult {
  normalized: string;
}

commands.previewNormalize: (text: string) => Promise<PreviewNormalizeResult>
```

## When to apply edits

The preview dialog is the only place in the current UI where the user can edit the source text **before** synthesis. After "Синтезировать" the text goes into `addTextEntry` as final and is written to storage as `original_text`; re-editing an already-synthesized entry is not supported — you have to delete the entry and add it again.

> The previously existing edit mode in `TextViewer` (FF 1.2) was removed in this branch — the feature saved edits to `edited_text` but didn't trigger re-synth, so playback kept playing the old WAV. See the corresponding commit in `task_history.md`.
