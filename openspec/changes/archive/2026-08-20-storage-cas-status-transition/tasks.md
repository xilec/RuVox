# Tasks: Storage-level compare-and-set for status transitions

## Implementation

- [ ] `src-tauri/src/storage/service.rs` — add `StorageService::update_entry_if(id, predicate, mutate) -> bool`: acquire the write lock, evaluate `predicate(&entry)`, and — only if `true` — apply `mutate(&mut entry)` then persist, all under the same lock; drop the guard before calling `save_history` (non-reentrant `RwLock`), return `false` for absent/rejected.

- [ ] `src-tauri/src/commands/mod.rs` `cancel_entry` — drive the `processing | pending → pending` transition through `update_entry_if`; keep the up-front `not_found` / terminal-status (`ready` / `playing` / `error`) rejections and the `synthesis_tasks` / `synthesize_entered` registry handling on the path that actually applies.

- [ ] `src-tauri/src/commands/mod.rs` `apply_ready_if_current` — pass `completion_is_current` as the predicate and the ready-field mutation as the closure; discard the late audio/timestamp files only when the CAS did not apply.

- [ ] `src-tauri/src/commands/mod.rs` `apply_error_if_current` — pass `require_processing ⇒ completion_is_current` as the predicate and the error-field mutation as the closure; discard candidate files only when the CAS did not apply on the `require_processing` path.

## Tests

- [ ] `src-tauri/src/storage/service.rs` — unit test for `update_entry_if`: predicate-accepted applies + persists + returns `true`; predicate-rejected is a no-op + returns `false`; absent id returns `false`.

- [ ] `src-tauri/src/commands/mod.rs` — existing `cancel_entry`, `apply_ready_if_current`, `apply_error_if_current` tests stay green (no observable behavior change on the non-racy path).

## Validation

- [ ] `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml` green.
- [ ] `nix develop -c just lint` green (fmt + clippy -D warnings).
- [ ] `nix develop -c pnpm dlx @fission-ai/openspec validate storage-cas-status-transition --strict` green.
