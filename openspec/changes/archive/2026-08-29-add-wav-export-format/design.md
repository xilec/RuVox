# Design: add-wav-export-format

## Context

Export today is a byte copy (`export.rs`), and the save dialog is filtered to
the stored format. The stored audio is Ogg-Opus (32 kbps VOIP, mono, encoded
from a mono WAV at one of the Opus-native rates; `OpusHead` records the encode
rate). The dependency tree already contains everything needed for a decoder:
`opus = "0.3"` (FFI libopus, `Decoder::decode_float`) and `ogg = "0.9"`
(`PacketReader`).

## Goals / Non-Goals

**Goals:**
- A user-chosen `.wav` export target produces a decodable PCM WAV without
  touching the cached Opus original.
- No new dependencies; decode stays streaming (constant memory).
- The exported WAV is honest about its content: standard 48 kHz decode, mono,
  16-bit PCM.

**Non-Goals:**
- `.wav` → `.opus` export (reverse conversion) — the issue only asks for
  Opus → WAV; `.wav`-stored (fallback) entries keep today's WAV-only dialog.
- Resampling back to the pre-encode rate recorded in `OpusHead` — 48 kHz is
  the Opus-native decode rate and what standard tools (e.g. `opusdec`) emit;
  a separate resampler would add code for no audible gain.
- Multiple logical Ogg streams, embedded metadata, stereo streams — the cache
  only ever stores our own mono single-stream files.

## Decisions

- **Reuse `opus` + `ogg`, no new deps.** A decode is the exact inverse of the
  existing encoder (`src-tauri/src/audio/mod.rs`) and lives next to it. The
  alternatives (`symphonia` with ogg/opus features, `ffmpeg`/`opusdec`
  subprocess) add a dependency or an external-process failure mode for no
  quality difference.
- **Decode at 48 kHz, write 16-bit int PCM.** Opus decodes natively at
  48 kHz; 16-bit PCM WAV is the universally editable format users pick WAV
  for. f32→i16 conversion clamps to [-1, 1] and rounds (`* 32767`), matching
  the silero-native convention. 48 kHz mono 16-bit is ~96 KB/s — fine for a
  user-initiated export.
- **Honor pre-skip and end trim (RFC 7845 §4.9-4.10).** `OpusHead.pre_skip`
  samples are discarded from the decoded start; output is capped at
  `last_page_granule − pre_skip` so encoder padding never leaks into the file.
  Without this, every export carries ~13 ms of leading offset and a partial
  trailing frame of silence — a correctness bug for any downstream
  sample-accurate use.
- **Dispatch on the target extension, not a new parameter.** `export_audio`
  keeps its `(id, path)` signature: `.wav` target + `.opus` source → decode;
  every other combination → byte copy. The frontend (native dialog filters)
  already lets the user express the choice through the file extension, so no
  command or wrapper change is needed.
- **Dialog filters, not the stored format, drive the choice.** For an
  `.opus`-stored entry the dialog shows both filters (`Ogg Opus` first/default,
  then `WAV`); the default name stays `ruvox-<id>.opus`. `.wav`-stored entries
  keep the current WAV-only dialog.

## Risks / Trade-offs

- [User picks a `.wav` name via manual typing while the Opus filter is
  selected] → Dispatch is by extension, so the file still converts correctly;
  no impossible state.
- [Malformed/foreign `.opus` in the cache (e.g. hand-copied file)] → Decode
  errors map to `export.convert_failed` with the reason in the message param;
  the cache file is never touched. Our own encoder output is covered by
  round-trip tests.
- [48 kHz is larger than the original 24 kHz content needs] → Accepted:
  matching the standard decode rate keeps the file compatible everywhere; the
  stored Opus already lost the sub-24 kHz bandwidth distinction.

## Migration Plan

None: additive behavior, no persisted format change, no API change.

## Open Questions

None.
