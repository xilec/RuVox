# Tasks: settings-reveal-logs-folder

- [x] Add a `get_log_dir()` command in `src-tauri/src/commands/mod.rs` that
  resolves the per-app log dir via `app.path().app_log_dir()`, creates it if
  missing, and returns the absolute path; register it in `generate_handler!`
  in `src-tauri/src/lib.rs`.
- [x] Add a `getLogDir()` wrapper to `src/lib/tauri.ts`.
- [x] Add a "Логи" section in `src/dialogs/Settings.tsx` with the resolved
  path and an "Открыть папку" button that calls `revealItemInDir`, with an
  error notification on failure.
- [x] Document `get_log_dir()` in the ipc-commands spec and the Settings
  reveal action in the logging spec (delta specs below).
- [x] Manual check: dev build opens Settings, the Logs section shows the path,
  and "Открыть папку" reveals the log directory in the OS file manager.
- [x] Archive the change (sync delta specs into `openspec/specs/`).
