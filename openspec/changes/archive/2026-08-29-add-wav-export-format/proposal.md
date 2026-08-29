# Proposal: add-wav-export-format

## Why

"Save audio as…" copies the stored audio byte-for-byte, so exports are always
Ogg Opus (`{uuid}.opus`). Users who need the audio in an editor, a player
without Opus support, or an archive workflow have no way out of the cache
format. (Issue #252.)

## What Changes

1. **The export save dialog offers both formats**: `Ogg Opus` (default, first
   filter) and `WAV`. The default filename still follows the stored format;
   for the synthesis-transcode fallback entries (stored `.wav`) the dialog
   stays WAV-only, as today.
2. **`export_audio` converts by the chosen target extension**: a `.wav` target
   for an `.opus`-stored entry decodes the Opus stream to PCM WAV at export
   time (`ruvox-<id>.wav`); every other combination keeps the byte-for-byte
   copy. Only the exported file is converted — the cache keeps the Opus
   original.
3. **Opus → WAV decoder in `src-tauri/src/audio/`**: streaming decode of the
   stored Ogg-Opus to a mono 16-bit PCM WAV at 48 kHz, honoring the stream's
   pre-skip and end trim. Reuses the `opus` + `ogg` crates already in the
   dependency tree — no new dependencies.
4. **New error code `export.convert_failed`** for a failed conversion, with
   localized messages in `ru.ts` / `en.ts`.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `ipc-commands`: the "Audio Export Commands" requirement — the save dialog
  gains a WAV filter, `export_audio` gains the convert-on-`.wav`-target
  behavior, and the `export.convert_failed` error code is added.

## Impact

- `src-tauri/src/audio/mod.rs` — new `decode_opus_to_wav` (+ `AudioError`
  variant for Ogg transport errors) and unit tests (round-trip through the
  existing encoder).
- `src-tauri/src/commands/export.rs` — dialog filters, extension-dispatched
  conversion, tests.
- `src/i18n/{ru,en}.ts` — one new error string each. No component logic
  changes: the format is picked in the native save dialog, the command
  signature is unchanged.
