# Proposal: frontend-update-check-logging

## Why

The manual "Проверить обновления" outcome is shown only as a toast and the
startup check is fully silent: `tauri-plugin-updater` logs only endpoint
errors, so a successful check (or an up-to-date result) leaves no trace in
the log file. When diagnosing update issues from a user's log we cannot tell
whether a check even ran.

## What

- The frontend logs every update-check outcome to the same log file via the
  `tauri-plugin-log` JS API (`@tauri-apps/plugin-log`): "up to date",
  "update available: <version>", and check failures (including the silent
  startup path) with the error message.

## Scope

- `src/lib/updater.ts` (both `checkForUpdatesOnStartup` and
  `checkForUpdatesManual`), plus the `@tauri-apps/plugin-log` dependency.
- No UI changes; toasts stay as they are.
