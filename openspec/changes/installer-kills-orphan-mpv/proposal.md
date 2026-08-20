# Proposal: installer-kills-orphan-mpv

## Why

Issue #211. On Windows the mpv subprocess (spawned via tauri-plugin-mpv)
outlives the app whenever the app is force-killed instead of exiting
gracefully — which is exactly what the updater-launched NSIS installer
does. The orphaned mpv.exe keeps `$INSTDIR\mpv\mpv.exe` locked, so the
install fails with "Error opening file for writing" and the user has to
retry after killing mpv by hand. Observed on the 0.3.0 → 0.3.1
auto-update and on manual reinstall of a running app.

## What

Two complementary fixes:

- **App side (deterministic for the auto-update flow):** the frontend
  destroys the mpv subprocess (new `shutdown_player_for_update` command)
  right before `Update.downloadAndInstall()` — before the installer can
  force-kill the app.
- **Installer side (covers manual reinstall and uninstall of a running
  app):** NSIS installer hooks (`NSIS_HOOK_PREINSTALL`,
  `NSIS_HOOK_PREUNINSTALL`) kill mpv.exe processes whose executable path
  is under `$INSTDIR`, leaving any standalone mpv the user may have
  alone.
