# Tasks: graceful-storage-startup-error

## 1. Implementation

- [x] 1.1 Add `rfd` to `src-tauri/Cargo.toml`
- [x] 1.2 Replace `StorageService::new().expect(...)` in `lib.rs` setup hook with a match: on `Err(e)` — `tracing::error!` with cause, native `rfd` error dialog (Russian text, log dir path), `std::process::exit(1)`
- [x] 1.3 Verify no other startup-path `.expect()` on storage open remains

## 2. Gates

- [x] 2.1 `cargo test --manifest-path src-tauri/Cargo.toml`, `just lint` green
- [x] 2.2 Manual pass (dev build, leftmost monitor, sandboxed XDG env): point data root at an impossible path (e.g. path under a file) → dialog appears in Russian, log contains the error entry, process exits non-zero after dismissal; normal start unaffected

## 3. Wrap-up

- [ ] 3.1 Validate specs, archive change (sync delta), pre-PR reviewer pass
- [ ] 4. PR, merge
