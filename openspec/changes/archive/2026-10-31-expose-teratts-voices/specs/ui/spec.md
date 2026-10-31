## MODIFIED Requirements

### Requirement: Settings dialog

The system SHALL provide a modal Settings dialog (`src/dialogs/Settings.tsx`) built on `@mantine/form::useForm`, loading `commands.getConfig()`, `commands.getAvailableEngines()` and `commands.getTerattsVoices()` whenever it opens, and submitting a `UIConfigPatch` (only the form fields) via `commands.updateConfig`.

The form SHALL expose: TTS engine (Piper / Silero / «Silero (нативный)» / «TeraTTSv2», with unavailable engines disabled and an alert when the saved engine was coerced to Piper), Piper voice or Silero speaker depending on the engine, the TeraTTS voice when that engine is selected (every id returned by `commands.getTerattsVoices()`, with `ru_m5` as the default), sample rate, `notify_on_ready`, `notify_on_error`, `preview_dialog_enabled`, `hide_to_tray_on_close` (the «Сворачивать в трей при закрытии окна» switch, off by default), `max_cache_size_mb` (minimum 100 MB), theme (Светлая/Тёмная/Авто), a language selector (Русский / English) bound to `UIConfig.language`, and a code block narration selector («Кратко» / «Читать полностью») bound to `UIConfig.code_block_mode` (`"brief"` / `"read"`), initialized from the loaded config and included in the submitted `UIConfigPatch` when changed. Applying a new theme SHALL push it into Mantine's color-scheme manager immediately. Choosing a language SHALL update the localization store immediately (the whole UI re-labels without reload) and include `language` in the submitted `UIConfigPatch`.

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
