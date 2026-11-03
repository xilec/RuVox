# UI Specification

## Purpose

Covers the structure and behavior of the RuVox frontend (`src/`): the application shell layout (header with the player, resizable navbar with the queue, main text viewer), the key components (`AppShell`, `QueueList`, `Player`, `TextViewer`, `Settings`), global state management, and the design-token system (`src/theme.ts` + `--ruvox-*` custom properties in `src/globals.css`). The frontend is React 18 + TypeScript (strict) + Mantine 8, bundled with Vite inside a Tauri 2 webview.
## Requirements
### Requirement: Application shell layout

The system SHALL render the root layout via `MantineAppShell` in `src/components/AppShell.tsx` with a fixed 74 px header containing only the `Player` component, a navbar containing the queue, and a main area rendering `TextViewer` for the selected entry.

The header height of 74 px with asymmetric padding (top 18 / bottom 8 in `Player.module.css`) MUST be preserved: the floating slider labels (position and volume) must not be clipped by the native window title bar, and the 48 px brand logo must fit next to the Play button.

The navbar SHALL be drag-resizable via an invisible 6 px strip on its right border with `cursor: col-resize`; the width MUST be clamped to a minimum of 180 px and a maximum of 70% of the window width.

The navbar SHALL contain a "Очередь" title with an Add split-button (size `xs`) to its right — the primary part keeps the clipboard Add behavior and the dropdown menu offers the import actions «Файл…», «Файл с кодировкой…», and «По ссылке…» (see the text-import capability) — plus a search input filtering the queue, and the `QueueList` component. The `PreviewDialog` and `SettingsModal` SHALL be mounted from `AppShell`.

While content is dragged over the window, AppShell SHALL render a full-window drop overlay («Отпустите, чтобы добавить») above the layout; the overlay SHALL not intercept clicks when no drag is active.

#### Scenario: Window renders the three regions

- GIVEN the application has started
- WHEN the main window is displayed
- THEN the header shows only the player controls, the navbar shows "Очередь" with an Add split-button, a search field and the entry list, and the main area shows the selected entry's text (or a "Нет выбранной записи" placeholder)

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

Each queue item SHALL show a 60-character preview of `original_text`, a status badge (Ожидание/Обработка/Готово/Играет/Ошибка), the duration when available, and a Play action enabled only for `ready`/`playing` entries. Clicking an item SHALL store it as the selected entry in the Zustand `selectedEntry` store. Double-clicking an item SHALL open the recording parameters dialog under the same gate as the "Параметры записи…" menu item: only when the entry has a generation snapshot or a generation timestamp; otherwise the double-click SHALL be a no-op. Single-click selection, the context menu, and keyboard activation keep their existing behavior.

Right-clicking an item SHALL open a context menu with "Воспроизвести", "Сохранить аудио как…", "Перегенерировать аудио" and "Удалить". The "Сохранить аудио как…" item SHALL be enabled under the same gate as Play (`ready`/`playing` entries). Activating it SHALL call `commands.pickExportAudioPath(entryId)`; a `null` result (cancelled dialog) SHALL be a silent no-op; a chosen path SHALL be handed to `commands.exportAudio(entryId, path)`, and on success a confirmation notification SHALL show the target path. Failures SHALL surface the localized red error notification. Deletion MUST be confirmed via `modals.openConfirmModal` before calling `commands.deleteEntry`.

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

#### Scenario: Export a ready entry's audio

- GIVEN an entry with status `ready` and a stored audio file
- WHEN the user activates "Сохранить аудио как…" and chooses a target path
- THEN `commands.exportAudio` is called with the entry id and the chosen
  path, and a confirmation notification with the path is shown

#### Scenario: Cancelled save dialog does nothing

- GIVEN the save dialog opened from "Сохранить аудио как…"
- WHEN the user cancels it
- THEN no export command is invoked and no notification is shown

#### Scenario: Export failure surfaces an error

- GIVEN an entry whose cached audio file has been evicted
- WHEN the user activates "Сохранить аудио как…" and chooses a target path
- THEN a red error notification is shown and no success notification appears

#### Scenario: Export is disabled for pending entries

- GIVEN an entry with status `pending` or `processing` or `error`
- WHEN the item's context menu is open
- THEN "Сохранить аудио как…" is disabled

#### Scenario: Double-click opens the parameters dialog

- GIVEN a ready entry with a generation snapshot
- WHEN the user double-clicks the queue item
- THEN the recording parameters dialog opens for that entry and the entry is selected

#### Scenario: Double-click is a no-op for never-synthesized entries

- GIVEN a pending entry with no generation snapshot and no generation timestamp
- WHEN the user double-clicks the queue item
- THEN the recording parameters dialog does not open

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

The system SHALL provide a modal Settings dialog (`src/dialogs/Settings.tsx`) built on `@mantine/form::useForm`, loading `commands.getConfig()`, `commands.getAvailableEngines()` and `commands.getTerattsVoices()` whenever it opens, and submitting a `UIConfigPatch` (only the form fields) via `commands.updateConfig`.

The form SHALL expose: TTS engine (Piper / Silero / «Silero (нативный)» / «TeraTTSv2», with unavailable engines disabled and an alert when the saved engine was coerced to Piper), Piper voice or Silero speaker depending on the engine, the TeraTTS voice when that engine is selected (every id returned by `commands.getTerattsVoices()`, with `ru_m5` as the default), sample rate, `notify_on_ready`, `notify_on_error`, `preview_dialog_enabled`, `hide_to_tray_on_close` (the «Сворачивать в трей при закрытии окна» switch, off by default), `max_cache_size_mb` (minimum 100 MB), theme (Светлая/Тёмная/Авто), a language selector (Русский / English) bound to `UIConfig.language`, a code block narration selector («Кратко» / «Читать полностью») bound to `UIConfig.code_block_mode` (`"brief"` / `"read"`), and an English text selector («Английский текст»: «Авто» / «Нативно» / «По-русски») bound to `UIConfig.english_reading` (`"auto"` / `"native"` / `"narrated"`), initialized from the loaded config and included in the submitted `UIConfigPatch` when changed. Applying a new theme SHALL push it into Mantine's color-scheme manager immediately. Choosing a language SHALL update the localization store immediately (the whole UI re-labels without reload) and include `language` in the submitted `UIConfigPatch`.

The form SHALL also expose a «Чтение» section: a `Узко | Широко` `SegmentedControl` bound to `UIConfig.reading_width` (`"narrow"` / `"wide"`) and a `[ − ] N% [ + ]` stepper bound to `UIConfig.reading_scale` with the same 80/100/125/150/175/200 ladder and reset action as the viewer toolbar, initialized from the loaded config and included in the submitted `UIConfigPatch`.

The English text selector SHALL read the active engine's capability from `commands.getAvailableEngines()` (`reads_latin_natively`), never from a hardcoded engine name. All three choices SHALL stay selectable on every engine — the preference is engine-independent — and when the selected engine cannot read Latin natively while the choice is «Нативно», the dialog SHALL show a hint stating that the engine will read English in Russian while the stored choice stays «Нативно».

When «Silero (нативный)» is unavailable because the model bundle is not downloaded, the form SHALL show a download action next to the engine option; triggering it calls `commands.downloadSileroNativeBundle()` and displays live progress from the `bundle_download_*` events. When «TeraTTSv2» is unavailable because its bundle is not downloaded, the form SHALL offer the analogous action through `commands.downloadTerattsBundle()`, naming the bundle size, with progress from the same event family and `engine: "teratts"`.

When the form's engine was coerced away from the saved engine because that engine is currently unavailable, and the user did not pick another engine, saving the dialog SHALL omit `engine` from the `UIConfigPatch` so the saved (or default) engine value is preserved instead of persisting the temporary fallback.

A "Очистить кэш…" button SHALL open a nested `CleanupCacheModal` with a target-MB input (disabled by "Очистить полностью"), a "Удалять тексты" checkbox, and a red warning when both full cleanup and text deletion are selected; confirmation calls `commands.clearCache({ mode, delete_texts })`.

#### Scenario: Save applies config and theme
- **GIVEN** the Settings dialog is open with theme changed to "Тёмная"
- **WHEN** the user clicks "Сохранить"
- **THEN** `commands.updateConfig` receives the patch, the UI switches to the dark scheme without reload, and a success notification is shown

#### Scenario: Save persists the code block narration mode
- **GIVEN** the Settings dialog is open with the saved config `code_block_mode: "read"` and the user picks «Кратко»
- **WHEN** the user clicks "Сохранить"
- **THEN** the submitted `UIConfigPatch` contains `code_block_mode: "brief"`

#### Scenario: Save persists the reading typography
- **GIVEN** the Settings dialog is open with the saved config `reading_width: "narrow"` and `reading_scale: 100`, and the user picks «Широко» and presses `+` once
- **WHEN** the user clicks "Сохранить"
- **THEN** the submitted `UIConfigPatch` contains `reading_width: "wide"` and `reading_scale: 125`

#### Scenario: English text selector reflects the saved choice
- **GIVEN** the Settings dialog is open with the saved config `english_reading: "narrated"`
- **WHEN** the dialog finishes loading
- **THEN** the «Английский текст» selector shows «По-русски», and saving without touching it submits `english_reading: "narrated"`

#### Scenario: Native choice on an incapable engine shows the fallback hint
- **GIVEN** `getAvailableEngines` reports `reads_latin_natively: false` for the selected engine
- **WHEN** the user picks «Нативно» in the «Английский текст» selector
- **THEN** a hint appears stating that this engine will read English in Russian, and saving still submits `english_reading: "native"`

#### Scenario: Capable engine shows no fallback hint
- **GIVEN** `getAvailableEngines` reports `reads_latin_natively: true` for the selected engine
- **WHEN** the user picks «Нативно»
- **THEN** no fallback hint is shown

#### Scenario: Save persists the tray-on-close setting
- **GIVEN** the Settings dialog is open with the saved config `hide_to_tray_on_close: false`
- **WHEN** the user enables «Сворачивать в трей при закрытии окна» and clicks "Сохранить"
- **THEN** the submitted `UIConfigPatch` contains `hide_to_tray_on_close: true`

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

#### Scenario: TeraTTS option lists its voices
- **GIVEN** `getAvailableEngines` reports TeraTTS as available, `getTerattsVoices` reports `ru_m5`, `ru_f1`, `ru_f2`, `ru_m1`, `eng_f3`, `eng_f4_whisper`, `eng_f5`, `eng_m2_whisper`, `eng_m3` and `eng_m4`, and the saved config has `teratts_voice: "eng_f5"`
- **WHEN** the user selects «TeraTTSv2»
- **THEN** the voice selector offers all ten ids with `eng_f5` preselected, and saving persists the chosen value

#### Scenario: Saved voice missing from the catalogue stays selectable
- **GIVEN** the saved config has `teratts_voice: "eng_f5"` and `getTerattsVoices` reports only `ru_m5` and `ru_f1`
- **WHEN** the user selects «TeraTTSv2» and saves without touching the selector
- **THEN** the selector still offers and preselects `eng_f5`, and the submitted `UIConfigPatch` keeps `teratts_voice: "eng_f5"`

#### Scenario: TeraTTS offers bundle download
- **GIVEN** `getAvailableEngines` reports TeraTTS as unavailable due to a missing bundle
- **WHEN** the Settings dialog opens
- **THEN** the «TeraTTSv2» option is disabled with its reason shown and a download action names the bundle size; clicking it starts `downloadTerattsBundle` and shows progress until `bundle_download_finished`

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
| `--ruvox-reading-measure` | Maximum line measure of the narrow reading column (70ch) |
| `--ruvox-reading-scale` | Reading zoom multiplier applied to the viewer content (1 = 100 %) |
| `--ruvox-preview-z` / `--ruvox-preview-shadow` | Floating preview window chrome |

CSS Modules MUST NOT contain literal hex/px values where a `--mantine-*` or `--ruvox-*` token exists; one-off measurements tied to a layout quirk (e.g. the 74 px header, the 9 px play-button lift) are allowed inline only with a "why" comment. `html { font-size: 17px }` SHALL scale all rem-based Mantine tokens (HiDPI workaround on Wayland/KDE with WebKitGTK). Light/dark variants SHALL use the native CSS `light-dark()` function inside token definitions.

#### Scenario: Token-driven colors

- GIVEN a component needs the word-highlight background
- WHEN its CSS Module is written
- THEN it references `var(--ruvox-highlight-bg)` instead of a literal color, and the value flips with the color scheme via `light-dark()`

#### Scenario: Reading width and scale come from tokens

- GIVEN the viewer renders the narrow reading column at 150 %
- WHEN its CSS Module is written
- THEN it uses `var(--ruvox-reading-measure)` and `var(--ruvox-reading-scale)` instead of a literal `70ch` or `1.5`, and the scale override is set as an inline custom property on the content container

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

### Requirement: Recording parameters dialog

The queue context menu item "Параметры записи…" — and a double-click on the
queue item, gated the same way — SHALL open a read-only dialog showing the
entry's ingestion source ("Источник": буфер обмена / файл / ссылка) as the
first row, followed by the generation snapshot: engine, voice, sample rate,
model identity (name and checksum when available), app version, the code-block
narration mode used, normalized-text checksum, audio codec and size, duration,
generation timestamp, and the generation number (`generation_count`). The item
SHALL be disabled for entries that have neither a snapshot nor a generation
timestamp (never synthesized).

Values absent from the snapshot SHALL render as a placeholder dash rather than guessed values. For entries with audio but no snapshot (synthesized by older builds), the dialog SHALL show an explanatory line that parameters were not recorded. Engine and Piper voice names SHALL be shown with their localized display names; the dialog and menu item SHALL be localized in Russian and English.

#### Scenario: Dialog shows the snapshot

- GIVEN a ready entry whose snapshot records engine `silero_native`, speaker `xenia`, and an Ogg Opus file
- WHEN the user opens "Параметры записи…" for the entry
- THEN the dialog shows the ingestion source first, then the localized engine name, the voice, the sample rate, the audio codec and size, and the generation number

#### Scenario: Absent values render as a dash

- GIVEN a ready entry whose snapshot has no model identity
- WHEN the parameters dialog is open
- THEN the model row renders a placeholder dash, not an invented value

#### Scenario: Legacy entry without a snapshot

- GIVEN a ready entry synthesized by an older build (`generation` is null, `audio_generated_at` is set)
- WHEN the user opens "Параметры записи…" for the entry
- THEN the dialog opens, shows the generation timestamp and duration where known, an explanatory line that parameters were not recorded, and dashes for unknown values

#### Scenario: Item disabled for never-synthesized entries

- GIVEN a pending entry with no audio
- WHEN the user opens the context menu for the entry
- THEN "Параметры записи…" is disabled

### Requirement: Dictionary editor

The Settings dialog SHALL contain a "Словарь" section showing the entry
count and a button that opens a dedicated dictionary editor modal (the
nested-modal pattern of the cache cleanup dialog). The editor SHALL provide:
an alphabetical, case-insensitive-substring searchable table of entries
(`from` as typed, `to`, a "переопределяет встроенное" badge on entries
flagged by the backend); add / edit / delete with delete confirmed via the
standard confirm modal; form validation per the user-dictionary spec, where
submitting a `from` that already exists updates that entry's spoken form
with the typed value instead of creating a duplicate. Every completed action saves immediately (no save
button); the modal footer SHALL carry a quiet status line — "Все изменения
сохранены" / "Сохранение…" / "Не сохранено — повторить" (with a retry
action) — and CRUD success SHALL NOT raise toast notifications. Import and
export buttons SHALL use the standard file dialogs; import additionally
accepts a dropped file and, before applying, asks the user to choose
"Объединить" (merge) or "Заменить список" (replace); the import result is
reported as a notification with added / updated / skipped counts. All UI
strings are Russian.

#### Scenario: Open the editor from Settings

- GIVEN the Settings dialog is open
- WHEN the user clicks the dictionary section button
- THEN the dictionary editor modal opens above Settings

#### Scenario: Search filters the list

- GIVEN entries "docker", "kubectl", "nginx"
- WHEN the user types "ku" in the search field
- THEN only the "kubectl" row remains

#### Scenario: Duplicate from updates the existing entry

- GIVEN the entry "docker → докер" exists
- WHEN the user submits a new entry with from "Docker" and spoken form
  "докка"
- THEN the existing entry becomes "docker → докка" and no second entry is
  created

#### Scenario: Delete requires confirmation

- GIVEN the entry list is shown
- WHEN the user invokes delete on a row
- THEN a confirmation modal appears and the entry is removed only after
  confirmation

#### Scenario: Status line reflects saving

- GIVEN the user adds a valid entry
- WHEN the save command is in flight
- THEN the footer reads "Сохранение…" and returns to "Все изменения
  сохранены" on completion — with no toast

#### Scenario: Import asks for the mode

- GIVEN the user drops a dictionary TOML file onto the editor
- WHEN the drop is accepted
- THEN a choice between "Объединить" and "Заменить список" appears, and the
  applied result is reported with added / updated / skipped counts

### Requirement: Synthesis failure on the queue card

A queue card SHALL render the entry's persisted `error_message`; it MUST NOT
depend on the transient `tts_error` toast. When the entry status is `error`,
the card SHALL show a red-tinted row with a 3 px left border and, below the
title, a localized prefix («Ошибка синтеза» in RU) followed by the stored
engine message verbatim — the message MUST NOT be parsed, classified or
translated. The message SHALL be clamped to two lines, SHALL expose the full
text through the `title` attribute, and SHALL be selectable and copyable. When
the clamped text overflows, a localized expand control («Показать полностью»)
SHALL appear and toggle the message between the clamped and the full state
(«Свернуть» when expanded).

Both failure variants SHALL offer a «Повторить» action that invokes
`commands.regenerateEntry(id, false)` directly — no regeneration preview
dialog, because the text has not changed and this is failure recovery. The
action SHALL be disabled while its command is pending and SHALL stay busy
until an `entry_updated` event for that entry reports that the retryable error
state has cleared; a rejected command SHALL clear the busy state and surface
the localized red error notification.

When the entry is playable (`ready`) and `error_message` is set — a failed
regeneration that kept the previous recording — the card SHALL show an amber
inline note «Последняя перегенерация не удалась: <message>» with the hint that
the previous recording still plays and the same «Повторить» action. The audio
SHALL stay playable, the red error treatment MUST NOT be applied, and the note
SHALL disappear when the next regeneration starts (its `entry_updated` clears
`error_message`).

The `tts_error` toast SHALL keep its existing behavior. Cards whose
`error_message` is `null` SHALL render exactly as before.

#### Scenario: failed entry shows its stored reason

- GIVEN an entry with status `error` and `error_message`
  "модель не загружена (ttsd)"
- WHEN the queue renders
- THEN the card shows "Ошибка синтеза: модель не загружена (ttsd)" with the red
  error treatment, without waiting for a toast

#### Scenario: long failure message expands and collapses

- GIVEN an error card whose stored message overflows the two-line clamp
- WHEN the user activates «Показать полностью»
- THEN the message is shown in full and the control reads «Свернуть»;
  activating it clamps the message again

#### Scenario: retry starts a regeneration without a preview

- GIVEN an error card
- WHEN the user activates «Повторить»
- THEN `commands.regenerateEntry(entryId, false)` is called once, no preview
  dialog opens, and the action is disabled until the entry's `entry_updated`
  clears the error state

#### Scenario: failed regeneration shows the amber note

- GIVEN a `ready` entry with stored audio and `error_message` "таймаут ttsd"
- WHEN the queue renders
- THEN the card shows the amber «Последняя перегенерация не удалась: таймаут
  ttsd» note with «Повторить», the Play action stays enabled, and the red
  error treatment is absent

#### Scenario: a new regeneration hides the note

- GIVEN a `ready` entry showing the amber failed-regeneration note
- WHEN the retry starts and `entry_updated` arrives with `error_message: null`
- THEN the note disappears and the card renders like a healthy entry

#### Scenario: healthy entries are unchanged

- GIVEN entries with no `error_message`
- WHEN the queue renders
- THEN no error block, retry action or amber note appears, and the card layout
  matches the pre-change layout

### Requirement: Synthesis progress on the queue card

A queue card whose status is `processing` SHALL render, below the title, an
indeterminate progress line without a percentage, ETA or chunk stage, the
stage text «Синтез… M:SS» carrying the elapsed time, and an «Отменить»
button on the same row. A card whose status is `pending` SHALL render «В
очереди» and the same button without a timer. The button MUST be visible
without hovering and MUST NOT live inside the hover-only action group; it
SHALL be disabled while its `cancel_synthesis` call is in flight, and its
accessible label SHALL name the entry (for example «Отменить синтез:
<preview>»). A rejected command SHALL surface the existing localized red
notification, and the RU and EN catalogs SHALL carry the new strings.

A small per-row component SHALL own the local start timestamp and the
one-second tick, so that a tick re-renders only that line and neither the
queue item nor the queue list. The interval SHALL be cleared when the entry
reaches any terminal status (`ready`, `error`, `cancelled`) and when the
component unmounts. Rows in a terminal status SHALL render exactly as before,
including the persisted failure note and retry action from #351.

#### Scenario: Processing row presentation

- GIVEN an entry with status `processing`
- WHEN the queue card is rendered
- THEN it shows an indeterminate progress line, «Синтез… 0:00» and an
  «Отменить» button visible without hover
- AND it shows no percentage, ETA or chunk stage

#### Scenario: Pending row presentation

- GIVEN an entry with status `pending`
- WHEN the queue card is rendered
- THEN it shows «В очереди» and the «Отменить» button and no timer

#### Scenario: Timer stops on a terminal status

- GIVEN a processing card whose timer has advanced to «Синтез… 0:05»
- WHEN the entry becomes `ready`, `error` or `cancelled`
- THEN the progress line disappears and no further timer tick renders

#### Scenario: A tick re-renders only the progress line

- GIVEN a queue with several cards, one of them `processing`
- WHEN one second passes and the elapsed text advances
- THEN only the ticking progress line re-renders; the queue items and the
  list do not

#### Scenario: Cancel is invoked once and disables the button

- GIVEN a processing card with the inline «Отменить» action
- WHEN the user clicks it while the command is in flight
- THEN `cancel_synthesis` is called once and the button is disabled until
  the command settles

#### Scenario: Cancel failure shows the existing toast

- GIVEN a processing card with the inline «Отменить» action
- WHEN `cancel_synthesis` rejects
- THEN the existing localized red notification is shown and the button is
  enabled again

#### Scenario: Accessible label names the entry

- GIVEN a processing card whose text starts with «Тёмная сторона инлайнинга»
- WHEN the inline cancel button is inspected
- THEN its accessible label is «Отменить синтез: Тёмная сторона инлайнинга…»

#### Scenario: English labels after a locale switch

- GIVEN the queue renders a `pending` and a `processing` card in RU
- WHEN the UI language switches to EN
- THEN the cards read "Queued" and "Synthesizing… 0:05" and the button reads
  "Cancel"

### Requirement: Text viewer reading controls

The text viewer SHALL show reading-typography controls in a toolbar above the
content, to the right of the display-mode switch, in every display mode —
including a plain source, which has no display-mode switch:

- a `Узко | Широко` `SegmentedControl` bound to `UIConfig.reading_width`
  (`"narrow"` | `"wide"`);
- a `[ − ] N% [ + ]` stepper bound to `UIConfig.reading_scale` with the ladder
  80/100/125/150/175/200 percent: `−` moves to the nearest lower step, `+` to
  the nearest higher step, clicking the percentage resets it to 100, and the
  bound buttons are disabled at 80 % and 200 %.

Each control SHALL persist its change immediately through
`commands.updateConfig` (`{ reading_width }` or `{ reading_scale }`) with
optimistic UI: the control shows the new value before the request resolves, and
on failure it reverts to the previous value and a localized error toast is
shown. Both controls SHALL be keyboard reachable with localized accessible
names from the RU/EN catalogs.

The `reading_scale` SHALL be applied as CSS `zoom` on the viewer content
container in all display modes, so text, headings, code, tables, borders and
images scale by the same factor while the pane keeps its width and the content
re-wraps inside it. `transform: scale()` MUST NOT be used. `pre` SHALL keep
`overflow-x: auto`, and every rendered table SHALL sit in a horizontal scroll
wrapper (`data-scroll-x`), so wide code and tables scroll inside the reading
column instead of being clipped.

`reading_width: "narrow"` SHALL constrain the content to
`max-width: min(var(--ruvox-reading-measure), 100%)` with `margin-inline: auto`;
`"wide"` SHALL keep the current full-pane width. Width and scale SHALL be
independent: the narrow column keeps its character measure as the scale
changes, and the wide layout still fills the pane.

#### Scenario: Controls are present in every display mode

- **GIVEN** an entry is selected, with any source format and display mode
- **WHEN** the viewer renders
- **THEN** the reading-width segments and the reading-scale stepper are in the
  toolbar, and either control changes the content immediately

#### Scenario: Scale ladder and bounds

- **GIVEN** the saved `reading_scale` is 100
- **WHEN** the user presses `+` twice
- **THEN** the displayed scale is 150 %, `+` stays enabled, and at 200 % `+` is
  disabled while at 80 % `−` is disabled

#### Scenario: Off-ladder persisted scale

- **GIVEN** the persisted `reading_scale: 93`
- **WHEN** the viewer renders
- **THEN** the stepper shows `93 %`, `−` moves it to 80 %, and `+` moves it to 100 %

#### Scenario: Clicking the percentage resets the scale

- **GIVEN** the saved `reading_scale` is 175
- **WHEN** the user clicks the percentage value
- **THEN** the scale becomes 100 % and `update_config` receives
  `reading_scale: 100`

#### Scenario: Failed persistence reverts the control

- **GIVEN** the saved `reading_scale` is 100
- **WHEN** the user presses `+` and `update_config` rejects
- **THEN** the control returns to 100 % and a localized error toast is shown

#### Scenario: Zoom scales the whole document

- **GIVEN** an HTML entry with headings, a table, code and an image
- **WHEN** the scale is 150 %
- **THEN** the headings, table, code and image all render 1.5× larger, the pane
  width is unchanged, and tables and code scroll inside the column instead of
  being clipped

#### Scenario: Narrow column measures about 70 characters

- **GIVEN** `reading_width: "narrow"` in a 1440 px window
- **WHEN** the viewer renders
- **THEN** the content is centered with a maximum measure near 70 characters,
  and choosing `"wide"` restores the full-pane layout

