# Tasks: SIGTERM before SIGKILL

## Implementation

- [x] `src-tauri/src/tts/mod.rs` `driver_task` shutdown timeout branch:
  SIGTERM via `libc::kill` → 2 s grace (`timeout` on `child.wait()`) →
  SIGKILL only if still alive; update the surrounding comments.

## Tests

- [x] If practical with the mock fixtures: a mock ttsd that ignores the
  shutdown command but exits on SIGTERM — assert the driver does not need
  SIGKILL; otherwise cover by code inspection + existing shutdown tests
  staying green.

## Validation

- [x] `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml` green.
- [x] `nix develop -c just lint` green.
- [x] openspec validate sigterm-before-sigkill --strict green.
