# Tray Specification

## Purpose

Covers the system-tray integration: hiding the main window into the tray
instead of exiting, the tray context menu and its actions, tray-icon click
behavior, and the mpv re-initialization required when the window is shown
again.

## Requirements

### Requirement: Hide to tray on window close

When the user closes the main window (the `X` button or a window-manager
close), the system SHALL intercept `CloseRequested`, prevent the close, hide
the window, and set `skip_taskbar`, so the application keeps running in the
background (synthesis, playback) instead of exiting. The system SHALL also
block the implicit `ExitRequested` that follows the last window closing. The
intercept MUST be bypassed when a real quit was requested through the tray's
"Выход" item (the `user_quit` flag).

#### Scenario: Close button hides the window

- GIVEN the application is running with the main window visible
- WHEN the user clicks the window's close button
- THEN the window disappears from the screen and taskbar, and the process
  keeps running with the tray icon present

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
