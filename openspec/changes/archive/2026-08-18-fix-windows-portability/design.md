# Design: fix-windows-portability

## Context

See proposal.md — Why. Static analysis plus eight `cargo xwin check`
iterations (`tmp/win-check/blockers.md`) identified two compile blockers
and four runtime adaptations, all in `src-tauri`. The follow-up installer
change will populate the bundled resources this design resolves paths to;
this change only makes the code compile on
`x86_64-pc-windows-msvc` and behave correctly whether or not those
resources are present.

## Goals / Non-Goals

**Goals:**

- `cargo check --target x86_64-pc-windows-msvc` (on a windows-latest
  runner, real MSVC) compiles `src-tauri` cleanly.
- Windows runtime degrades gracefully: no ttsd, no `/proc` reaping, mpv
  and espeak-ng-data resolved from bundled resources when present.
- Zero behavior change on Linux — pinned by the existing test suite.

**Non-Goals:**

- Bundling resources, installer, release CI (next change).
- A Windows equivalent of orphan-mpv reaping (named-pipe based). Not
  needed for correctness: worst case is a leftover mpv process after an
  app crash, which exits with its parent on Windows job-object semantics
  or lingers harmlessly until reboot.
- Running `cargo test` on Windows.

## Decisions

### D1: `cfg(unix)` gates over abstraction layers

`reap_orphan_mpv` (`src-tauri/src/lib.rs`) and the `libc::kill` in the
ttsd shutdown path (`src-tauri/src/tts/mod.rs`) get `#[cfg(unix)]` (call
sites compiled out on Windows). No cross-platform abstraction is
introduced: the Windows counterpart (named-pipe orphan reaping) is
explicitly a non-goal, so a trait/seam would be dead weight.

*Alternative considered:* replace `libc::kill` with
`tokio::process::Child::kill` everywhere. Rejected — the ttsd path kills
by PID after losing the handle (supervisor restart race), and the mpv
reaper walks foreign processes by design; neither maps to `Child::kill`.

### D2: mpv resolution — one function, platform-first

A single `resolve_mpv_path()` helper in `player/mod.rs`: on Windows,
check `<resource_dir>/mpv/mpv.exe` (via `AppHandle::path().resource_dir()`),
fall back to plain `"mpv"` (PATH); on Linux return `"mpv"`. The helper
takes the resource dir as a parameter so unit tests can point it at a
tempdir — no `AppHandle` in tests.

*Alternative considered:* Tauri `externalBin` sidecar. Rejected — a
sidecar ships exactly one renamed binary; mpv needs its DLLs co-located,
so a `resources/` folder is the natural fit (also keeps the LICENSE file
next to the binaries).

### D3: espeak-ng-data env var at the earliest point in `main`

`std::env::set_var` is `unsafe` in edition 2024 because it races with
threads reading the environment. Setting
`PIPER_ESPEAKNG_DATA_DIRECTORY` at the top of `main` (before Tokio
runtime and Tauri builder start any threads) is safe and documented as
such with a comment. Windows-only, wrapped in `#[cfg(windows)]`; only set
when the bundled directory exists, otherwise Piper's built-in search
applies (degraded Russian stress, but functional).

*Alternative considered:* set it in the Tauri `setup` hook. Rejected —
setup runs after the async runtime and plugins exist; the race window is
real.

### D4: ttsd probe — spawn failure is "unavailable"

`tts/availability.rs` probes `uv --version`. `Command::output()` already
returns `Err` when the binary is missing; the probe maps any non-success
(non-zero exit **or** spawn error) to `available: false` with the
existing Russian reason string. Covered by a unit test that probes a
guaranteed-nonexistent binary name (same pattern as the existing
`/nonexistent/tts/binary` supervisor tests).

### D5: Unix-only test helpers behind `cfg(unix)`

Tests spawning `cat`/`tail` (`tts/supervisor.rs`) get
`#[cfg(unix)]` so target-scoped `cargo check --all-targets` stays clean
on Windows. Test-only `/tmp` paths are inside those same tests.

## Risks / Trade-offs

- [Bundled resources absent in dev on Windows → mpv/Piper degraded] →
  Acceptable: PATH fallback for mpv, Piper falls back to its built-in
  espeak data search; the installer change makes resources present in
  shipped builds.
- [`cfg(unix)` drift: future Linux-only code added ungated] → CI check
  job on windows-latest (part of the installer change) fails the build.
- [Unsafe `set_var` misused later] → isolated in one `cfg(windows)`
  startup fn with a comment explaining the before-threads invariant.

## Migration Plan

Pure code change; existing Linux installs unaffected. No data migration.

## Open Questions

None blocking. The exact bundled-resource layout (`mpv/` subtree,
`espeak-ng-data/` path inside resources) is finalized in the installer
change; this change only depends on the two directory names above.
