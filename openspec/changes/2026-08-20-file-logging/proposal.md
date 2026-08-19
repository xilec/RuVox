# Proposal: file-logging

Fixes #202 (targeted at v0.3.0). Found during the v0.3.0 VM verification
pass: the installed Windows build writes no logs anywhere.

## Problem

The backend uses `tracing::info!/warn!/error!` throughout (~84 call
sites, including the ttsd subprocess output relay), but no subscriber is
ever installed — `src-tauri` depends on `tracing` only, and neither
`main.rs` nor `lib.rs` initializes one. Every tracing event is silently
discarded on every platform. On Windows release builds there is not even
a console (`windows_subsystem = "windows"`), so stdout would be useless
anyway. Result: user-reported problems are undebuggable — there is no
log file to attach to a bug report.

## Change

Add `tauri-plugin-log` (the standard Tauri 2 logging plugin, built on
`log` + `fern`) and register it in the app builder:

- **Targets:** `LogDir` (always) + `Stdout` (debug builds only).
- **Log dir:** the Tauri per-app log dir
  (`%LOCALAPPDATA%\com.ruvox.app\logs` on Windows,
  `~/.local/share/com.ruvox.app/logs` on Linux) — a second data location
  next to the storage root; documented in the new spec.
- **Rotation:** keep the current and a few previous log files
  (`RotationStrategy::KeepSome`), with a size cap per file.
- **Level:** `RUST_LOG`-style filter via env, default `info`.

`tauri-plugin-log` is built on `log` + `fern`; enabling its `tracing`
feature installs a subscriber layer that forwards `tracing` records into
the plugin's targets, so the existing `tracing::info!/warn!/error!` call
sites start producing output unchanged. The `log:default` capability is
added to the window capabilities file.

## Out of scope

- Forwarding frontend (webview) console logs to the file — can be added
  later via the plugin's webview target.
- A "Reveal log folder" UI entry (Settings/tray) — nice-to-have, tracked
  in #202's acceptance notes but not required for v0.3.0.
- Changing the storage root layout (#200 stays as shipped).
