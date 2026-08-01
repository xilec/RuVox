# Tasks: Harden cancel_entry against non-processing entries

## Implementation

- [x] `src-tauri/src/commands/mod.rs` `cancel_entry` — return
  `CommandError::SynthesisError` ("entry {id} cannot be cancelled
  (status: …)") for `ready` / `playing` / `error`, before touching the
  registries or storage; `pending` stays allowed (idempotent cancel, #129
  semantics; a just-added entry sits in `pending` with its task already
  registered); keep the `processing` path unchanged.

## Tests

- [x] Unit test per terminal status (`ready`, `error`; `playing` is
  unreachable through storage): fails with `SynthesisError`, stored status
  unchanged, registries untouched.
- [x] Unit test: `pending` entry with a registered task — cancel succeeds
  and the task is aborted.
- [x] Existing `processing` tests still pass unchanged.

## Validation

- [x] `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml` green.
- [x] `nix develop -c just lint` green.
- [x] `openspec validate harden-cancel-entry-status-guard --strict` green.
