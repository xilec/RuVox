# Delta: ui

## ADDED Requirements

### Requirement: Settings shows the app version

The Settings dialog SHALL display the application version in the bottom-right of
the dialog (the footer, right-aligned) so it is always visible (not gated behind
the Windows-only updater). The version SHALL be read from the app manifest via
Tauri's `getVersion()` (which resolves the `version` field of `tauri.conf.json`
at build time) and shown as `Версия: <version>`.

The version SHALL be fetched whenever the dialog opens (alongside `getConfig` and
`getCacheDir`) and SHALL fall back to `—` if the IPC call fails, so a backend or
version read error never blocks the dialog.

#### Scenario: Version is shown when Settings opens

- GIVEN the application has started
- WHEN the user opens Settings
- THEN the bottom-right footer shows `Версия: <version>` matching the built
  `tauri.conf.json` version (e.g. `0.3.1`)

#### Scenario: Version read failure degrades gracefully

- GIVEN `getVersion()` fails (e.g. IPC unavailable)
- WHEN the user opens Settings
- THEN the version line shows `—` and the rest of the dialog still works
