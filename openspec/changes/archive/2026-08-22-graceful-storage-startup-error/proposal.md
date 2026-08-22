# Proposal: graceful-storage-startup-error

## Why

`StorageService::new().expect("failed to open storage")` in the Tauri setup hook
(`src-tauri/src/lib.rs`) hard-crashes the app when the per-user storage cannot be
opened (unresolvable home dirs, permissions, disk errors). Combined with the release
profile's `panic = "abort"` the user gets a silent process death — no message, no log
line, nothing actionable — which reads as "the app is broken" and produces
undiagnosticsable bug reports (#223).

## What Changes

- Replace the startup `expect()` with graceful failure handling: on storage-open
  error the app logs a structured error entry (existing tracing/log plugin), shows a
  native error dialog with a Russian user-facing message (via the `rfd` crate, the
  same dialog backend Tauri's own dialog plugin uses on desktop), and exits cleanly
  with a non-zero exit code.
- The app never panics/aborts on this path; no debug assertion, no raw panic text.
- New direct dependency: `rfd` (desktop-only blocking message dialog; no JS/plugin
  registration needed).

## Capabilities

- **Modified Capabilities:**
  - `storage` — new ADDED requirement covering startup-failure behavior: user-visible
    error dialog, log entry, clean non-zero exit instead of a panic abort.
- **New Capabilities:** none.

## Impact

- `src-tauri/src/lib.rs` — setup hook storage initialization replaced with
  match-and-exit helper.
- `src-tauri/Cargo.toml` — add `rfd`.
- Specs: `openspec/specs/storage/spec.md` gains an ADDED requirement.
- No changes to StorageService itself; no persisted-format or IPC changes.

## Non-goals

- Recovery UI inside the webview (the webview may not even be constructible if the
  data dir is unusable).
- Automatic retry / alternate-directory fallback logic.
- Handling of failures other than the storage-open path (player, TTS engines keep
  their existing error flows).
