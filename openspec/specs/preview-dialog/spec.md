# Preview Dialog Specification

## Purpose

Covers the normalization preview dialog (FF 1.1): a floating, non-modal window (`src/dialogs/PreviewDialog.tsx`) that shows the clipboard text and its normalized form side by side before synthesis, lets the user edit the source text with live re-normalization, and only then creates a queue entry. Also covers the `preview_normalize` backend command and the `preview_dialog_enabled` configuration gate that controls whether the dialog appears in the Add flow.

## Requirements

### Requirement: Add flow gating

The system SHALL read the clipboard via `tauri-plugin-clipboard-manager::readText()` when the Add button is pressed — the only clipboard path that works reliably on Wayland/KDE Plasma 6 (WebKit's `navigator.clipboard` is permission-gated; the Rust-side `arboard` crate fails silently with `ContentNotAvailable`). A clipboard read failure SHALL surface a red "Не удалось прочитать буфер обмена" notification.

When `config.preview_dialog_enabled` is `true` (the default in `storage::schema::UIConfig::default`), the system SHALL open `PreviewDialog` with the clipboard text; otherwise it SHALL call `commands.addTextEntry(text, true)` directly, skipping the dialog. `AppShell` SHALL load `UIConfig` once per mount for this decision and treat a config load failure as "dialog disabled".

#### Scenario: Dialog opens when enabled

- GIVEN `preview_dialog_enabled` is `true` and the clipboard contains text
- WHEN the user clicks Add
- THEN the preview dialog opens pre-filled with the clipboard text and no queue entry is created yet

#### Scenario: Direct add when disabled

- GIVEN `preview_dialog_enabled` is `false`
- WHEN the user clicks Add
- THEN `commands.addTextEntry(text, true)` is called immediately without opening the dialog

### Requirement: Floating non-modal window

The system SHALL implement the dialog as a floating, non-modal window via `react-rnd` inside a Mantine `Portal`, so it sits above the app (z-index `--ruvox-preview-z`, below Mantine notifications) without blocking the UI underneath.

The `Rnd` element SHALL be anchored inside a viewport-sized fixed container (`.viewportContainer`, `position: fixed; inset: 0; pointer-events: none`) so that its `(x, y)` coordinates are viewport coordinates regardless of document scroll. The window SHALL be draggable by its header ("Предпросмотр нормализации") and resizable from any edge or corner, with a minimum size of 560×380, an initial size of 900×620, centered on every open.

Pressing ESC SHALL close the dialog, equivalent to Cancel. The header close button SHALL behave the same way.

#### Scenario: Geometry resets on each open

- GIVEN the user previously moved and resized the dialog, then closed it
- WHEN the dialog is opened again
- THEN it reappears centered at 900×620, with edit mode off, "Read Now" on, and both checkboxes unchecked

#### Scenario: ESC cancels

- GIVEN the dialog is open
- WHEN the user presses ESC
- THEN the dialog closes and no entry is added to the queue

#### Scenario: Underlying UI stays interactive

- GIVEN the dialog is open over the main window
- WHEN the user clicks outside the dialog panel
- THEN the click reaches the underlying application (the container is click-through; only the panel itself captures pointer events)

### Requirement: Side-by-side panes with live normalization

The dialog body SHALL show two panes: "Оригинал" (left) and "После нормализации" (right).

The left pane SHALL be a read-only scrollable `<pre>` by default; clicking "Редактировать" switches it to a `<Textarea>` with the current text. Every change to the text under consideration SHALL trigger re-normalization after a 1000 ms debounce.

The right pane SHALL show the normalized result from `commands.previewNormalize(text)`; while a normalization request is in flight it SHALL show a `<Loader>`, and on failure it SHALL show `"(ошибка нормализации: ...)"` inline. When the text is empty or whitespace-only, the right pane SHALL be empty and no request SHALL be issued.

#### Scenario: Edit re-normalizes with debounce

- GIVEN the dialog is open in edit mode
- WHEN the user types into the textarea and pauses for 1 second
- THEN `commands.previewNormalize` is called once with the edited text and the right pane updates with the result

#### Scenario: Normalization error is shown inline

- GIVEN the dialog is open
- WHEN `commands.previewNormalize` rejects
- THEN the right pane shows `"(ошибка нормализации: <reason>)"` instead of a result

### Requirement: Footer controls

The dialog footer SHALL contain:

| Control | Behavior |
|---------|----------|
| "Больше не показывать этот диалог" (Checkbox) | On synthesis, persists `preview_dialog_enabled: false` via `commands.updateConfig` (no threshold; disables the dialog globally) |
| "Синхронный скроллинг" (Checkbox) | Mirrors scrolling between the two panes by relative position, with ping-pong protection via `syncingRef` |
| "Read Now" (Switch, default ON) | Passed as `playWhenReady` to `addTextEntry`; ON plays after `ready`, OFF only enqueues |
| "Отмена" (Button) | Closes the dialog without adding anything |
| "Редактировать" (Button) | Switches the left pane to edit mode; hidden while editing |
| "Синтезировать" (Button) | Confirms and synthesizes; disabled while normalization is loading |

#### Scenario: Synchronized scrolling mirrors position

- GIVEN "Синхронный скроллинг" is checked and both panes overflow
- WHEN the user scrolls the left pane to 50% of its range
- THEN the right pane scrolls to 50% of its own range, without echoing a scroll event back

### Requirement: Synthesis confirmation

When "Синтезировать" is pressed, the system SHALL:

1. Use `editedText.trim()` when in edit mode, otherwise the original clipboard text; an empty edited result MUST fall back to the original text.
2. If "Больше не показывать этот диалог" is checked, call `commands.updateConfig({ preview_dialog_enabled: false })` and update the cached config in `AppShell`.
3. Close the dialog and call `commands.addTextEntry(finalText, playWhenReady)`, selecting the new entry and showing a confirmation notification.

The preview dialog SHALL be the only place in the UI where the user can edit source text before synthesis; after confirmation the text is stored as the entry's immutable `original_text`.

#### Scenario: Edited text is synthesized

- GIVEN the user edited the text and left "Read Now" ON
- WHEN the user clicks "Синтезировать"
- THEN the dialog closes, `addTextEntry` receives the trimmed edited text with `playWhenReady = true`, and the new entry becomes selected

#### Scenario: Opt-out persists

- GIVEN "Больше не показывать этот диалог" is checked
- WHEN the user clicks "Синтезировать"
- THEN `commands.updateConfig({ preview_dialog_enabled: false })` is issued, and the next Add click skips the dialog

### Requirement: preview_normalize backend command

The system SHALL expose a Tauri command `preview_normalize` (`src-tauri/src/commands/mod.rs`) that runs the normalization pipeline on the given text inside `tokio::task::spawn_blocking` (the pipeline is CPU-bound and must not block the tokio reactor) and returns `PreviewNormalizeResult { normalized }`.

```rust
#[derive(Debug, Serialize)]
pub struct PreviewNormalizeResult {
    pub normalized: String,
}
```

The command MUST NOT touch storage: previewing creates no `TextEntry`, so preview drafts never appear in the queue. The character mapping returned by `process_with_char_mapping` SHALL be discarded — the dialog needs only the result string.

#### Scenario: Preview without side effects

- GIVEN the dialog requested a preview for some text
- WHEN `preview_normalize` completes
- THEN the response contains only the normalized string, and the history file and audio cache are unchanged

#### Scenario: Frontend contract

- GIVEN the frontend wrapper `commands.previewNormalize(text)` in `src/lib/tauri.ts`
- WHEN it invokes the backend
- THEN it calls `tauriInvoke('preview_normalize', { text })` and resolves to `PreviewNormalizeResult`

### Requirement: preview_dialog_enabled configuration

The system SHALL store the dialog gate as `preview_dialog_enabled: boolean` in `UIConfig`, defaulting to `true`. It SHALL be changeable in two ways: the Settings dialog toggle "Показывать диалог предпросмотра перед синтезом", and the in-dialog "Больше не показывать этот диалог" checkbox (which only ever sets it to `false`; re-enabling requires the Settings toggle).

#### Scenario: Restore via Settings

- GIVEN the dialog was disabled via its own checkbox
- WHEN the user enables "Показывать диалог предпросмотра перед синтезом" in Settings and saves
- THEN the next Add click opens the preview dialog again
