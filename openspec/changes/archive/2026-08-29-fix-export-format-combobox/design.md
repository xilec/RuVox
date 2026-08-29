# Design: fix-export-format-combobox

## Context

The export dialog runs through rfd, whose Linux backend is the
xdg-desktop-portal. The portal response carries `uris` and the selected
values of custom `choices`, but rfd flattens it to `Option<PathBuf>` and
the portal has no "chosen file-type filter" field at all — so no dialog
flow built on rfd can ever learn which filter the user switched to (this
killed both earlier iterations: filter switching was cosmetic, and the
extensionless default always resolved to the stored format).

## Goals / Non-Goals

**Goals:**
- One menu item; the format is chosen inside the dialog.
- WAV as the default (users export for editing; Opus stays available).
- The exported file name always matches the exported bytes.

**Non-Goals:**
- A custom GTK dialog (main-thread/GTK integration with tauri's loop is
  risky, and it would not help Windows).
- Reading the chosen filter on platforms that don't report it.

## Decisions

- **Direct ashpd call on Linux.** ashpd 0.11 is already in the dependency
  tree (rfd's portal backend uses it); `SaveFileRequest` supports
  `current_name` and a `choices` combo, and the response's `choices()`
  reports the selection. The «Формат» combo (`wav` default, `opus`
  alternative) is rendered by the portal backend itself. The command awaits
  the portal future directly — it is IPC plus user think time, not CPU
  work, so no `spawn_blocking`.
- **ashpd is declared with `default-features = false`.** ashpd 0.11 fails
  to compile with both the `async-std` and `tokio` features enabled; the
  runtime feature must come solely from rfd's declaration (`async-std`, an
  rfd default). Awaiting its futures from tokio works via the global
  async-io driver.
- **Normalization follows the reported choice; extension follows the
  report.** With a reported format, the path's extension is forced to it
  (matching kept as typed) — the combo is an explicit declaration, so a
  typed `.opus` under a WAV choice becomes `.wav`. Without a report (older
  portal backends, non-Linux), the recognized `opus`/`wav` extension in the
  name decides; anything else falls back to the stored format's extension.
- **No file-type filters in the portal dialog.** A filter combo next to the
  format combo would invite the same "switched the filter" confusion; the
  combo is the single control.
- **Windows unchanged at the dialog level.** Win32 rewrites the typed
  extension to the selected filter's extension on save, so the recognized-
  extension fallback already tracks the user's choice there; rfd stays for
  the dialog, `export_audio`'s extension dispatch stays the executor.

## Risks / Trade-offs

- [Older portal backends may drop the choice combo] → `choices()` comes
  back empty; the stored format's extension is used as the fallback and a
  debug line records it. The dialog still saves.
- [The combo is portal chrome, not our widget] → Its label («Формат») and
  values are ours; placement is the backend's, consistent with other
  portal apps (Firefox-style format choices).

## Migration Plan

None: command signature unchanged (`pick_export_audio_path(entry_id)`), the
frontend keeps a single call.

## Open Questions

None.
