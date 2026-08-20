# Proposal: settings-app-version

Part of #94 (tag-driven release workflow + version in UI). This change covers
the UI/tray half of #94: making the app version visible so bug reports can
quote a specific build. The CHANGELOG-driven release notes (the other half of
#94) are handled in `release.yml` and are out of scope here.

## Why

Bug reports against a specific version depend on the user knowing which version
they run. Today there is no About dialog and the tray tooltip is a static
`RuVox`, so the version is only discoverable by reading `tauri.conf.json` or
the installer filename. Issue #94 asked for the version to be shown in the UI
("About dialog or tray tooltip") for exactly this reason.

## What Changes

1. **Settings shows the app version.** The Settings dialog gains an
   "О приложении" section (always visible, not gated behind the Windows-only
   updater) that displays the version as `Версия: <version>`. The value comes
   from Tauri's `getVersion()` (resolves `tauri.conf.json` `version` at build
   time), fetched each time the dialog opens and falling back to `—` on IPC
   failure so a version read error never blocks the dialog.
2. **Tray tooltip shows the version.** The tray icon tooltip is formatted as
   `RuVox v<version>` (e.g. `RuVox v0.3.1`), read from `app.package_info().version`
   at tray init.

## Scope

- Frontend: `src/dialogs/Settings.tsx` — fetch version on open + render the
  "О приложении" section.
- Backend: `src-tauri/src/tray/mod.rs` — `format!("RuVox v{}", …)` in
  `TrayIconBuilder::tooltip`.
- OpenSpec deltas: `ui` (Settings version display), `tray` (tooltip version).

## Non-goals

- No new dedicated About dialog — `#94` allows "About dialog or tray tooltip";
  reusing Settings avoids a new route and keeps the version next to the
  updater check.
- No CHANGELOG-driven release notes — handled separately in `release.yml`.
- No version-based update gating beyond display.

## Risks

- `getVersion()` reads `tauri.conf.json` at build time; in dev it matches the
  dev version, which is the intended behavior. Low risk.
- The IPC call can fail (offline backend, dev tools); the `—` fallback keeps
  the dialog usable, matching the existing `getCacheDir` failure handling.
