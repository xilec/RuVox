# Tray Specification

## Purpose

Covers the system-tray integration: hiding the main window into the tray
instead of exiting, the tray context menu and its actions, tray-icon click
behavior, and the mpv re-initialization required when the window is shown
again.

## Requirements

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

The window SHALL be hidden into the tray only while a tray icon exists: if the
icon could not be created, the window close SHALL quit the application, so the
user never loses a window that nothing can restore. When saving the setting
fails to create the icon, the system SHALL roll the setting back to its
previous value and report the failure, so the stored state always matches the
tray that actually exists.

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

#### Scenario: Closing quits when the tray icon is missing

- GIVEN `hide_to_tray_on_close` is enabled but the tray icon could not be
  created
- WHEN the user closes the main window
- THEN the window closes and the application quits instead of hiding into a
  tray that does not exist

#### Scenario: The setting is rolled back when the icon cannot be created

- GIVEN the system has no usable tray icon
- WHEN the user enables the switch and saves
- THEN the save reports a failure, the stored setting stays off, and the
  Settings switch shows the stored state

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

### Requirement: Tray context menu

The tray icon SHALL expose a menu with exactly the items "Открыть окно",
"Добавить", and "Выход" (in that order, separated by separators), with
"Открыть окно" on top so that a click followed by picking the first item is
the shortest path back to the window. On Linux (libayatana-appindicator) the
menu SHALL NOT be bound to left click (`show_menu_on_left_click(false)`),
because the platform opens the menu on every click.

#### Scenario: Menu structure

- GIVEN the application is running
- WHEN the user opens the tray context menu
- THEN the items are "Открыть окно", "Добавить", "Выход" in that order

### Requirement: Show window action

The "Открыть окно" item SHALL restore the main window: clear `skip_taskbar`,
unminimize, show, and focus it. Because `tauri-plugin-mpv` destroys the mpv
subprocess whenever the main window's close is requested, showing the window
SHALL lazily re-initialize mpv via `ensure_mpv_alive` and emit
`playback_stopped` so the frontend discards stale player state.

#### Scenario: Restore from tray

- GIVEN the window was hidden to the tray
- WHEN the user picks "Открыть окно"
- THEN the window reappears focused and playback works again without
  restarting the app

### Requirement: Tray add action

The "Добавить" item SHALL read the system clipboard on the Rust side
(`arboard` on a blocking thread), create a queue entry from non-empty text,
emit `entry_updated`, and start background synthesis with
`play_when_ready = true`, all without requiring a webview round-trip. Empty
or unreadable clipboard content SHALL be logged and skipped without creating
an entry.

#### Scenario: Add from the tray

- GIVEN the window is hidden and the clipboard contains text
- WHEN the user picks "Добавить"
- THEN a new entry appears in the queue, synthesis starts, and playback
  begins automatically when the entry is ready

#### Scenario: Empty clipboard

- GIVEN the clipboard is empty
- WHEN the user picks "Добавить"
- THEN no entry is created and a warning is logged

### Requirement: Tray icon click behavior

A single left click (on button release) or a left double click on the tray
icon SHALL show the main window, because libayatana-appindicator on KDE/GNOME
often does not propagate double-click events. Filtering on the `Up` button
state ensures the action fires exactly once per click.

#### Scenario: Single click shows the window

- GIVEN the window is hidden in the tray
- WHEN the user left-clicks the tray icon once
- THEN the main window is shown exactly once

### Requirement: Clean player shutdown on exit

On application exit the system SHALL mark the player as destroyed before
destroying the mpv instance, so in-flight commands (position-emitter ticks,
tray callbacks) short-circuit instead of panicking on the removed mpv
instance.

#### Scenario: Exit during playback

- GIVEN an entry is playing
- WHEN the user quits via "Выход"
- THEN mpv is destroyed and no panic occurs from in-flight player commands

### Requirement: Tray tooltip shows the app version

The tray icon tooltip SHALL include the application version, formatted as
`RuVox v<version>` (e.g. `RuVox v0.3.1`), so bug reports can quote the version
directly from the tray. The version SHALL be read from the app package info
(`app.package_info().version`, the `version` field of `tauri.conf.json`) at tray
init.

#### Scenario: Tooltip names the version

- GIVEN the application is running
- WHEN the user hovers the tray icon
- THEN the tooltip reads `RuVox v<version>` matching the built `tauri.conf.json`
  version, not the bare `RuVox`

