## Why

Issue #19 measured the pure-Rust `opus-rs` encoder as ~2× slower than the C
`libopus` FFI path on RuVox's exact profile (48 kHz mono, 20 ms frames,
32 kbps VOIP): 17.9 s vs 8.6 s for a 22:49 narration. Profiling explains the
gap and points at a third option the issue did not consider:

- 56 % of `opus-rs` 0.1.18's time is the scalar
  `silk_noise_shape_quantizer_del_dec`; `libopus` runs an AVX2 version of the
  same quantizer (`silk_NSQ_del_dec_avx2`) — 2.5 s vs 10.1 s absolute.
- Another 16 % is the scalar fixed-point
  `silk_warped_autocorrelation_fix`; `libopus` uses its floating-point FLP
  variant. Those two functions account for the whole regression.
- `opus-rs` 0.1.40 improved the scalar quantizer (13.1 s) but still has no
  SILK SIMD.

`opus1` 0.3.0 (a hard fork of `opus-rs` 0.1.37 with SSE4.1/AVX2 kernels for
exactly those SILK functions) encodes the same sample in **4.94 s — 1.74×
faster than `libopus`** — at the same bitrate (5,593,012 vs 5,592,269 bytes)
and the same measured quality (SNR vs the source: 9.06 dB vs 9.04 dB).

Switching to `opus1` therefore both closes #19's performance question and
achieves what the issue actually wanted: dropping the `libopus` native
dependency (one less native library in the AppImage / Nix closure, a simpler
cross-compile and a fully reproducible pure-Rust audio path).

## What Changes

- Replace the `opus` crate (FFI to C `libopus`) with the pure-Rust `opus1`
  crate for **both** the WAV → Ogg-Opus encoder and the Opus → WAV decoder
  used by audio export.
- Drop `libopus` from `nix/devshell.nix` and the `flake.nix` build inputs and
  runtime library path; drop the `libopus-dev`/`pkg-config` requirement from
  the non-Nix install instructions.
- Keep the stored format and every user-visible behavior identical: Ogg
  container, mono Opus, 32 kbps VOIP, 20 ms frames, pre-skip from the
  encoder's lookahead, same file naming and atomic publish.
- Update the storage spec wording that names `libopus` (the five native rates
  and the pre-skip source) — behavior is unchanged, only the implementation
  reference moves to the new encoder.
- Add a `[Unreleased]` CHANGELOG note: audio saving is ~1.7× faster and the
  app no longer needs the system `libopus`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `storage`: the "Audio File Storage" requirement stops naming `libopus` as
  the source of the native-rate list and the pre-skip; both now come from the
  in-process Rust encoder. No scenario changes.

## Impact

- `src-tauri/Cargo.toml`, `Cargo.lock` — dependency swap (`opus` → `opus1`).
- `src-tauri/src/audio/mod.rs` — encoder/decoder construction, bitrate,
  lookahead, error type; no pipeline changes.
- `nix/devshell.nix`, `flake.nix` — remove `libopus` from build inputs and the
  wrapper's library path.
- `docs/install.md` — drop `libopus-dev` from the non-Nix dependency list.
- `openspec/specs/storage/spec.md` — synced by archiving this change.
- `CHANGELOG.md` — one user-visible entry.

## Non-goals

- No change to the stored audio format, bitrate, frame size or file layout;
  existing `.opus` files stay valid and are not re-encoded.
- No quality tuning or A/B listening campaign: the switch targets speed and
  the dependency, at matched bitrate and measured SNR.
- No use of `opus1`'s optional neural features (DRED, LACE/NoLACE, OSCE-BWE)
  and no decoder-complexity changes.
- No migration of already stored audio and no change to playback (mpv) or to
  the audio-export UI.
