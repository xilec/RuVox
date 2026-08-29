# Tasks: add-wav-export-format

## 1. Backend decoder

- [x] 1.1 `src-tauri/src/audio/mod.rs`: add `decode_opus_to_wav(opus_path,
      wav_path)` — stream-decode a stored Ogg-Opus file to a mono 16-bit PCM
      WAV at 48 kHz via the `ogg` PacketReader + `opus::Decoder`
      (`decode_float`), discarding `OpusHead.pre_skip` samples and capping
      output at the final page granule minus pre-skip (end trim). New
      `AudioError::Ogg(String)` variant for transport errors
      (`OggReadError::ReadError` maps to `AudioError::Io`).

## 2. Backend export dispatch

- [x] 2.1 `src-tauri/src/commands/export.rs`: `save_dialog_defaults` returns
      both filters for an `.opus` source (`Ogg Opus` first, `WAV` second) and
      the single WAV filter for a `.wav` source; `pick_export_audio_path`
      registers both filters.
- [x] 2.2 `export_audio_to`: a `.wav` target for an `.opus` source runs
      `decode_opus_to_wav` (errors map to the new
      `export.convert_failed` code); every other combination keeps the
      byte-for-byte copy.

## 3. Backend tests

- [x] 3.1 Decoder unit tests: round-trip through `encode_wav_to_opus` (mono
      sine → Opus → WAV) asserting format (mono / 48000 Hz / 16-bit int),
      audible duration ≈ pre-encode duration (pre-skip + end trim honored:
      length within ±1 frame of the source), and a non-silent signal.
- [x] 3.2 Export tests: `.opus` entry exported to a `.wav` path produces the
      decoded WAV and leaves the cache intact; `.opus` → `.opus` stays a
      byte-for-byte copy; a corrupt `.opus` exported to `.wav` fails with
      `export.convert_failed`; dialog-filter derivation covers both source
      formats.

## 4. Frontend

- [x] 4.1 `src/i18n/ru.ts` / `en.ts`: add `errors.export.convert_failed`
      («Не удалось преобразовать аудио в WAV: {0}» / "Failed to convert the
      audio to WAV: {0}"). No component changes — the command signature and
      QueueList flow are unchanged.

## 5. Gates & docs

- [x] 5.1 `just test` and `just lint` green; `openspec validate --specs
      --strict` green.
- [x] 5.2 `CHANGELOG.md`: 1-2-line `[Unreleased]` note (export can save WAV).
- [x] 5.3 Manual pass (checklist to the user): export a silero-native entry
      via the dialog choosing the WAV filter (real .opus → .wav file plays,
      cache keeps .opus), and re-check the plain .opus export.
