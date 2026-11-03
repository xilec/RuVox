# Delta: ui

## MODIFIED Requirements

### Requirement: Settings dialog

The system SHALL provide a modal Settings dialog (`src/dialogs/Settings.tsx`) built on `@mantine/form::useForm`, loading `commands.getConfig()`, `commands.getAvailableEngines()` and `commands.getTerattsVoices()` whenever it opens, and submitting a `UIConfigPatch` (only the form fields) via `commands.updateConfig`. The dialog SHALL be about 900 px wide (capped by the viewport) and use a two-pane layout: a section nav with the search field on the left, an independently scrolling content pane on the right, and a footer pinned below both panes so «Сброс» and «Сохранить» stay visible without scrolling.

Settings SHALL be grouped into seven sections in this order: «Движок и голос» (engine, the engine-specific voice, sample rate, the models dialog entry and the coerced-engine alert), «Чтение текста» (code block narration, English text with its fallback hint, the preview-dialog toggle and the reading width/scale controls), «Уведомления» (`notify_on_ready`, `notify_on_error`), «Хранилище» (`max_cache_size_mb`, the data folder with its reveal action and «Очистить кэш…»), «Интерфейс» (`hide_to_tray_on_close`, theme, language), «Словарь» (the entry count and the button opening the nested dictionary editor) and «О программе» (version, the update check when the updater is supported, and the logs folder with its reveal action). Every control SHALL live in exactly one section, and switching sections SHALL not change any form value.

Above the section nav the dialog SHALL show a search field filtering the settings rows by their labels. The index SHALL cover both the RU and EN catalogs — a Russian query finds a setting while the UI runs in English and vice versa — plus a small synonym map, case- and ё-insensitively, and SHALL contain only the rows the dialog actually renders in the current environment (for example, the update check when the updater is supported, the data and logs folder rows when their paths are available), so Enter always lands on a visible row. Sections without a matching row SHALL be hidden from the nav, and a query with no matches SHALL show a localized no-results state instead of rows.

A matching row SHALL always be filtered, but SHALL be highlighted only when the query is a literal substring of its visible label; a cross-catalog or synonym match filters the row without a highlight.

Enter SHALL jump to the first matching row in nav order — switching to its section and scrolling it into view — without submitting the form. Esc with a non-empty query SHALL clear the query and keep the dialog open; with an empty query it closes the dialog.

Unsaved changes SHALL be tracked against the loaded config, not against the factory defaults: every section owning a field whose value differs from the loaded config SHALL show a dot in the nav, and the footer SHALL show «Изменено: N» with N being the number of changed fields. The coerced engine SHALL NOT count as a change and SHALL NOT mark «Движок и голос» dirty; choosing a new value and restoring the saved one SHALL clear the marker.

«Сброс» SHALL restore the factory defaults (the form values and the localization store; field errors are cleared), and «Сохранить» SHALL submit the same `UIConfigPatch` as before — every form field, with `engine` omitted only in the coerced case. Closing the dialog with X SHALL discard the edits.

Below about 700 px the section nav SHALL collapse into a `Select` above the content, with the search field staying above it; the select SHALL list the visible sections, mark the ones with unsaved changes like the nav does, and switching there SHALL show that section's rows.

The nested models dialog, the cleanup dialog and the dictionary editor SHALL keep working; closing a nested dialog SHALL leave the dialog on the section it was opened from, and the cleanup dialog's `closeOnEscape` gating SHALL stay unchanged.

The form SHALL expose: TTS engine (Piper / Silero / «Silero (нативный)» / «TeraTTSv2», with unavailable engines disabled and an alert when the saved engine was coerced to Piper), Piper voice or Silero speaker depending on the engine, the TeraTTS voice when that engine is selected (every id returned by `commands.getTerattsVoices()`, with `ru_m5` as the default), sample rate, `notify_on_ready`, `notify_on_error`, `preview_dialog_enabled`, `hide_to_tray_on_close` (the «Сворачивать в трей при закрытии окна» switch, off by default), `max_cache_size_mb` (minimum 100 MB), theme (Светлая/Тёмная/Авто), a language selector (Русский / English) bound to `UIConfig.language`, a code block narration selector («Кратко» / «Читать полностью») bound to `UIConfig.code_block_mode` (`"brief"` / `"read"`), and an English text selector («Английский текст»: «Авто» / «Нативно» / «По-русски») bound to `UIConfig.english_reading` (`"auto"` / `"native"` / `"narrated"`), initialized from the loaded config and included in the submitted `UIConfigPatch` when changed. Applying a new theme SHALL push it into Mantine's color-scheme manager immediately. Choosing a language SHALL update the localization store immediately (the whole UI re-labels without reload) and include `language` in the submitted `UIConfigPatch`.

The «Чтение текста» section SHALL expose a `Узко | Широко` `SegmentedControl` bound to `UIConfig.reading_width` (`"narrow"` / `"wide"`) and a `[ − ] N% [ + ]` stepper bound to `UIConfig.reading_scale` with the same ladder and reset action as the viewer header's reading controls, initialized from the loaded config and included in the submitted `UIConfigPatch`.

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

#### Scenario: Sections group the controls
- **GIVEN** the Settings dialog is open
- **WHEN** the user switches through the section nav
- **THEN** every setting control is rendered in exactly one section, and the form values stay unchanged while switching

#### Scenario: Search matches a label in the active catalog
- **GIVEN** the dialog runs with the RU catalog
- **WHEN** the user types «тема» in the search field
- **THEN** only «Интерфейс» stays in the nav, only the «Тема оформления» row is shown, and the matched «Тема» substring is highlighted

#### Scenario: Cross-catalog match filters without a highlight
- **GIVEN** the dialog runs with the RU catalog
- **WHEN** the user types "theme" in the search field
- **THEN** only «Интерфейс» stays in the nav and only the «Тема оформления» row is shown, with no highlighted substring because the visible label does not contain the query

#### Scenario: Search finds Russian labels while the UI runs in English
- **GIVEN** the dialog has been relabelled to the EN catalog
- **WHEN** the user types «тема» in the search field
- **THEN** the "Interface" section is shown with the "Color theme" row, because the index covers both catalogs, and no substring is highlighted

#### Scenario: Enter never targets a hidden row
- **GIVEN** the updater is unsupported, so the update-check row is not rendered
- **WHEN** the user types «обновление» in the search field
- **THEN** «О программе» does not match and Enter leaves the active section unchanged, because the index covers only the rendered rows

#### Scenario: Search hides sections without matches
- **GIVEN** the Settings dialog is open
- **WHEN** the user types a query that only matches the notification rows
- **THEN** the nav shows only «Уведомления» and the content pane shows only the matching switches

#### Scenario: No results state
- **GIVEN** the Settings dialog is open
- **WHEN** the user types a query that matches no row
- **THEN** the nav is empty and a localized no-results message replaces the rows

#### Scenario: Enter jumps to the first match
- **GIVEN** a query whose first match is in a section other than the active one
- **WHEN** the user presses Enter in the search field
- **THEN** the dialog switches to that section, scrolls the first matching row into view, and does not submit the form

#### Scenario: Esc clears the query first
- **GIVEN** the search field contains a query
- **WHEN** the user presses Esc
- **THEN** the query is cleared and the full nav is restored while the dialog stays open
- **AND** pressing Esc again with an empty query closes the dialog

#### Scenario: Dirty markers follow the loaded config
- **GIVEN** the dialog loaded with `theme: "auto"` and `notify_on_ready: true`
- **WHEN** the user changes the theme and turns `notify_on_ready` off
- **THEN** «Интерфейс» and «Уведомления» show a dot in the nav and the footer shows «Изменено: 2»

#### Scenario: A coerced engine is not dirty
- **GIVEN** the saved engine is unavailable, so the form is coerced to the fallback engine with the alert
- **WHEN** the dialog finishes loading
- **THEN** «Движок и голос» has no dirty dot and the footer shows «Изменено: 0»

#### Scenario: Saving unchanged values submits the same patch
- **GIVEN** the dialog loaded the saved config and the user changed nothing
- **WHEN** the user clicks «Сохранить»
- **THEN** `commands.updateConfig` receives the same `UIConfigPatch` as the pre-sections dialog (every form field, no `engine` when coerced) and the dialog closes

#### Scenario: Reset restores factory defaults
- **GIVEN** the user changed the theme in the dialog
- **WHEN** the user clicks «Сброс»
- **THEN** the form values return to the factory defaults, including the engine reset to the storage default `"teratts"` through the same availability coercion as the initial load (the form falls back to Piper when the TeraTTSv2 bundle is missing), the dirty markers reflect the difference from the loaded config, and field errors are cleared

#### Scenario: Footer stays visible
- **GIVEN** the content pane is scrolled to the bottom of a long section
- **WHEN** the user looks at the dialog
- **THEN** «Сброс» and «Сохранить» are still visible without scrolling, because the footer sits outside the scrolling pane

#### Scenario: Narrow window collapses the nav
- **GIVEN** the window is narrower than about 700 px
- **WHEN** the Settings dialog opens
- **THEN** the side nav is replaced by a section `Select` above the content, and choosing a section there shows its rows

#### Scenario: Nested dialogs return to the originating section
- **GIVEN** the user opened the cleanup dialog from «Хранилище» or the dictionary editor from «Словарь»
- **WHEN** the nested dialog is closed
- **THEN** the Settings dialog is still on that section, and the cleanup dialog keeps its `closeOnEscape` gating
