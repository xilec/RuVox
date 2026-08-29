# IPC Commands — Audio Export (delta)

## ADDED Requirements

### Requirement: Audio Export Commands

The system SHALL expose two Tauri commands for per-entry audio export
(issue #225), following the #224 rfd-backend pattern (no dialog/fs plugin,
no capability changes):

`pick_export_audio_path(entry_id)` SHALL read the entry under the storage
lock, derive the save-dialog default name `ruvox-<entry_id>.<ext>` and a
file filter from the entry's stored audio format — `Ogg Opus`/`opus` for an
`.opus` file, `WAV`/`wav` for a `.wav` file — open the native save dialog on
the blocking thread (the dialog is modal-blocking and must not run on the
tokio reactor), and return the chosen path as `Option<String>`: `None` when
the user cancels. A missing entry SHALL fail with `entry.not_found`; an
entry without a stored `audio_path` SHALL fail with `export.no_audio`.

`export_audio(entry_id, path)` SHALL resolve the entry's stored audio file
under the storage lock (`audio/<audio_path>` inside the data dir), copy it
byte-for-byte to `path` on the blocking thread, and return `Unit`. The copy
MUST NOT modify the cached original. A missing entry SHALL fail with
`entry.not_found`; a missing source file SHALL fail with `export.no_audio`;
an I/O failure of the copy SHALL fail with `export.copy_failed` carrying the
underlying error as a message param. The commands MUST NOT create history
or queue side effects — no `entry_updated` emission, no status change.

The frontend wrappers SHALL be `commands.pickExportAudioPath(entryId)` and
`commands.exportAudio(entryId, path)`.

#### Scenario: Save dialog is filtered by the stored format

- GIVEN an entry whose stored audio file is `audio/<id>.opus`
- WHEN `pick_export_audio_path` is invoked for the entry
- THEN the save dialog opens pre-filled with `ruvox-<id>.opus` and an
  `Ogg Opus`/`opus` filter, and the chosen path is returned

#### Scenario: Cancelled dialog resolves to null

- GIVEN the save dialog is open for an entry
- WHEN the user cancels the dialog
- THEN the command resolves to `null` and no file is written

#### Scenario: Export copies the stored file

- GIVEN an entry with a stored audio file and a chosen target path
- WHEN `export_audio` is invoked
- THEN the cached file is copied byte-for-byte to the target path, the cache
  file remains in place, and no `entry_updated` is emitted

#### Scenario: Export without audio fails

- GIVEN an entry whose `audio_path` is `None` or whose cached audio file has
  been evicted
- WHEN `export_audio` is invoked
- THEN the command rejects with `export.no_audio` and no file is written

#### Scenario: Export to an unwritable target fails

- GIVEN a chosen target path whose copy fails at the OS level (e.g. a
  read-only directory)
- WHEN `export_audio` is invoked
- THEN the command rejects with `export.copy_failed` and the localized error
  is shown by the frontend
