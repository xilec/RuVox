# Design: add-audio-export

## Context

Audio is stored under `<data_dir>/audio/` as Ogg-Opus (transcoded from the
raw WAV at synthesis, `replace_wav_with_opus`) with a `.wav` fallback when
the transcode fails; `TextEntry.audio_path` holds the file name. The repo
already has an rfd-backend dialog precedent — `pick_import_file` (#224) —
which deliberately avoids the dialog plugin and its capability surface. The
queue context menu (QueueList.tsx) is the per-entry action surface; its
Play item already gates on `ready`/`playing`.

## Goals / Non-goals

- Goal: two clicks from a queue item to the audio file on disk, with the
  native save dialog and a clear confirmation.
- Non-goals: Opus→WAV transcoding on export (needs an Ogg-Opus decoder the
  repo does not have), batch export, timestamps export, new plugins.

## Decisions

### D1: rfd backend, no dialog/fs plugins

The save dialog runs backend-side on `tokio::task::spawn_blocking` via
`rfd::FileDialog::save_file` — the exact pattern of `pick_import_file`
(#224). This avoids `tauri-plugin-dialog` + `tauri-plugin-fs`, their JS
counterparts, and capability scoping for arbitrary user-chosen paths; the
copy happens inside the Rust process, which needs no fs scope at all.

### D2: two commands, dialog separated from the copy

`pick_export_audio_path(entry_id)` (dialog) and
`export_audio(entry_id, path)` (copy) mirror the import split
(`pick_import_file` / `read_text_file`). The split keeps the copy logic
unit-testable without a real dialog: the export core is a dialog-free
`export_audio_to(storage, entry_id, &path)` helper, with the `#[tauri::command]`
wrappers around it. The pick command needs `entry_id` (not just a filter)
because the backend derives both the default file name and the filter from
the stored audio format.

### D3: export as-is, no transcode

The stored file is copied byte-for-byte — `.opus` (Ogg-Opus, the normal
case) or `.wav` (transcode fallback). This covers the issue's "(Opus/WAV)"
without an Ogg-Opus decoder (only an encoder exists in `src/audio`). The
save-dialog filter and default name (`ruvox-<entry_id>.<ext>`) follow the
stored extension. `export_audio` never mutates the cache and emits no
`entry_updated`.

### D4: error mapping

Reuses the `CommandError` taxonomy: `entry.not_found` (existing),
`export.no_audio` (no `audio_path` on the entry, or the file is gone — e.g.
cache eviction), `export.copy_failed` (I/O error, message param carries the
OS error). Codes localize under `errors.export.*` in both catalogs;
frontend failure handling is the standard red notification pattern
(`errors.title` + `formatError`).

### D5: UI wiring

The menu item «Сохранить аудио как…» sits between «Воспроизвести» and
«Перегенерировать аудио», gated like Play (`ready`/`playing`). Handler:
pick → `null` = silent no-op → export → success notification «Аудио
сохранено: {0}» (path as the param) / red error on rejection. No modal
confirmation needed — the action creates a file and overwrites nothing the
app owns; the OS save dialog itself asks about overwrites.

## Risks / Trade-offs

- **Two IPC round-trips per export** instead of one combined command:
  consistent with the import split and keeps rfd dialog usage out of the
  testable core; the cost is negligible for a user-initiated action.
- **rfd save dialog on the blocking pool**: same as the import picker —
  modal-blocking, off the tokio reactor, no timeout needed (user-driven).
- **Export of an evicted file fails late** (dialog first, copy second):
  accepted; the error names the cause and the cache is the source of truth.
- **`ruvox-<uuid>.opus` default name is not human-friendly**: acceptable
  MVP; the user renames in the dialog. A text-derived slug risks path
 -illegal characters and encoding issues for no functional gain.

## Migration Plan

None — additive commands and a menu item.

## Open Questions

None.
