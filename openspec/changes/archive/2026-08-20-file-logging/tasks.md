# Tasks: file-logging

- [x] Add `tauri-plugin-log` (with the `tracing` feature) to
  `src-tauri/Cargo.toml` and register it in the app builder: `LogDir`
  target always, `Stdout` in debug builds, `RotationStrategy::KeepSome`
  with a per-file size cap, level filter from `RUST_LOG` with `info`
  default.
- [x] Add the `log:default` permission to
  `src-tauri/capabilities/default.json` (or the existing capabilities
  file).
- [x] Verify on the Win10 VM: after install + launch + one synthesis,
  `%LOCALAPPDATA%\com.ruvox.app\logs\` contains a log file with startup
  and engine-selection entries.
- [x] `just test` + `just lint` green.
- [x] Archive the change (sync delta specs into `openspec/specs/`).
