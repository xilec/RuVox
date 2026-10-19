# Proposal: optional-tray-on-close

## Why

Closing the main window currently always hides the app into the system tray and
keeps the process running. That is the right behavior for background narration,
but it is surprising for users who expect the window's close button to quit: the
window disappears, audio may keep playing, and the only way out is the tray
menu's "Выход". The tray icon and its clipboard actions are useful on their own,
so the hide-on-close behavior should be a choice rather than a hard-coded
consequence of the tray existing.

## What Changes

- Add `UIConfig.hide_to_tray_on_close` (boolean, default `false`), persisted in
  `config.json` and patchable through `update_config`.
- Gate the window-close intercept on that setting: with it off, `CloseRequested`
  is not prevented and the implicit `ExitRequested` is allowed, so closing the
  window quits the app. With it on, the existing hide-to-tray behavior is kept.
- Re-read the setting on every close so toggling it in Settings applies without
  a restart.
- Expose the setting as a Settings switch («Сворачивать в трей при закрытии
  окна», off by default) with RU/EN catalog strings.
- Keep the tray icon, its menu, and the "read clipboard now / queue clipboard"
  actions unchanged regardless of the setting.

## Non-goals

- No minimize-to-tray on the minimize button or on window blur — only the close
  request is affected.
- No quit confirmation prompt when the setting is off.
- No migration that preserves the old behavior for existing installations: the
  default is off for everyone (an existing `config.json` without the field gets
  `false`), matching the requested default.
- No change to the tray menu structure, tray icon visibility, or clipboard
  actions.
- No new settings store: the flag lives in the existing `UIConfig`/`config.json`.

## Capabilities

### Modified Capabilities

- `tray`: the "Hide to tray on window close" requirement becomes conditional on
  the persisted setting and gains a quit-on-close scenario.
- `ui`: the Settings dialog requirement lists the new switch.
- `ipc-commands`: `UIConfig` gains `hide_to_tray_on_close` with a `false`
  default, including for configs written before the field existed.

## Impact

- `src-tauri/src/storage/schema.rs` — `UIConfig` + `UIConfigPatch` fields,
  defaults, serde-default and patch tests.
- `src-tauri/src/commands/mod.rs` — `apply_config_patch` wiring and the
  exhaustive patch-field table.
- `src-tauri/src/lib.rs` — `should_hide_to_tray` gate used by the
  `CloseRequested` and `ExitRequested` handlers, plus its unit test.
- `src/lib/tauri.ts` — `UIConfig` type.
- `src/dialogs/Settings.tsx` — form field, loaded config, submitted patch, and
  the interface-section switch.
- `src/i18n/{ru,en}.ts` — switch label and description.
- `src/dialogs/Settings.test.tsx` — config fixture and toggle submission test.
- `CHANGELOG.md` `[Unreleased]`.
