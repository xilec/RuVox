# UI

RuVox frontend — React 18 + TypeScript 5 (strict) + Mantine 8, Vite, pnpm. Source: `src/`.

## Window structure

```
┌─────────────────────────────────────────────────────────────┐
│ Header (74 px)                                              │
│ [⌬] ▶ ━━━━━●━━━━ 01:23 / 03:45  [1.0x]  [██ Vol]  [⚙]   │
├──────────────────┬──────────────────────────────────────────┤
│ Navbar (resize)  │ Main                                     │
│ Очередь   [Add]  │ ┌────────────────────────────────────┐   │
│  ▶ Текст 1       │ │ TextViewer                         │   │
│    Текст 2       │ │  [Plain | Markdown | HTML]   [✏]   │   │
│    Текст 3       │ │                                    │   │
│                  │ │ Текст с **подсветкой**             │   │
│                  │ │ читаемого слова.                   │   │
│                  │ └────────────────────────────────────┘   │
└──────────────────┴──────────────────────────────────────────┘
```

- **Header height 74 px** — contains only `<Player />`. The height intentionally has extra room at the top: `Player.module.css` sets asymmetric padding (top 18 / bottom 8) so the floating slider `label` (for position and volume) doesn't escape the webview and get clipped by the native window title bar, and so the large brand logo (48 px, noticeably bigger than the Play button's `ActionIcon size="md"`) fits.
- **Brand icon** — the first element of the Player, to the left of the Play button (`IconAppLogo` from `components/icons.tsx`, the same geometry as in `src-tauri/icons/source.svg`). The color is `violet` via `light-dark(violet-7, violet-5)`. Mantine reserves blue for interactive elements (buttons, sliders), so the logo is intentionally given a different accent so it reads as "brand" rather than "clickable".
- **The application title and icon in the system title bar** are set by `tauri.conf.json` (`windows[0].title = "RuVox"`) and `src-tauri/src/lib.rs::run` (via `window.set_icon(...)` from `icons/128x128.png`). On Linux/Wayland without an installed `.desktop` file the WM may show the default icon in alt-tab — this is fixed after `cargo tauri build` + bundle install. The icon source is `src-tauri/icons/source.svg`, rendered via `rsvg-convert` to 32×32 / 128×128 / 128×128@2x / tray 22×22.
- **Add button** — in the navbar to the right of the "Очередь" title (size `xs`).
- **Settings cog** — the last element of the Player, after the volume slider. The application theme is configured only through the Settings dialog (the "Тема оформления" field).
- **Navbar drag-to-resize** — the right border of the navbar is an invisible 6 px strip with `cursor: col-resize`. Drag changes `navWidth` (180 px minimum, 70% of window width maximum).
- **Main** — `flex: 1` column, takes the entire available height under the header.

## File structure

```
src/
├── App.tsx                       # MantineProvider, Notifications, ModalsProvider
├── main.tsx                      # createRoot + import '@mantine/core/styles.css'
├── components/
│   ├── AppShell.tsx              # MantineAppShell + Header/Navbar/Main + preview flow
│   ├── QueueList.tsx             # TextEntry list (Zustand selectedEntry store)
│   ├── Player.tsx                # tauri-plugin-mpv wrapper + Space/←/→ hotkeys
│   ├── TextViewer.tsx            # plain/markdown/html + word highlight (read-only)
│   ├── icons.tsx                 # Inline SVG icons (Play/Pause/Settings)
│   ├── *.module.css              # CSS Modules per component
├── dialogs/
│   ├── PreviewDialog.tsx         # FF 1.1: floating preview window (react-rnd)
│   ├── PreviewDialog.module.css
│   └── Settings.tsx              # @mantine/form + commands.updateConfig
├── lib/
│   ├── tauri.ts                  # Typed wrappers for invoke + listen
│   ├── markdown.ts               # markdown-it + data-orig-* spans
│   ├── html.ts                   # DOMPurify + text extraction
│   ├── mermaid.ts                # mermaid.js init + run
│   ├── wordHighlight.ts          # Binary search + DOM highlight application
│   ├── wordSpans.ts              # Utilities for data-orig-start/end attributes
│   ├── notificationBridge.ts     # listen → notifications.show
│   └── errors.ts                 # formatError for Tauri CommandError
└── stores/
    └── selectedEntry.ts          # Zustand: selectedId, selectedEntry, setSelectedEntry
```

## Key components

### AppShell

`src/components/AppShell.tsx` — root layout.

**Structure:**
- `MantineAppShell` with `header={ height: 74 }`, `navbar={ width: navWidth, breakpoint: 'sm' }`.
- Header: only `<Player onOpenSettings={…} />`. The "RuVox" title is shown in the system title bar.
- Navbar: `Group` (`Title` "Очередь" + `Button` "Add") → `<QueueList />` → drag handle with pointer events.
- Main: `<TextViewer entry={selectedEntry} />`.
- Additionally, `<PreviewDialog>` and `<SettingsModal>` are mounted via Portal.

**Add flow:**
1. Read the clipboard via `tauri-plugin-clipboard-manager::readText()` — the only way that reliably works on Wayland/KDE Plasma 6 (WebKit's `navigator.clipboard` is restricted, Rust-side `arboard` silently fails with `ContentNotAvailable`).
2. If `config.preview_dialog_enabled = true` (default) — `<PreviewDialog>` is opened.
3. Otherwise call `commands.addTextEntry(text, true)` directly → entry is added to the queue.

### QueueList

`src/components/QueueList.tsx` — entry list.

- Loading: `commands.getEntries()` on mount, sort `created_at` desc.
- Listen to `events.entryUpdated` — in-place update (prepend new ones, indexed replace existing ones).
- Item: preview (60 chars) + status badge + duration + Play/Delete buttons.
- Click → `setSelectedEntry` in the Zustand store.
- Delete via `@mantine/modals::openConfirmModal`.
- ScrollArea + opacity-on-hover for actions.

### Player

`src/components/Player.tsx` — wrapper around the player Tauri commands.

- Listens to `events.playbackStarted/Paused/Stopped/Finished/Position` to sync UI state.
- Hotkeys via `@mantine/hooks::useHotkeys`: `Space`, `←` (−5), `→` (+5).
- Position slider — optimistic update during drag (`draggingRef`), seek on `onChangeEnd`.
- Mouse wheel over the speed `NumberInput` — native wheel listener (not React `onWheel`) with `passive: false` for correct `preventDefault`.
- Prev/Next — navigation through `entryIds` (props).
- Optional `onOpenSettings: () => void` prop — renders the settings cog after the volume slider (if provided); used by AppShell to open `<SettingsModal>`.

### TextViewer

`src/components/TextViewer.tsx` — read-only display of `original_text`.

- Three modes: plain / markdown / html (`SegmentedControl`).
- Markdown: `markdown-it` with a custom `text` rule — wraps inline tokens in `<span data-orig-start data-orig-end>` for highlighting. Mermaid blocks → `<div class="mermaid">` + `mermaid.run()`.
- HTML: `DOMPurify.sanitize()` + code highlighting (highlight.js).
- Word highlight: useEffect subscribes to playback events; `useRef` (timestamps, playingEntryId, activeIdx) caches without re-renders, binary search + `applyHighlight` on the DOM.
- Editing the text of an existing entry in the UI **is not supported** — to make a correction you have to delete the entry and add it again through the preview dialog.

### Settings

`src/dialogs/Settings.tsx` — modal configuration dialog.

- `@mantine/form::useForm` with initialValues / validation.
- Loading: `useEffect([opened])` → `commands.getConfig()` → `form.setValues`.
- Submit → builds a `UIConfigPatch` (only the form fields; the rest of `UIConfig` is left alone) → `commands.updateConfig(patch)` → notification.
- Fields: speaker, sample_rate, notify_on_ready/error, max_cache_size_mb, theme, preview_dialog_enabled.
- Cache cleanup: a "Очистить кэш…" button opens a sub-modal (`CleanupCacheModal`) with a target-MB input, a "Удалять тексты" checkbox, and a "Очистить полностью" checkbox (which disables the input). Confirms via `commands.clearCache({ mode, delete_texts })`.

### PreviewDialog

See [preview-dialog.md](preview-dialog.md). Floating non-modal window via `react-rnd` + Mantine Portal.

## Styling

- **CSS Modules** (`*.module.css`) + the `classNames` prop. No `sx`, `createStyles`, or emotion.
- Mantine CSS variables (`--mantine-spacing-*`, `--mantine-color-default-hover`, `--mantine-radius-sm`).
- Light/dark via the native CSS `light-dark()` (for the `.word-highlight` background).
- Global CSS — only `@mantine/core/styles.css` in `main.tsx`.

## State management

- **Zustand** (`src/stores/selectedEntry.ts`) — global state of the selected entry. Minimum: `selectedId`, `selectedEntry`, `setSelectedId`, `setSelectedEntry`.
- **`useState`** — local component state.
- **No Redux / React Query** — Tauri invoke + `useEffect` is enough.

## IPC typing

`src/lib/tauri.ts` — typed `commands` and `events` wrappers:

```typescript
export const commands = {
  addClipboardEntry: (play_when_ready: boolean): Promise<EntryId> =>
    tauriInvoke('add_clipboard_entry', { playWhenReady: play_when_ready }),
  addTextEntry: (text: string, play_when_ready: boolean): Promise<EntryId> =>
    tauriInvoke('add_text_entry', { text, playWhenReady: play_when_ready }),
  // ...
};

export const events = {
  entryUpdated: (handler: (p: { entry: TextEntry }) => void) =>
    listen<{ entry: TextEntry }>('entry_updated', (e) => handler(e.payload)),
  // ...
};
```

> **Important camelCase nuance:** Tauri 2's `invoke()` accepts parameters in **camelCase** on the JS side, even if the Rust handler is declared in snake_case. Visible in code: `tauriInvoke('seek_to', { positionSec: position_sec })` — the function takes snake_case but passes a camelCase key into invoke. See fix `b420d6e fix(ipc): use camelCase param names for Tauri invoke`.

## Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| `react`, `react-dom` | 18.x | UI runtime |
| `@mantine/core` | 8.x | UI components |
| `@mantine/hooks` | 8.x | useHotkeys etc. |
| `@mantine/form` | 8.x | Settings form |
| `@mantine/notifications` | 8.x | Toasts |
| `@mantine/modals` | 8.x | Confirm modals |
| `zustand` | 5.x | Selected entry store |
| `markdown-it` | — | Markdown rendering |
| `mermaid` | — | Mermaid diagrams |
| `dompurify` | — | HTML sanitize |
| `highlight.js` | — | Code highlighting |
| `react-rnd` | — | PreviewDialog drag/resize |
| `@tauri-apps/api` | 2.x | invoke / listen |
| `@tauri-apps/plugin-clipboard-manager` | 2.x | Clipboard reading |
| `vite` | — | Dev server / bundler |
| `typescript` | 5.x | strict mode |

See `package.json` for current versions.
