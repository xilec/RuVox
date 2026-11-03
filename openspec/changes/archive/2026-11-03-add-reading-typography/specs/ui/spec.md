## ADDED Requirements

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

## MODIFIED Requirements

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
