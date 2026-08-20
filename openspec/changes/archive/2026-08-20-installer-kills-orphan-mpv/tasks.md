# Tasks: installer-kills-orphan-mpv

- [x] Add the `shutdown_player_for_update` command (mark destroyed +
      best-effort `mpv().destroy()`, mirroring the `RunEvent::Exit`
      cleanup) and register it.
- [x] Call it from `installAndRelaunch` before `downloadAndInstall()`;
      pin the call in `updater.test.ts`.
- [x] Add `src-tauri/windows/hooks.nsh` with `NSIS_HOOK_PREINSTALL` /
      `NSIS_HOOK_PREUNINSTALL` killing mpv.exe path-filtered by
      `$INSTDIR`; wire `bundle.windows.nsis.installerHooks` in
      `tauri.conf.json`.
- [x] Verify on the Win10 VM with a release build: auto-update no longer
      hits "Error opening file for writing" for mpv.exe (needs a
      release build — CI uploads no installer artifacts).
- [x] Archive the change.
