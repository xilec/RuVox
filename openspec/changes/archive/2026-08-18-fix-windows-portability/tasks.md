# Tasks: fix-windows-portability

## 1. Compile blockers

- [x] 1.1 `#[cfg(unix)]`-gate `reap_orphan_mpv` in `src-tauri/src/lib.rs` (function, its call site in `run()`, and the `libc` import if unused on Windows)
- [x] 1.2 `#[cfg(unix)]`-gate the `libc::kill` ttsd-shutdown path in `src-tauri/src/tts/mod.rs` (and its surrounding context; on Windows the supervisor drop path must still compile and be a no-op)
- [x] 1.3 Verify: Linux `cargo check`/`clippy` stays clean. The Windows-target check cannot run locally (vendored C deps fight the cargo-xwin cross toolchain — see `tmp/win-check/blockers.md`); it is deferred to the build-only Windows CI job in `windows-installer-and-release` (task 4.4 there)

## 2. mpv executable resolution (D2)

- [x] 2.1 Add `resolve_mpv_path(resource_dir: &Path) -> PathBuf` in `src-tauri/src/player/mod.rs`: Windows — `<resource_dir>/mpv/mpv.exe` if it exists, else `"mpv"`; Linux — `"mpv"`
- [x] 2.2 Wire it into `Player::init_mpv` (replace `MpvConfig { path: "mpv" }`, resolving the resource dir from `AppHandle`)
- [x] 2.3 Unit tests (tempdir): bundled present → bundled path; bundled absent → PATH fallback; Linux branch unchanged

## 3. Windows startup environment (D3)

- [x] 3.1 Add a `#[cfg(windows)]` startup function (called at the top of `main`, before Tokio/Tauri spawn threads) that sets `PIPER_ESPEAKNG_DATA_DIRECTORY` to the bundled `espeak-ng-data/` resource dir when it exists; comment the unsafe/before-threads invariant
- [x] 3.2 Verify Linux startup does not touch the variable (existing behavior)

## 4. ttsd availability probe (D4)

- [x] 4.1 In `src-tauri/src/tts/availability.rs`, map `uv --version` spawn errors (binary missing) to `available: false` with the existing Russian reason
- [x] 4.2 Unit test: probe with a guaranteed-nonexistent binary name reports unavailable without erroring the command

## 5. Unix-only test gating (D5)

- [x] 5.1 `#[cfg(unix)]`-gate tests in `src-tauri/src/tts/supervisor.rs` that spawn `cat`/`tail` and use `/tmp` paths (plus any other unix-only test helpers found during the target check)

## 6. Gates

- [x] 6.1 `nix develop -c just lint` and `nix develop -c just test` green on Linux (incl. `cargo update -p h2` 0.4.15 → 0.4.16 for RUSTSEC-2026-0258)
- [x] 6.2 `pnpm dlx @fission-ai/openspec validate --strict` passes for this change
