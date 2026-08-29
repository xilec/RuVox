# Proposal: add-audio-export

## Why

There is no audio export: the only way to reach a synthesized file is
"Open cache folder" in Settings, which exposes the raw cache layout
(`audio/<uuid>.opus|.wav` names) and requires manual copying. The user
needs a per-entry "Save audio as…" action that writes the entry's audio to
a user-chosen path. (Issue #225; acceptance: the user can export an entry's
audio to disk.)

## What Changes

1. **Two backend commands** (the #224 rfd-backend pattern — no dialog/fs
   plugins, no capability changes):
   - `pick_export_audio_path(entry_id)` — native save dialog (rfd,
     `spawn_blocking`), pre-filled name `ruvox-<entry_id>.<ext>` and a file
     filter matching the entry's stored audio format (`.opus` normal case,
     `.wav` fallback); returns the chosen path or `null` on cancel.
   - `export_audio(entry_id, path)` — copies the entry's stored audio file
     from the storage audio dir to the target path. Errors:
     `entry.not_found`, `export.no_audio` (no audio file on disk),
     `export.copy_failed`.
2. **Queue context menu item** «Сохранить аудио как…» (after «Воспроизвести»),
   enabled for `ready`/`playing` entries — the same gate as Play. On
   success a confirmation notification shows the target path; a cancelled
   dialog is a silent no-op; failures surface the localized red error.
3. **README** «Как управлять»/export mention is out of scope — the feature is
   self-evident from the menu; the CHANGELOG entry is proposed separately at
   review time.

Export is a byte-for-byte copy of the stored file — the entry already holds
Ogg-Opus (transcoded at synthesis) or WAV (transcode fallback), so the
issue's "Opus/WAV" is covered by exporting what is on disk. No transcode,
no decoder dependency.

## Impact

- **Affected specs:** `ipc-commands` (new "Audio Export Commands"
  requirement), `ui` ("Queue list behavior" — new menu item + flow).
- **Affected code:**
  - `src-tauri/src/commands/mod.rs` (or a sibling module): the two commands
    + registration; a dialog-free `export_audio_to` core for unit tests.
  - `src/lib/tauri.ts` — `pickExportAudioPath`, `exportAudio` wrappers.
  - `src/components/QueueList.tsx` — menu item + handler; `src/i18n/{ru,en}.ts`
    — menu/notification/error strings.
- **Out of scope:** Opus→WAV transcoding on export, batch export, export of
  timestamps.
