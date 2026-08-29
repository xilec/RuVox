# Delta spec: ipc-commands

## MODIFIED Requirements

### Requirement: Audio Export Commands

The system SHALL expose two Tauri commands for per-entry audio export
(issue #225), following the #224 rfd-backend pattern (no dialog/fs plugin,
no capability changes):

`pick_export_audio_path(entry_id)` SHALL open the xdg-desktop-portal save
dialog (Linux) pre-filled with the extensionless name `ruvox-<entry_id>` (the portal
does not sync the name with the combo, and a stale pre-filled extension
would trip the overwrite confirmation) and a
«Формат» choice combo — `WAV` selected by default, `Ogg Opus` as the
alternative — and SHALL NOT gate it on file-type filters (the combo, not a
filter switch, decides the format). The portal response SHALL report the
combo's selected value, and the returned path SHALL be normalized to that
format: a matching extension (case-insensitive) SHALL be kept as typed, a
mismatched or foreign extension SHALL be replaced, and a missing one
appended, so the file name always matches the exported bytes. If the
response carries no usable choice, the stored format's extension SHALL be
used as the fallback. Cancelling the dialog SHALL return `None`. A missing entry SHALL fail with
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



#### Scenario: Returned path is normalized to the stored format

- GIVEN an `.opus`-stored entry and a dialog result of `/tmp/audio.mp3` (or
  an extensionless `/tmp/audio`)
- WHEN the command normalizes the returned path
- THEN the path is `/tmp/audio.opus` — a foreign extension is replaced and a
  missing one appended with the stored format's extension, while a
  recognized `opus`/`wav` extension (any case) is kept as typed

#### Scenario: Export dialog carries a format choice with WAV default

- GIVEN an entry with stored audio
- WHEN `pick_export_audio_path` is invoked
- THEN the dialog opens pre-filled with the extensionless name `ruvox-<id>`
  and a «Формат» combo reporting `WAV` by default with `Ogg Opus` as the
  alternative

#### Scenario: The chosen format decides the export

- GIVEN a dialog result of `/tmp/audio.wav` while the «Формат» combo
  reports `Ogg Opus`
- WHEN the command normalizes the returned path
- THEN the path is `/tmp/audio.opus` — the chosen format's extension is
  enforced (matching extensions in any case are kept as typed), and the
  subsequent `export_audio` copy/convert decision follows it

#### Scenario: A response without a usable choice falls back to the stored format

- GIVEN a portal response that carries no «Формат» value
- WHEN `pick_export_audio_path` is invoked for an `.opus`-stored entry
- THEN the returned path is normalized to the stored format's extension

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
