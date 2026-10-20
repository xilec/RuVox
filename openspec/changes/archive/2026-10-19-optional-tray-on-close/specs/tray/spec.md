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

The tray icon SHALL exist only while the setting is enabled. With
`hide_to_tray_on_close` off the app quits together with its window, so the
system SHALL NOT show a tray icon and the tray menu and clipboard actions
SHALL be unreachable. The icon SHALL be created at startup only when the
persisted setting is enabled, and `update_config` SHALL create or remove it
after the config is saved, so toggling the Settings switch applies without a
restart. With the setting enabled the icon, its context menu, and the
clipboard actions SHALL remain available.

#### Scenario: No tray icon by default

- GIVEN the application starts with the default configuration
  (`hide_to_tray_on_close: false`)
- WHEN the main window is open
- THEN no tray icon is shown in the notification area

#### Scenario: Enabling the switch adds the icon

- GIVEN the Settings dialog is open with `hide_to_tray_on_close: false`
- WHEN the user enables the switch and saves
- THEN the tray icon appears without a restart, and a later window close hides
  the window instead of quitting

#### Scenario: Disabling the switch removes the icon

- GIVEN the application is running with `hide_to_tray_on_close: true`
- WHEN the user disables the switch and saves
- THEN the tray icon disappears without a restart, and a later window close
  quits the application

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
