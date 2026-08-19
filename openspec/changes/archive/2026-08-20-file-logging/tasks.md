# Tasks: file-logging

- [ ] Add `tauri-plugin-log` (with the `tracing` feature) to
  `src-tauri/Cargo.toml` and register it in the app builder: `LogDir`
  target always, `Stdout` in debug builds, `RotationStrategy::KeepSome`
  with a per-file size cap, level filter from `RUST_LOG` with `info`
  default.
- [ ] Add the `log:default` permission to
  `src-tauri/capabilities/default.json` (or the existing capabilities
  file).
- [ ] Verify on the Win10 VM: after install + launch + one synthesis,
  `%LOCALAPPDATA%\com.ruvox.app\logs\` contains a log file with startup
  and engine-selection entries.
- [ ] `just test` + `just lint` green.
- [ ] Archive the change (sync delta specs into `openspec/specs/`).
