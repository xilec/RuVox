# Use cases

## 1. Adding text from the clipboard

**Goal:** listen to copied text.

**Steps:**
1. Copy the text to the clipboard (`Ctrl+C` in the source application).
2. Open RuVox (or restore it from the system tray).
3. Press the **Add** button on the right side of the header.

**What happens:**
- The text is read from the clipboard via `tauri-plugin-clipboard-manager` (works reliably on Wayland/KDE).
- If the preview dialog is enabled (FF 1.1, on by default) — a floating window with the original and normalized version opens. See [preview-dialog.md](preview-dialog.md).
- Otherwise the text goes straight into the pipeline → the entry lands in the queue with status `pending` → `processing` → `ready`.
- If the **Read Now** switch in the preview dialog is on (default) — playback starts automatically once the entry reaches `ready`.

## 2. Entry queue

All added entries are shown in the navbar on the left. For each entry:

- **Preview** — first 60 characters of the original text.
- **Status badge** — `pending` / `processing` / `ready` / `playing` / `error` (Russian labels, color-coded).
- **Duration** — `MM:SS` after successful synthesis.
- **Buttons** — Play and Delete (with confirmation).

**Sorting:** newest entries on top (`created_at` desc).

**Resizing the queue:** drag the navbar's right border (drag-to-resize, minimum 180 px, maximum 70% of window width).

## 3. Playback control

The player sits at the bottom of the header (under the strip with the Add button).

**Hotkeys (with focus in the application window):**

| Key | Action |
|-----|--------|
| `Space` | Play / Pause |
| `←` | Seek −5 seconds |
| `→` | Seek +5 seconds |

**Mouse control:**

- Play/Pause button.
- Position slider — click / drag → seek.
- Speed field (0.5x–2.0x) — `NumberInput` with step 0.1; mouse wheel over the field changes speed.
- Volume slider (0–100%).
- Prev/Next — switch between entries in the queue (only shown when the queue has entries).

> **Global hotkeys** are not implemented — this is an intentional decision (see RewriteNotes.md § 7). The player's hotkeys only work while the RuVox window is focused.

## 4. Plain / Markdown / HTML modes

The `TextViewer` has a `SegmentedControl` on the right for switching the display mode.

**Plain Text:**
- Original text as-is, in a monospace font.
- Markdown markup is visible: `**bold**`, `# Header`.
- Word highlight works directly on positions.

**Markdown:**
- Rendered via `markdown-it` with code highlighting.
- Each inline-text token is wrapped in `<span data-orig-start data-orig-end>` for highlighting.
- Supports: headings, bold/italic, inline code, code blocks, links, lists, tables, mermaid blocks.

**HTML:**
- Sanitized via `DOMPurify`.
- Used when copying HTML from the browser.
- Word highlighting in HTML mode currently doesn't work (see follow-up in `task_history.md` U5).

Switching between modes is instant; highlighting is restored automatically.

## 5. Word highlighting during playback

When playback starts, `TextViewer` loads `WordTimestamp[]` via `commands.getTimestamps(entry_id)`. On every `playback_position` tick a binary search finds the active word and the corresponding span gets the `.word-highlight` CSS class.

- Highlighting works in **plain** and **markdown** modes (HTML — not yet).
- If the word goes out of the viewport — `scrollIntoView`.
- Highlight color — light-dark (`rgba(255,213,0,0.45)` / `rgba(255,213,0,0.3)`), 80 ms transition.

When the mode is switched / a different entry is selected, highlighting is cleared; the new render re-subscribes to playback events.

## 6. Mermaid diagrams

In **markdown** mode, ` ```mermaid ... ``` ` blocks are rendered through `mermaid.js` as inline SVG. The mermaid theme is synced with the current color scheme (light/dark).

**In TTS** the diagram is replaced with a marker:

```markdown
## Архитектура

​```mermaid
graph TD
  A[Client] --> B[Server]
​```

Клиент обращается к серверу.
```

→ voicing: «Архитектура. Тут мермэйд диаграмма. Клиент обращается к серверу.»

You can pause and inspect the diagram in the UI.

In **plain** mode the mermaid block is shown as source code.

## 7. Technical text

**Example input:**

```markdown
## Установка

1. Запустите `pip install package`
2. Версия должна быть >= 2.0.0
3. API доступен на https://api.example.com
```

**Voiced as:**

> Установка.
> Первое: Запустите пип инсталл пэкэдж.
> Второе: Версия должна быть больше или равно два точка ноль точка ноль.
> Третье: Эй пи ай доступен на эйч ти ти пи эс двоеточие слэш слэш api точка example точка ком.

**What got processed:**

- camelCase / snake_case / kebab-case → split and transliterated (`getUserData` → «гет юзер дата»)
- Abbreviations → dictionary or letter by letter (`API` → «эй пи ай», `JSON` → «джейсон»)
- Numbers → numerals (`123` → «сто двадцать три», `2.0` → «два точка ноль»)
- URLs → spelled out word by word (`https://...` → «эйч ти ти пи эс двоеточие слэш слэш...»)
- Email → «собака» instead of `@` (`user@mail.com` → «user собака mail точка ком»)
- Operators → spoken description (`>=` → «больше или равно», `->` → «стрелка»)
- Markdown links `[text](url)` → only the link text is read, the URL is dropped
- Markdown numbered lists `1.`/`2.` → «первое:»/«второе:»

See [pipeline.md](pipeline.md) for the full description.

## 8. System tray

When the close button (`X`) is clicked, the app **hides into the tray** instead of exiting. This lets the background process keep working (synthesis, playback) and lets you return to the window quickly.

**Tray menu:**

| Item | Action |
|------|--------|
| Открыть окно | Show the main window + re-init mpv (libmpv is recreated on close, so it has to be brought back on show) |
| Добавить | Read the clipboard + put the entry in the queue |
| Выход | Actually exit the application |

**Clicking the tray icon** (Linux/KDE/GNOME via libayatana-appindicator) — opens the context menu. A double click / single click also shows the window (the plugin doesn't handle every click correctly).

## 9. Settings

`Settings` is a modal dialog in the corner of the header (the cog icon).

**Available options:**

- **Speech synthesis:** speaker (`xenia` / `aidar` / `baya` / `kseniya` / `eugene` / `random`), sample rate (8000 / 24000 / 48000).
- **Notifications:** notify_on_ready, notify_on_error.
- **Cache:** `max_cache_size_mb` (minimum 100), `auto_cleanup_days` (0 = disabled).
- **Interface:** theme (light / dark / auto).

**There are no global hotkeys in the UI** — this is an intentional decision (see RewriteNotes.md § 7). The player's hotkeys only work while the RuVox window is focused.

Changes are saved via `commands.updateConfig(patch)` → `~/.cache/ruvox/config.json`.
