# Delta spec: ipc-commands

## MODIFIED Requirements

### Requirement: Audio Export Commands

The system SHALL expose two Tauri commands for per-entry audio export
(issue #225), following the #224 rfd-backend pattern (no dialog/fs plugin,
no capability changes):

`pick_export_audio_path(entry_id)` SHALL read the entry under the storage
lock, derive the save-dialog default name `ruvox-<entry_id>.<ext>` from the
entry's stored audio format, and open the native save dialog on the blocking
thread (the dialog is modal-blocking and must not run on the tokio reactor),
returning the chosen path as `Option<String>`: `None` when the user cancels.
For an `.opus`-stored entry (the normal case) the dialog SHALL offer two
file filters — `Ogg Opus`/`opus` first and `WAV`/`wav` second; for a
`.wav`-stored entry (the synthesis-transcode fallback) the dialog SHALL
offer only `WAV`/`wav`, as before. A missing entry SHALL fail with
`entry.not_found`; an entry without a stored `audio_path` SHALL fail with
`export.no_audio`.

`export_audio(entry_id, path)` SHALL resolve the entry's stored audio file
under the storage lock (`audio/<audio_path>` inside the data dir) and, on
the blocking thread, produce the file at `path` (issue #252): a `.wav`
target for an `.opus`-stored file SHALL be produced by decoding the Opus
stream to a mono 16-bit PCM WAV at 48 kHz, honoring the stream's pre-skip
and end trim; every other combination SHALL be a byte-for-byte copy. The
cached original MUST NOT be modified in either case. A missing entry SHALL
fail with `entry.not_found`; a missing source file SHALL fail with
`export.no_audio`; a failed conversion SHALL fail with
`export.convert_failed` carrying the underlying error as a message param;
an I/O failure of the copy SHALL fail with `export.copy_failed` carrying the
underlying error as a message param. A panicked blocking task SHALL fail
with `export.dialog_panicked` (pick) or `export.task_panicked` (export).
The commands MUST NOT create history or queue side effects — no
`entry_updated` emission, no status change.

The frontend wrappers SHALL be `commands.pickExportAudioPath(entryId)` and
`commands.exportAudio(entryId, path)`.

#### Scenario: Save dialog offers both formats for an Opus-stored entry

- GIVEN an entry whose stored audio file is `audio/<id>.opus`
- WHEN `pick_export_audio_path` is invoked for the entry
- THEN the save dialog opens pre-filled with `ruvox-<id>.opus` offering an
  `Ogg Opus`/`opus` filter (first/default) and a `WAV`/`wav` filter, and the
  chosen path is returned

#### Scenario: WAV-stored entry keeps the WAV-only dialog

- GIVEN an entry whose stored audio file is `audio/<id>.wav`
- WHEN `pick_export_audio_path` is invoked for the entry
- THEN the save dialog opens pre-filled with `ruvox-<id>.wav` and a single
  `WAV`/`wav` filter

#### Scenario: Cancelled dialog resolves to null

- GIVEN the save dialog is open for an entry
- WHEN the user cancels the dialog
- THEN the command resolves to `null` and no file is written

#### Scenario: Export copies the stored file

- GIVEN an entry with a stored audio file and a chosen target path whose
  extension does not request a conversion (e.g. `.opus` for an `.opus`
  source)
- WHEN `export_audio` is invoked
- THEN the cached file is copied byte-for-byte to the target path, the cache
  file remains in place, and no `entry_updated` is emitted

#### Scenario: Export to a `.wav` target converts the audio

- GIVEN an entry whose stored audio file is `audio/<id>.opus` and a chosen
  target path ending in `.wav`
- WHEN `export_audio` is invoked
- THEN a mono 16-bit PCM WAV at 48 kHz is written to the target path
  (decodable, with pre-skip discarded and end trim applied), and the cached
  `.opus` file remains in place

#### Scenario: Conversion failure fails with `export.convert_failed`

- GIVEN an entry whose stored `.opus` file cannot be decoded (e.g. corrupt
  data) and a chosen target path ending in `.wav`
- WHEN `export_audio` is invoked
- THEN the command rejects with `export.convert_failed` and the localized
  error is shown by the frontend; the target file is not left behind

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
