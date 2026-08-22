# Design: graceful-storage-startup-error

## Context

The setup hook runs after the log plugin is registered, so `tracing` entries already
land in the per-user log file when storage initialization fails. There is no webview
window yet at that point, and the tray is not yet initialized — the only reliable
user-visible surface is a native OS dialog. The release profile sets
`panic = "abort"`, so any `.expect()`/`.unwrap()` on this path kills the process
without unwinding: no message box, no flush of anything the user could read.

## Goals / Non-Goals

- Goal: a startup storage failure produces (1) a log entry and (2) a user-visible
  native dialog with actionable Russian text, then a clean non-zero exit.
- Non-goals: see proposal.

## Decisions

1. **Match-and-exit instead of `expect()`:** a dedicated helper in `lib.rs` matches
   on `StorageService::new()`'s result; on `Err(e)` it logs
   `tracing::error!("failed to open storage: {e}")`, shows the dialog, and calls
   `std::process::exit(1)`. Alternative rejected: returning an error from `setup` —
   Tauri aborts the run loop but prints to stderr only (invisible for a GUI launch),
   and still reads as a crash.
2. **Native dialog via `rfd`, not `tauri-plugin-dialog`:** the plugin brings JS
   bindings and plugin registration we do not need before the webview exists; `rfd`
   is already the dialog backend that plugin uses on desktop, so adding it directly
   keeps the dependency tree flat (`rfd::MessageDialog::new().set_level(Error)
   ... .show()` blocks until dismissed). Alternative rejected: printing to stderr
   only — invisible for desktop launches.
3. **Message text in Russian** (user-facing string rule): states what failed, that
   data was not lost, and where logs live. The detailed error `Display` goes into
   the log entry; the dialog carries a short generic cause hint plus the log
   directory path resolved from `crate::paths`.
4. **Exit code 1:** distinguishes "clean refusal" from a successful run; the process
   exits before any window exists, so there is nothing to tear down.

## Risks / Trade-offs

- [`rfd` pulls GTK dialogs on Linux] → it links the same GTK stack WebKitGTK already
  requires; no new system libraries.
- [Dialog shown before log flush] → tracing appender writes synchronously per event;
  the error line is durable before `show()` returns control.

## Migration Plan

No data or config migration. Behavior changes only on the failure path.

## Open Questions

None.
