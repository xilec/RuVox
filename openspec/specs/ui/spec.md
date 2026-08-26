# UI Specification

## Purpose

Covers the structure and behavior of the RuVox frontend (`src/`): the application shell layout (header with the player, resizable navbar with the queue, main text viewer), the key components (`AppShell`, `QueueList`, `Player`, `TextViewer`, `Settings`), global state management, and the design-token system (`src/theme.ts` + `--ruvox-*` custom properties in `src/globals.css`). The frontend is React 18 + TypeScript (strict) + Mantine 8, bundled with Vite inside a Tauri 2 webview.

## Requirements

### Requirement: Application shell layout

The system SHALL render the root layout via `MantineAppShell` in `src/components/AppShell.tsx` with a fixed 74 px header containing only the `Player` component, a navbar containing the queue, and a main area rendering `TextViewer` for the selected entry.

The header height of 74 px with asymmetric padding (top 18 / bottom 8 in `Player.module.css`) MUST be preserved: the floating slider labels (position and volume) must not be clipped by the native window title bar, and the 48 px brand logo must fit next to the Play button.

The navbar SHALL be drag-resizable via an invisible 6 px strip on its right border with `cursor: col-resize`; the width MUST be clamped to a minimum of 180 px and a maximum of 70% of the window width.

The navbar SHALL contain a "Очередь" title with an "Add" button (size `xs`) to its right, a search input filtering the queue, and the `QueueList` component. The `PreviewDialog` and `SettingsModal` SHALL be mounted from `AppShell`.

#### Scenario: Window renders the three regions

- GIVEN the application has started
- WHEN the main window is displayed
- THEN the header shows only the player controls, the navbar shows "Очередь" with an Add button, a search field and the entry list, and the main area shows the selected entry's text (or a "Нет выбранной записи" placeholder)

#### Scenario: Navbar resize respects bounds

- GIVEN the application window is 1000 px wide
- WHEN the user drags the navbar's right border far to the left
- THEN the navbar width stops at 180 px
- AND when the user drags it far to the right the width stops at 700 px (70% of the window width)

#### Scenario: Search hotkey focuses the queue filter

- GIVEN the application window is focused
- WHEN the user presses Ctrl+F (or Cmd+F)
- THEN focus moves to the "Поиск по записям" input and the webview's built-in find-in-page is suppressed

### Requirement: Brand mark and accent discipline

The system SHALL display the brand logo (`IconAppLogo` from `src/components/icons.tsx`, same geometry as `src-tauri/icons/source.svg`) as the first element of the Player, to the left of the Play button, at 48 px, colored via `var(--ruvox-brand)` (violet `#9d4edd`).

Blue MUST be reserved for interactive affordances (buttons, sliders, links); the violet brand color MUST NOT be used on clickable elements. The `--ruvox-brand` token MUST stay in sync with `src-tauri/icons/source.svg`.

The application title and window icon SHALL be set by `tauri.conf.json` (`windows[0].title = "RuVox"`) and `window.set_icon(...)` in `src-tauri/src/lib.rs::run`.

#### Scenario: Logo renders as brand, not action

- GIVEN the main window is displayed
- WHEN the user looks at the header
- THEN the leftmost element is the violet 48 px logo, and it carries no click handler

### Requirement: Queue list behavior

The system SHALL load entries on mount via `commands.getEntries()` sorted by `created_at` descending, and SHALL keep the list in sync by listening to `events.entryUpdated` (prepend new entries, replace existing ones in place) and `events.entryRemoved` (remove and clear the selection if the selected entry was deleted).

Each queue item SHALL show a 60-character preview of `original_text`, a status badge (Ожидание/Обработка/Готово/Играет/Ошибка), the duration when available, and a Play action enabled only for `ready`/`playing` entries. Clicking an item SHALL store it as the selected entry in the Zustand `selectedEntry` store.

Right-clicking an item SHALL open a context menu with "Воспроизвести", "Перегенерировать аудио" and "Удалить"; deletion MUST be confirmed via `modals.openConfirmModal` before calling `commands.deleteEntry`.

The navbar search input SHALL filter entries case-insensitively by `original_text` substring; when the playing entry is scrolled out of view, a floating "К читаемому" button SHALL appear that selects and scrolls to the playing entry.

#### Scenario: New entry appears at the top

- GIVEN the queue shows existing entries
- WHEN an `entry_updated` event arrives with an entry id not in the list
- THEN the entry is prepended and the list remains sorted by `created_at` descending

#### Scenario: Delete requires confirmation

- GIVEN a queue item's context menu is open
- WHEN the user clicks "Удалить"
- THEN a confirmation modal appears, and only after confirming does the system call `commands.deleteEntry` and remove the item

#### Scenario: Search filters the list

- GIVEN the queue contains entries
- WHEN the user types into "Поиск по записям"
- THEN only entries whose `original_text` contains the query (case-insensitive) remain visible, or "Ничего не найдено" is shown when none match

### Requirement: Player controls

The system SHALL provide playback controls in `src/components/Player.tsx` as a wrapper over the player Tauri commands: Play/Pause, a position slider, a time display (`mm:ss / mm:ss`), a speed `NumberInput` (0.5x–2.0x, step 0.1), a volume slider, and — when the `onOpenSettings` prop is provided — a settings cog as the last element.

The Player SHALL synchronize its state from `events.playbackStarted`, `playbackPaused`, `playbackStopped`, `playbackFinished` and `playbackPosition`. The position slider MUST apply optimistic updates while dragging (`draggingRef`) and seek only on `onChangeEnd`.

Hotkeys via `useHotkeys` SHALL be: `Space` (play/pause), `ArrowLeft` (seek −5 s), `ArrowRight` (seek +5 s). The mouse wheel over the speed input SHALL adjust speed by ±0.1 using a native non-passive wheel listener.

#### Scenario: Space toggles playback

- GIVEN an entry is loaded and paused
- WHEN the user presses Space
- THEN playback resumes, and pressing Space again pauses it

#### Scenario: Dragging the position slider does not fight position events

- GIVEN audio is playing
- WHEN the user drags the position slider thumb
- THEN the thumb follows the pointer without being reset by incoming `playback_position` events, and the seek is issued when the drag ends

#### Scenario: Speed wheel control

- GIVEN the pointer hovers the speed input
- WHEN the user scrolls the mouse wheel
- THEN the speed changes by 0.1 per wheel tick within [0.5, 2.0] and the page does not scroll

### Requirement: Text viewer rendering

The system SHALL display the selected entry's `original_text` read-only in `src/components/TextViewer.tsx` with three switchable modes via `SegmentedControl`: Plain, Markdown (default), HTML.

Markdown mode SHALL render via `markdown-it` with a custom rule wrapping inline tokens in `<span data-orig-start data-orig-end>` for word highlighting, and SHALL render mermaid code blocks with `mermaid.run()`; clicking a rendered mermaid diagram SHALL open it zoomed in a modal. HTML mode SHALL sanitize with `DOMPurify.sanitize()` and apply highlight.js code highlighting.

During playback the viewer SHALL highlight the currently spoken word: timestamps are fetched via `commands.getTimestamps`, cached in refs, and the active word is located by binary search (`findActiveTimestamp`) with `applyHighlight` mutating the DOM directly (no re-renders per position tick). Highlighting SHALL be cleared on stop/finish, preserved on pause, and disabled in HTML mode.

Editing the text of an existing entry in the UI MUST NOT be supported; corrections require deleting the entry and re-adding it (see the preview-dialog capability).

#### Scenario: Word follows narration

- GIVEN a ready entry is selected and playing in Markdown mode
- WHEN `playback_position` events arrive
- THEN the word span matching the current position is highlighted with `--ruvox-highlight-bg`, and the highlight moves forward without React re-renders

#### Scenario: Highlight survives pause

- GIVEN a word is highlighted during playback
- WHEN the user pauses
- THEN the highlight remains visible; it is cleared only on stop or finish

### Requirement: Settings dialog

The system SHALL provide a modal Settings dialog (`src/dialogs/Settings.tsx`) built on `@mantine/form::useForm`, loading `commands.getConfig()` and `commands.getAvailableEngines()` whenever it opens, and submitting a `UIConfigPatch` (only the form fields) via `commands.updateConfig`.

The form SHALL expose: TTS engine (Piper / Silero / «Silero (нативный)», with unavailable engines disabled and an alert when the saved engine was coerced to Piper), Piper voice or Silero speaker depending on the engine, sample rate, `notify_on_ready`, `notify_on_error`, `preview_dialog_enabled`, `max_cache_size_mb` (minimum 100 MB), theme (Светлая/Тёмная/Авто), and a language selector (Русский / English) bound to `UIConfig.language`. Applying a new theme SHALL push it into Mantine's color-scheme manager immediately. Choosing a language SHALL update the localization store immediately (the whole UI re-labels without reload) and include `language` in the submitted `UIConfigPatch`.

When «Silero (нативный)» is unavailable because the model bundle is not downloaded, the form SHALL show a download action next to the engine option; triggering it calls `commands.downloadSileroNativeBundle()` and displays live progress from the `bundle_download_*` events.

When the form's engine was coerced away from the saved engine because that engine is currently unavailable, and the user did not pick another engine, saving the dialog SHALL omit `engine` from the `UIConfigPatch` so the saved (or default) engine value is preserved instead of persisting the temporary fallback.

A "Очистить кэш…" button SHALL open a nested `CleanupCacheModal` with a target-MB input (disabled by "Очистить полностью"), a "Удалять тексты" checkbox, and a red warning when both full cleanup and text deletion are selected; confirmation calls `commands.clearCache({ mode, delete_texts })`.

#### Scenario: Save applies config and theme
- **GIVEN** the Settings dialog is open with theme changed to "Тёмная"
- **WHEN** the user clicks "Сохранить"
- **THEN** `commands.updateConfig` receives the patch, the UI switches to the dark scheme without reload, and a success notification is shown

#### Scenario: Engine availability gates selection
- **GIVEN** `getAvailableEngines` reports Silero as unavailable
- **WHEN** the Settings dialog opens
- **THEN** the Silero option is disabled with its reason shown, and a config saved with Silero is coerced to Piper with a yellow alert

#### Scenario: Save while coerced preserves the saved engine
- **GIVEN** the saved engine is unavailable and the form was coerced to the fallback engine
- **WHEN** the user saves the dialog without picking an engine
- **THEN** the `UIConfigPatch` sent to `commands.updateConfig` does not contain `engine`, so the previously saved (or default) engine value stays in effect

#### Scenario: Native engine offers bundle download
- **GIVEN** `getAvailableEngines` reports Silero Native as unavailable due to a missing bundle
- **WHEN** the Settings dialog opens
- **THEN** the «Silero (нативный)» option is disabled with its reason shown and a download button is offered; clicking it starts the download and shows progress until `bundle_download_finished`

#### Scenario: Full cache cleanup warns
- **GIVEN** the cleanup sub-modal is open
- **WHEN** the user checks both "Очистить полностью" and "Удалять тексты"
- **THEN** a red irreversibility warning appears and the confirm button turns red

#### Scenario: Language switch relabels immediately
- **GIVEN** the Settings dialog is open with language changed to "English"
- **WHEN** the user picks "English" (before or after saving)
- **THEN** all visible UI strings switch to the EN catalog without a reload,
  and saving persists `language: "en"` via `commands.updateConfig`

### Requirement: Design tokens

The system SHALL define design tokens in exactly two places: `src/theme.ts` (`createTheme` with `primaryColor: 'blue'`, otherwise Mantine defaults) for Mantine-scale values, and `:root` custom properties in `src/globals.css` prefixed `--ruvox-*` for what Mantine has no scale for.

The following `--ruvox-*` tokens SHALL exist:

| Token | Meaning |
|-------|---------|
| `--ruvox-brand` | Brand mark violet; must match `src-tauri/icons/source.svg` |
| `--ruvox-highlight-bg` | Word-highlight background during playback |
| `--ruvox-pre-bg` / `--ruvox-code-bg` | Code surfaces in rendered markdown/HTML |
| `--ruvox-table-border` / `--ruvox-table-header-bg` | Table chrome in rendered content |
| `--ruvox-reading-font-size` / `--ruvox-reading-line-height` | Reading text in TextViewer (17 px / 1.6) |
| `--ruvox-preview-z` / `--ruvox-preview-shadow` | Floating preview window chrome |

CSS Modules MUST NOT contain literal hex/px values where a `--mantine-*` or `--ruvox-*` token exists; one-off measurements tied to a layout quirk (e.g. the 74 px header, the 9 px play-button lift) are allowed inline only with a "why" comment. `html { font-size: 17px }` SHALL scale all rem-based Mantine tokens (HiDPI workaround on Wayland/KDE with WebKitGTK). Light/dark variants SHALL use the native CSS `light-dark()` function inside token definitions.

#### Scenario: Token-driven colors

- GIVEN a component needs the word-highlight background
- WHEN its CSS Module is written
- THEN it references `var(--ruvox-highlight-bg)` instead of a literal color, and the value flips with the color scheme via `light-dark()`

### Requirement: Styling approach

The system SHALL style components exclusively via CSS Modules (`*.module.css`) and the Mantine `classNames` prop. `sx`, `createStyles` and emotion MUST NOT be used. Global CSS SHALL be limited to `@mantine/core/styles.css` (plus notifications/highlight.js stylesheets) and `src/globals.css`, both imported in `src/main.tsx`.

#### Scenario: Component styles come from CSS Modules

- GIVEN a new component is added
- WHEN it needs custom styling
- THEN a sibling `*.module.css` file is created and passed via `className`/`classNames`, with no `sx` prop

### Requirement: State management

The system SHALL use Zustand for global client state — `src/stores/selectedEntry.ts` (`selectedId`, `selectedEntry`, `setSelectedId`, `setSelectedEntry`) and `src/stores/searchQuery.ts` (`query`, `setQuery`) — and `useState` for local component state. Redux and React Query MUST NOT be introduced; backend data flows through Tauri `invoke` + `useEffect`.

#### Scenario: Selection propagates across components

- GIVEN the user clicks a queue item
- WHEN `setSelectedEntry` stores the entry
- THEN `TextViewer` re-renders with that entry's text and `QueueList` marks the item as selected, without any prop drilling through `AppShell`

### Requirement: Typed IPC layer

The system SHALL route all backend communication through typed wrappers in `src/lib/tauri.ts` (`commands` for `invoke`, `events` for `listen`). Invoke parameter keys MUST be camelCase on the JS side even when the Rust handler declares snake_case parameters (Tauri 2 convention).

#### Scenario: Snake-case function, camel-case invoke key

- GIVEN a wrapper such as `seekTo(position_sec)`
- WHEN it calls the backend
- THEN it invokes `tauriInvoke('seek_to', { positionSec: position_sec })` so Tauri maps the argument correctly

### Requirement: First-run Silero Native bundle prompt

On startup, when the persisted `UIConfig.engine` is `"silero_native"` and
the availability probe reports the Silero Native bundle as unavailable, the
app SHALL show a modal prompt offering a one-time download of the model
bundle (~230 MB) — the default engine cannot run without it, and the session
is otherwise silently served by the Piper fallback.

The prompt SHALL offer two actions:

- **Download** — starts `download_silero_native_bundle` and shows inline
  progress driven by the `bundle_download_*` events. On success the app
  SHALL activate the native engine for the running session (via
  `update_config` with `engine: "silero_native"`) and confirm with a green
  notification. On failure the prompt SHALL show the error and allow a
  retry.
- **Stay on Piper** — closes the prompt for the current run without
  persisting anything.

The prompt SHALL NOT appear when the user has explicitly chosen another
engine, nor once the bundle is on disk. It reappears on the next launch
while the condition holds (no "don't ask again" persistence in this
revision).

#### Scenario: fresh install is offered the bundle

- GIVEN a fresh install (default config `engine = "silero_native"`) and no
  downloaded bundle
- WHEN the app starts and the config loads
- THEN the bundle prompt opens, naming the engine and the download size

#### Scenario: download activates the native engine

- GIVEN the bundle prompt is open
- WHEN the user accepts and the download finishes successfully
- THEN the active engine switches to Silero Native without a restart, and a
  green confirmation is shown

#### Scenario: explicit Piper choice never prompts

- GIVEN the persisted config has `engine = "piper"`
- WHEN the app starts
- THEN the bundle prompt does not open, regardless of bundle availability

#### Scenario: downloaded bundle never prompts

- GIVEN the persisted config has `engine = "silero_native"` and the bundle
  probe reports available
- WHEN the app starts
- THEN the bundle prompt does not open

#### Scenario: decline is per-run

- GIVEN the bundle prompt is open
- WHEN the user chooses to stay on Piper
- THEN the prompt closes, the session continues on the Piper fallback, the
  persisted config is unchanged, and the prompt appears again on the next
  launch

### Requirement: Settings shows the app version

The Settings dialog SHALL display the application version in the bottom-right of
the dialog (the footer, right-aligned) so it is always visible (not gated behind
the Windows-only updater). The version SHALL be read from the app manifest via
Tauri's `getVersion()` (which resolves the `version` field of `tauri.conf.json`
at build time) and shown as `Версия: <version>`.

The version SHALL be fetched whenever the dialog opens (alongside `getConfig` and
`getCacheDir`) and SHALL fall back to `—` if the IPC call fails, so a backend or
version read error never blocks the dialog.

#### Scenario: Version is shown when Settings opens

- GIVEN the application has started
- WHEN the user opens Settings
- THEN the bottom-right footer shows `Версия: <version>` matching the built
  `tauri.conf.json` version (e.g. `0.3.1`)

#### Scenario: Version read failure degrades gracefully

- GIVEN `getVersion()` fails (e.g. IPC unavailable)
- WHEN the user opens Settings
- THEN the version line shows `—` and the rest of the dialog still works

### Requirement: UI localization layer

The frontend SHALL localize user-visible strings through a hand-rolled
localization layer (`src/i18n/`): string catalogs for `ru` (default) and
`en`, keyed by stable message ids; a translation helper `t(key, params?)`;
and a locale store (`src/stores/locale.ts`) seeded from
`commands.getConfig()` at startup and updated by the Settings selector.
Interpolation SHALL use positional `{0}`-style params. A missing catalog key
SHALL fall back to the RU entry, then to the key itself. Non-React modules
(notification bridge, updater, copy helpers) SHALL read the active locale at
call time.

All user-visible strings of the shell, queue, player, viewer, dialogs, and
notification helpers SHALL come from the catalogs; no hardcoded UI prose in
components. Backend error codes (`CommandError.code`) SHALL have catalog
entries under an `errors.` prefix; unknown codes fall back to `message`, then
to a generic per-`type` string (see the ipc-commands spec).

#### Scenario: RU is the default on first run
- **GIVEN** a fresh config without a saved language
- **WHEN** the app starts
- **THEN** every visible string comes from the RU catalog

#### Scenario: Switching to EN relabels everything
- **GIVEN** the app running in Russian
- **WHEN** the user switches the language to English in Settings
- **THEN** component labels, dialog texts, status badges, toasts triggered
  afterwards, and error messages rendered from backend codes are all in
  English

### Requirement: Code-highlight theme follows color scheme

The code-highlight stylesheet (highlight.js) SHALL follow Mantine's active
color scheme — light theme CSS when the scheme resolves to light, dark theme
CSS when it resolves to dark — instead of being statically imported once.
Switching the scheme (including via the auto scheme following the OS) SHALL
swap the highlight styles without a reload.

#### Scenario: Dark scheme swaps highlight styles
- **GIVEN** the app rendering a Markdown entry with highlighted code
- **WHEN** the user switches the theme from Светлая to Тёмная
- **THEN** the `.hljs` colors switch to the dark palette without a reload
