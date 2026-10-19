# Delta: tray

## MODIFIED Requirements

### Requirement: Hide to tray on window close

When the user closes the main window (the `X` button or a window-manager
close), the system SHALL hide the window into the system tray only when
`UIConfig.hide_to_tray_on_close` is `true` (off by default, including for
configs written before the field existed). With the setting enabled the system
SHALL intercept `CloseRequested`, prevent the close, hide the window, and set
`skip_taskbar`, so the application keeps running in the background (synthesis,
playback) instead of exiting. The setting SHALL be re-read from the persisted
config on every close request, so toggling it in the Settings dialog applies
without a restart.

The same gate SHALL govern the implicit `ExitRequested` that follows the last
window closing: with the setting enabled the exit SHALL be prevented, and with
it disabled the exit SHALL proceed so the application quits together with the
window. The intercept MUST be bypassed when a real quit was requested through
the tray's "Выход" item (the `user_quit` flag).

The tray icon, its context menu, and the clipboard actions SHALL remain
available regardless of the setting.

#### Scenario: Close button hides the window

- GIVEN the application is running with the main window visible and
  `hide_to_tray_on_close` enabled
- WHEN the user clicks the window's close button
- THEN the window disappears from the screen and taskbar, and the process
  keeps running with the tray icon present

#### Scenario: Close button quits when hiding is disabled

- GIVEN the application is running with the default configuration
  (`hide_to_tray_on_close: false`)
- WHEN the user clicks the window's close button
- THEN the window closes, the implicit exit is not prevented, and the process
  terminates instead of staying in the tray

#### Scenario: Quit bypasses the intercept

- GIVEN the user chose "Выход" in the tray menu
- WHEN the window close is processed
- THEN the close is not prevented and the application exits
