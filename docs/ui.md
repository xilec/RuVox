# UI

Frontend RuVox — React 18 + TypeScript 5 (strict) + Mantine 8, Vite, pnpm. Источник: `src/`.

## Структура окна

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

- **Header высота 74 px** — содержит только `<Player />`. Высота нарочно с запасом наверху: `Player.module.css` выставляет асимметричный padding (top 18 / bottom 8), чтобы всплывающий `label` ползунков (позиции и громкости) не уезжал за пределы webview и не обрезался native title-bar окна, и чтобы помещался крупный брендовый логотип (48 px, заметно крупнее `ActionIcon size="md"` Play-кнопки).
- **Брендовая иконка** — первый элемент Player'а слева от Play-кнопки (`IconAppLogo` из `components/icons.tsx`, та же геометрия, что и в `src-tauri/icons/source.svg`). Цвет — `violet` через `light-dark(violet-7, violet-5)`. Синий зарезервирован Mantine'ом за интерактивными элементами (кнопки, ползунки), поэтому логотипу намеренно дан другой акцент, чтобы он читался как «бренд», а не как «кликабельно».
- **Заголовок и иконка приложения в системной title-bar** окна задаются `tauri.conf.json` (`windows[0].title = "RuVox"`) и `src-tauri/src/lib.rs::run` (через `window.set_icon(...)` из `icons/128x128.png`). На Linux/Wayland без установленного `.desktop` файла WM может отображать дефолтную иконку в alt-tab — это исправляется после `cargo tauri build` + bundle install. Источник иконок — `src-tauri/icons/source.svg`, рендерится через `rsvg-convert` в 32×32 / 128×128 / 128×128@2x / tray 22×22.
- **Кнопка Add** — в Navbar справа от заголовка «Очередь» (size `xs`).
- **Шестерёнка настроек** — последний элемент Player'а, после ползунка громкости. Тема приложения настраивается только через диалог Settings (поле «Тема оформления»).
- **Navbar drag-to-resize** — правая граница navbar — невидимая полоса 6 px шириной с `cursor: col-resize`. Drag меняет `navWidth` (180 px минимум, 70% ширины окна максимум).
- **Main** — `flex: 1` колонка, занимает всю доступную высоту под header.

## Структура файлов

```
src/
├── App.tsx                       # MantineProvider, Notifications, ModalsProvider
├── main.tsx                      # createRoot + import '@mantine/core/styles.css'
├── components/
│   ├── AppShell.tsx              # MantineAppShell + Header/Navbar/Main + preview-флоу
│   ├── QueueList.tsx             # Список TextEntry (Zustand selectedEntry store)
│   ├── Player.tsx                # tauri-plugin-mpv обёртка + хоткеи Space/←/→
│   ├── TextViewer.tsx            # plain/markdown/html + word highlight (read-only)
│   ├── icons.tsx                 # Inline SVG иконки (Play/Pause/Settings)
│   ├── *.module.css              # CSS Modules per component
├── dialogs/
│   ├── PreviewDialog.tsx         # FF 1.1: floating preview window (react-rnd)
│   ├── PreviewDialog.module.css
│   └── Settings.tsx              # @mantine/form + commands.updateConfig
├── lib/
│   ├── tauri.ts                  # Типизированные обёртки invoke + listen
│   ├── markdown.ts               # markdown-it + data-orig-* spans
│   ├── html.ts                   # DOMPurify + извлечение текста
│   ├── mermaid.ts                # mermaid.js init + run
│   ├── wordHighlight.ts          # Бинарный поиск + DOM-применение подсветки
│   ├── wordSpans.ts              # Утилиты для data-orig-start/end атрибутов
│   ├── notificationBridge.ts     # listen → notifications.show
│   └── errors.ts                 # formatError для Tauri CommandError
└── stores/
    └── selectedEntry.ts          # Zustand: selectedId, selectedEntry, setSelectedEntry
```

## Ключевые компоненты

### AppShell

`src/components/AppShell.tsx` — корневой layout.

**Структура:**
- `MantineAppShell` с `header={ height: 74 }`, `navbar={ width: navWidth, breakpoint: 'sm' }`.
- Header: только `<Player onOpenSettings={…} />`. Заголовок «RuVox» отображается системной title-bar окна.
- Navbar: `Group` (`Title` «Очередь» + `Button` «Add») → `<QueueList />` → drag-handle с pointer events.
- Main: `<TextViewer entry={selectedEntry} />`.
- Кроме того, mount `<PreviewDialog>` и `<SettingsModal>` через Portal.

**Add flow:**
1. Чтение буфера через `tauri-plugin-clipboard-manager::readText()` — единственный способ, надёжно работающий на Wayland/KDE Plasma 6 (WebKit `navigator.clipboard` ограничен, Rust-side `arboard` молча падает с `ContentNotAvailable`).
2. Если `config.preview_dialog_enabled = true` (по умолчанию) — открывается `<PreviewDialog>`.
3. Иначе сразу `commands.addTextEntry(text, true)` → запись добавляется в очередь.

### QueueList

`src/components/QueueList.tsx` — список записей.

- Загрузка: `commands.getEntries()` на mount, sort `created_at` desc.
- Listen на `events.entryUpdated` — in-place update (prepend новых, indexed replace существующих).
- Item: preview (60 char) + статус-бейдж + duration + Play/Delete кнопки.
- Click → `setSelectedEntry` в Zustand-store.
- Delete через `@mantine/modals::openConfirmModal`.
- ScrollArea + opacity-on-hover для actions.

### Player

`src/components/Player.tsx` — обёртка над Tauri-командами плеера.

- Слушает `events.playbackStarted/Paused/Stopped/Finished/Position` для синхронизации UI-state.
- Хоткеи через `@mantine/hooks::useHotkeys`: `Space`, `←` (−5), `→` (+5).
- Slider позиции — оптимистичный update во время drag (`draggingRef`), seek на `onChangeEnd`.
- Колесо мыши над `NumberInput` скорости — нативный wheel-listener (не React onWheel) с `passive: false` для корректного `preventDefault`.
- Prev/Next — навигация по `entryIds` (props).
- Опциональный prop `onOpenSettings: () => void` — рендерит шестерёнку настроек после ползунка громкости (если передан); используется AppShell для открытия `<SettingsModal>`.

### TextViewer

`src/components/TextViewer.tsx` — read-only отображение `original_text`.

- Три режима: plain / markdown / html (`SegmentedControl`).
- Markdown: `markdown-it` с custom `text` rule — оборачивает inline-токены в `<span data-orig-start data-orig-end>` для подсветки. Mermaid-блоки → `<div class="mermaid">` + `mermaid.run()`.
- HTML: `DOMPurify.sanitize()` + подсветка кода (highlight.js).
- Word highlight: useEffect подписывается на playback-events, `useRef` (timestamps, playingEntryId, activeIdx) кэшируют без re-renders, бинарный поиск + `applyHighlight` на DOM.
- Правка текста существующей записи в UI **не предусмотрена** — для коррекции нужно удалить запись и добавить заново через preview-диалог.

### Settings

`src/dialogs/Settings.tsx` — модальный диалог конфигурации.

- `@mantine/form::useForm` с initialValues / validation.
- Loading: `useEffect([opened])` → `commands.getConfig()` → `form.setValues`.
- Submit → builds `UIConfigPatch` (только поля формы, остальные `UIConfig` поля не трогаются) → `commands.updateConfig(patch)` → нотификация.
- Поля: speaker, sample_rate, notify_on_ready/error, max_cache_size_mb, auto_cleanup_days, theme, preview_dialog_enabled.

### PreviewDialog

См. [preview-dialog.md](preview-dialog.md). Floating non-modal window через `react-rnd` + Mantine Portal.

## Стилизация

- **CSS Modules** (`*.module.css`) + prop `classNames`. Никаких `sx`, `createStyles`, emotion.
- Mantine CSS-переменные (`--mantine-spacing-*`, `--mantine-color-default-hover`, `--mantine-radius-sm`).
- Light-dark через нативный CSS `light-dark()` (для `.word-highlight` фон).
- Глобальный CSS — только `@mantine/core/styles.css` в `main.tsx`.

## State management

- **Zustand** (`src/stores/selectedEntry.ts`) — глобальное состояние выбранной записи. Минимум: `selectedId`, `selectedEntry`, `setSelectedId`, `setSelectedEntry`.
- **`useState`** — локальное состояние компонентов.
- **Без Redux / React Query** — Tauri invoke + `useEffect` справляется.

## Типизация IPC

`src/lib/tauri.ts` — типизированные обёртки `commands` и `events`:

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

> **Важный нюанс camelCase:** Tauri 2 при `invoke()` принимает параметры **в camelCase** на JS-стороне, даже если Rust handler объявлен в snake_case. Видно в коде: `tauriInvoke('seek_to', { positionSec: position_sec })` — функция принимает snake_case, но передаёт в invoke camelCase ключ. См. фикс `b420d6e fix(ipc): use camelCase param names for Tauri invoke`.

## Зависимости

| Пакет | Версия | Назначение |
|-------|--------|------------|
| `react`, `react-dom` | 18.x | UI runtime |
| `@mantine/core` | 8.x | UI компоненты |
| `@mantine/hooks` | 8.x | useHotkeys и др. |
| `@mantine/form` | 8.x | Settings form |
| `@mantine/notifications` | 8.x | Тосты |
| `@mantine/modals` | 8.x | Confirm-модалки |
| `zustand` | 5.x | Selected entry store |
| `markdown-it` | — | Markdown rendering |
| `mermaid` | — | Mermaid diagrams |
| `dompurify` | — | HTML sanitize |
| `highlight.js` | — | Code highlighting |
| `react-rnd` | — | PreviewDialog drag/resize |
| `@tauri-apps/api` | 2.x | invoke / listen |
| `@tauri-apps/plugin-clipboard-manager` | 2.x | Чтение буфера |
| `vite` | — | Dev server / bundler |
| `typescript` | 5.x | strict mode |

См. `package.json` для актуальных версий.
