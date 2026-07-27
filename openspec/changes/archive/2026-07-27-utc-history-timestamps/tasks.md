# Tasks: UTC history timestamps

## Implementation

- [x] `src-tauri/src/storage/service.rs:222` — `Local::now().naive_local()` →
  `Utc::now().naive_utc()`.
- [x] `src-tauri/src/commands/mod.rs` (`audio_generated_at`) — same change.

## Validation

- [x] `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml` green.
- [x] `nix develop -c just lint` green.
- [x] openspec validate utc-history-timestamps --strict green.
