# Design: optional-tray-on-close

## Context

Tauri's `RunEvent` loop in `src-tauri/src/lib.rs` intercepts the main window's
`CloseRequested` and hides the window, and separately blocks `ExitRequested`
whenever no real quit was requested. The tray menu's "Выход" sets
`AppState.user_quit` (an `AtomicBool`), which is the only current bypass.

## Decisions

### Store the flag in `UIConfig`, not in a separate store

The backend already owns `config.json`, `get_config`, `update_config`, and
`UIConfigPatch`, and several behavior switches already live there
(`preview_dialog_enabled`, `code_block_mode`). A second store would need its own
IPC and test harness for no gain. The field is a plain `bool` with
`#[serde(default)]`, so old configs parse without a migration and get `false`.

Rejected: a frontend-only `localStorage` flag. The Rust runtime handles the
close, so it must read the setting; a webview round-trip on close would race
with the window teardown and fail when the webview is gone.

### Re-read the config on every close

`should_hide_to_tray` loads `config.json` through `AppState.storage` each time a
close is requested. This mirrors how `code_block_mode` is read and makes the
Settings switch effective immediately, without caching a second copy in
`AppState` that could drift from disk after an external edit or a failed save.
The config file is small and the read happens only on a close request.

Rejected: caching the flag in `AppState` (e.g. an `AtomicBool`). It would need
to be updated by `update_config`, re-initialized at startup, and kept in sync
with the value loaded by other commands — duplicated state for a once-per-close
read.

### One gate for both `CloseRequested` and `ExitRequested`

`ExitRequested` is what actually keeps the process alive after the last window
is gone; gating only `CloseRequested` would leave a windowless process that can
never be reopened. Both handlers call the same `should_hide_to_tray` helper and
are bypassed by `user_quit`, preserving the tray "Выход" path.

### The tray icon follows the setting (reversed during implementation)

Originally the design kept the tray icon and its actions unconditional: the
tray provides clipboard ingestion independent of how the app is closed, so
hiding the icon with the setting off would remove those actions from users who
never close the window.

The implementation reversed this decision: with the setting off the app quits
together with its window, so a tray icon would have nothing to restore, and
users reported the icon lingering in the notification area while the switch was
off. The icon is now created only while the setting is on and removed when it
is turned off, which also takes the menu and the clipboard actions with it.
The trade-off is accepted: the clipboard actions are reachable through the
window while the app is open, and the icon is restored the moment the setting
is turned on again.

### Default off, including for existing configs

`#[serde(default)]` means an existing `config.json` without the key starts
quitting on close after the upgrade. This is intended: the request defines off
as the default, and a per-install migration would make the setting's meaning
depend on installation history. The Settings description makes the consequence
explicit, and the tray "Выход" item still exists for users who enable hiding.

## Risks

- A user who relied on hide-on-close loses background playback on upgrade until
  they enable the switch. Mitigated by the changelog note and the switch's
  description; no data is lost.
- The close path performs a small synchronous config read on the event-loop
  thread. The file is a few hundred bytes and the read only happens on close.
