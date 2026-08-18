# Proposal: fix-windows-portability

## Why

RuVox currently compiles only on Linux: `src-tauri` contains ungated
Unix-only code (`/proc`, `/tmp`, `libc::kill`) that fails compilation for
`x86_64-pc-windows-msvc`, and several runtime concerns (mpv location,
espeak-ng data directory, ttsd probe) assume a Linux environment. Epic #185
targets Windows 10 22H2+ / 11 — this change is its first step: make the
backend compile and behave correctly on Windows without changing any
Linux behavior.

## What Changes

- Gate Unix-only process-cleanup code behind `cfg(unix)`:
  `reap_orphan_mpv` (`src-tauri/src/lib.rs`) and the `libc::kill` call in
  the ttsd shutdown path (`src-tauri/src/tts/mod.rs`).
- Resolve the mpv executable per-OS: on Windows prefer the bundled
  `mpv/mpv.exe` resource directory (populated by the installer change),
  falling back to PATH; Linux keeps the current PATH lookup.
- On Windows startup, point `PIPER_ESPEAKNG_DATA_DIRECTORY` at the bundled
  `espeak-ng-data/` resource before any threads spawn (`std::env::set_var`
  is `unsafe` in edition 2024).
- Make the ttsd availability probe treat a failed `uv` spawn (binary
  missing — the normal case on Windows, where ttsd is not shipped) as
  "engine unavailable" with a Russian `reason`, not an error.
- Gate Unix-only test helpers (tests spawning `cat`/`tail`, `/tmp` paths)
  so `cargo check`/`clippy` for the Windows target stays clean.

## Capabilities

### New Capabilities

- `windows-runtime`: Windows-specific runtime adaptation — startup
  environment (espeak-ng data dir), per-OS subprocess cleanup semantics,
  and which engines/subprocesses exist on Windows (no ttsd).

### Modified Capabilities

- `playback`: mpv executable resolution becomes per-OS (bundled resource
  dir on Windows, PATH elsewhere); orphan-mpv reaping is Unix-only.
- `ipc-commands`: the Silero (ttsd) availability probe must degrade
  gracefully when the `uv` binary cannot be spawned at all.

## Impact

- **Code:** `src-tauri/src/lib.rs`, `src-tauri/src/player/mod.rs`,
  `src-tauri/src/tts/mod.rs`, `src-tauri/src/tts/availability.rs`,
  `src-tauri/src/main.rs` (or equivalent startup point), test modules in
  `src-tauri/src/tts/supervisor.rs`.
- **APIs:** no IPC contract changes; `get_available_engines` behavior on
  Windows only gets more robust.
- **Dependencies:** none added. `tauri.conf.json` gains no bundle changes
  yet — actual resource bundling (mpv, onnxruntime.dll, espeak-ng-data) is
  the follow-up installer change.
- **Platforms:** Linux behavior unchanged (verified by existing test
  suite); Windows target compiles and runs with graceful degradation.

## Non-goals

- Building the NSIS installer, bundling mpv/onnxruntime/espeak-ng-data,
  WebView2 bootstrapper, release workflow, tauri-plugin-updater — all of
  that is the next change (`windows-installer-and-release`).
- Shipping ttsd (Python sidecar) on Windows.
- Running the Rust test suite on Windows CI.
- Code signing (deferred to #183).
