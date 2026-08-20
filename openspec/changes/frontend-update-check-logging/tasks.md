# Tasks: frontend-update-check-logging

- [x] Add `@tauri-apps/plugin-log` to the frontend dependencies.
- [x] Log every update-check outcome (up to date / update available /
      failure) from `src/lib/updater.ts`, both the startup and the manual
      path; startup failures go to the log while staying silent in the UI.
- [x] Mock `@tauri-apps/plugin-log` in `updater.test.ts` and pin the log
      calls.
- [ ] Verify on the Win10 VM: after a manual "Проверить обновления", the
      log file contains an `update check (manual)` entry with the webview
      target. (Needs a release build — CI uploads no installer artifacts;
      verify with the v0.3.1 installer.)
- [ ] Archive the change.
