# Delta spec: ui

## MODIFIED Requirements

### Requirement: Settings dialog

The system SHALL provide a modal Settings dialog (`src/dialogs/Settings.tsx`) built on `@mantine/form::useForm`, loading `commands.getConfig()` and `commands.getAvailableEngines()` whenever it opens, and submitting a `UIConfigPatch` (only the form fields) via `commands.updateConfig`.

The form SHALL expose: TTS engine (Piper / Silero / «Silero (нативный)», with unavailable engines disabled and an alert when the saved engine was coerced to Piper), Piper voice or Silero speaker depending on the engine, sample rate, `notify_on_ready`, `notify_on_error`, `preview_dialog_enabled`, `max_cache_size_mb` (minimum 100 MB), and theme (Светлая/Тёмная/Авто). Applying a new theme SHALL push it into Mantine's color-scheme manager immediately.

When «Silero (нативный)» is unavailable because the model bundle is not downloaded, the form SHALL show a download action next to the engine option; triggering it calls `commands.downloadSileroNativeBundle()` and displays live progress from the `bundle_download_*` events.

When the form's engine was coerced away from the saved engine because that engine is currently unavailable, and the user did not pick another engine, saving the dialog SHALL omit `engine` from the `UIConfigPatch` so the saved (or default) engine value is preserved instead of persisting the temporary fallback.

A "Очистить кэш…" button SHALL open a nested `CleanupCacheModal` with a target-MB input (disabled by "Очистить полностью"), a "Удалять тексты" checkbox, and a red warning when both full cleanup and text deletion are selected; confirmation calls `commands.clearCache({ mode, delete_texts })`.

#### Scenario: Save applies config and theme

- GIVEN the Settings dialog is open with theme changed to "Тёмная"
- WHEN the user clicks "Сохранить"
- THEN `commands.updateConfig` receives the patch, the UI switches to the dark scheme without reload, and a success notification is shown

#### Scenario: Engine availability gates selection

- GIVEN `getAvailableEngines` reports Silero as unavailable
- WHEN the Settings dialog opens
- THEN the Silero option is disabled with its reason shown, and a config saved with Silero is coerced to Piper with a yellow alert

#### Scenario: Save while coerced preserves the saved engine

- GIVEN the saved engine is unavailable and the form was coerced to the fallback engine
- WHEN the user saves the dialog without picking an engine
- THEN the `UIConfigPatch` sent to `commands.updateConfig` does not contain `engine`, so the previously saved (or default) engine value stays in effect

#### Scenario: Native engine offers bundle download

- GIVEN `getAvailableEngines` reports Silero Native as unavailable due to a missing bundle
- WHEN the Settings dialog opens
- THEN the «Silero (нативный)» option is disabled with its reason shown and a download button is offered; clicking it starts the download and shows progress until `bundle_download_finished`

#### Scenario: Full cache cleanup warns

- GIVEN the cleanup sub-modal is open
- WHEN the user checks both "Очистить полностью" and "Удалять тексты"
- THEN a red irreversibility warning appears and the confirm button turns red
