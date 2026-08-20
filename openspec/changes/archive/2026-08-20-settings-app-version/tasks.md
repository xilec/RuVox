# Tasks: settings-app-version

## Frontend: Settings version display

- [x] Import `getVersion` from `@tauri-apps/api/app` in `src/dialogs/Settings.tsx`
- [x] Add `appVersion` state; fetch it in the open effect (mirroring
      `getCacheDir`), falling back to `''` on failure
- [x] Render an "О приложении" section after the theme selector with
      `Версия: {appVersion || '—'}`
- [x] `pnpm typecheck` green

## Backend: tray tooltip version

- [x] `src-tauri/src/tray/mod.rs`: `.tooltip(format!("RuVox v{}", app.package_info().version))`
- [x] `cargo check` (or `cargo build`) green

## OpenSpec

- [x] Delta specs: `ui` (Settings version display), `tray` (tooltip version)
- [x] `openspec change validate settings-app-version` green
- [x] Verify change vs implementation, then archive (syncs deltas into
      `openspec/specs/`)

## Gates

- [x] `pnpm test:unit` green (157 tests)
- [x] `just lint` green (eslint)
- [ ] Manual dev pass: open Settings → "О приложении" shows `Версия: 0.3.1`;
      hover tray → tooltip `RuVox v0.3.1`
