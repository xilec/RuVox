# Tasks: windows-data-dir

- [x] Add a shared data-root resolver (`src-tauri/src/paths.rs` or
  equivalent): on Windows `dirs::data_local_dir()/<identifier>`, on other
  OSes the existing roots (storage: `dirs::cache_dir()/ruvox`, voices:
  `dirs::data_local_dir()/ruvox`). The Windows dir name MUST come from a
  single constant tied to `tauri.conf.json` `identifier`.
- [x] Switch `StorageService::new()` to the resolver.
- [x] Switch `build_engine` voices root (`src-tauri/src/lib.rs:216`) to the
  resolver.
- [x] Unit test for the Windows dir-name join helper (pure function, runs
  on Linux).
- [x] `just test` + `just lint` green; cross-check
  `cargo check --target x86_64-pc-windows-msvc` if the toolchain allows.
  (Windows-target check skipped locally: no windows target installed and
  xwin not set up in this workspace; the CI "Windows build (no bundle)"
  job on windows-latest covers compilation of the windows arm.)
- [ ] Archive the change (sync delta specs into `openspec/specs/`).
- [ ] VM verification (manual, after the release rebuild): install v0.3.0
  on the Win10 VM, synthesize, uninstall with "Delete the application
  data" checked → `%LOCALAPPDATA%\com.ruvox.app` and the install dir are
  gone; uninstall without the checkbox → data root survives.
