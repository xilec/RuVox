# Proposal: settings-reveal-logs-folder

Follow-up to #202 (file-logging, targeted at v0.3.0). The
`file-logging` change explicitly deferred a "Reveal log folder" UI entry
as a nice-to-have; this change delivers it.

## Problem

`file-logging` now writes diagnostic logs to the per-app log dir, but
there is no way for a user to reach that folder from the app. When a user
hits a problem they cannot grab the log file to attach to a bug report,
and there is no discoverable hint of where logs live. Power users who
know the path still have to open a terminal or file manager manually.

## Change

Expose the log directory to the frontend and add a one-click reveal from
Settings:

- **Backend:** add a `get_log_dir()` command that returns the absolute
  per-user log directory resolved via Tauri's `app_log_dir()` (the same
  directory `tauri-plugin-log` writes its rotated files into). The command
  creates the directory if it does not yet exist, so the frontend can
  reveal a real path even before the first log line is flushed.
- **Frontend:** add a `getLogDir()` wrapper in `src/lib/tauri.ts`.
- **UI:** add a "Логи" section in Settings that shows the resolved path
  and an "Открыть папку" button. The button calls `revealItemInDir` from
  `@tauri-apps/plugin-opener` to open the directory in the OS file
  manager, with an error notification if the reveal fails.

This mirrors the existing cache-dir reveal already present in Settings.

## Out of scope

- A built-in log viewer inside the app.
- Copying / uploading logs (e.g. to a paste service) from Settings.
- A tray-menu entry for the same action (Settings-only for now).
