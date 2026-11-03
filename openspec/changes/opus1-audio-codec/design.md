## Context

`src-tauri/src/audio/mod.rs` encodes every synthesized narration to Ogg-Opus
through the `opus` crate, an FFI binding to the system `libopus` 1.6.1. The
same crate decodes stored `.opus` files for audio export. `libopus` is
therefore a build input (`nix/devshell.nix`, `flake.nix`) and a runtime
library in the packaged app.

Issue #19 asked whether the pure-Rust `opus-rs` could replace it and found it
~2× slower. This design re-measured the problem on the exact production
profile and evaluated a newer pure-Rust alternative before deciding.

### Measurements

Sample: 22:49 (1369 s) Russian narration, 48 kHz mono f32 WAV (250 MiB),
decoded from a RuVox-produced `.opus`. Profile: 20 ms frames, 32 kbps VOIP,
VBR, complexity 9, pre-skip 312, the same Ogg muxer for every Rust variant.
AMD Ryzen 9 7900, `opt-level=3` + `lto = "thin"`, three interleaved runs.

| Variant | Wall (s) | Output (bytes) | SNR vs source |
|---|---:|---:|---:|
| `libopus` 1.6.1 FFI (`opus` 0.3) | 8.58–8.62 | 5,592,269 | 9.04 dB |
| `opus-rs` 0.1.18 | 17.86–17.99 | 5,553,348 | — |
| `opus-rs` 0.1.40 | 13.15–13.27 | 6,930,485 | — |
| `opus1` 0.3.0 | 4.93–4.95 | 5,593,012 | 9.06 dB |

`opus1` is 1.74× faster than `libopus`, matches its bitrate (0.01 % larger
file) and its measured distortion. `opus-rs` 0.1.40's larger file is a VBR
budgeting difference, not a quality gain.

### Where the time goes (`perf`, user-space callchains)

| `libopus` | % | `opus-rs` 0.1.18 | % |
|---|---:|---|---:|
| `silk_NSQ_del_dec_avx2` | 28.9 | `silk_noise_shape_quantizer_del_dec` (scalar) | 56.4 |
| `silk_warped_autocorrelation_FLP` | 13.1 | `silk_warped_autocorrelation_fix` (scalar) | 16.1 |
| `silk_NLSF_del_dec_quant` | 6.6 | `silk_nlsf_del_dec_quant` | 2.5 |
| `tonality_analysis` | 3.8 | `CeltEncoder::encode_impl` | 2.0 |

Absolute time in the delayed-decision quantizer: 2.5 s (`libopus`) vs 10.1 s
(`opus-rs`) — 4.1×. Warped autocorrelation: 1.1 s vs 2.9 s. Together they
account for essentially the entire 9.3 s gap. `opus-rs` ports the fixed-point
SILK path with scalar kernels and no SILK SIMD; `libopus` uses hand-written
AVX2 for the quantizer and the floating-point FLP analysis. `opus-rs` 0.1.40
speeds up the scalar quantizer (5.3 s absolute) but still ships no SILK SIMD.
`opus1` is a fork of `opus-rs` 0.1.37 that adds SSE4.1/AVX2 kernels for
exactly these two functions.

## Goals / Non-Goals

**Goals:**

- Keep the stored format and every user-visible behavior identical while
  removing the `libopus` native dependency from the app and its build.
- Do not regress encode speed; ideally gain it on the production profile.
- Keep the encoder and decoder behind one module boundary so a future codec
  swap stays a local change.

**Non-Goals:**

- No format/bitrate/frame-size change, no re-encoding of existing audio.
- No new decoder features (DRED, LACE/NoLACE, OSCE-BWE) and no playback
  changes.
- No quality tuning beyond matching the current bitrate and measured SNR.

## Decisions

### Use `opus1` 0.3.0 for both encode and decode

`opus1` is the only evaluated pure-Rust option that is faster than the C
reference on this profile, and it covers the decoder path too. Pinning
`opus1 = "0.3"` in `Cargo.toml` with the workspace `Cargo.lock` gives a
reproducible dependency; the crate is `no_std`-capable with no build script
and no C toolchain, which is exactly what makes the Nix/AppImage closure
smaller.

### Keep every format constant in `audio/mod.rs` unchanged

Frame size, bitrate, `OpusHead` fields, granule handling and the atomic
`.part` publish are independent of the codec crate. Only construction, the
encode/decode calls and the error type change. Pre-skip still comes from
`OpusEncoder::lookahead()` (6.5 ms at the input rate, 312 samples at 48 kHz),
scaled to 48 kHz ticks by the existing code.

### Drop `libopus` from the devshell and the flake

The app no longer links it. `mpv`/`ffmpeg` keep their own copies for
playback, so nothing else in the closure changes. The non-Nix install
instructions lose `libopus-dev`; `pkg-config` stays because other native
dependencies need it.

### Rejected alternatives

- **Keep `libopus` (status quo).** Cheapest, but leaves the native dependency
  and the slower encoder; the issue's "why we care" stays unresolved.
- **`opus-rs` 0.1.18 / 0.1.40.** 2.0× / 1.5× slower than `libopus`; the
  SILK hotspots are scalar in both.
- **Patch `opus-rs` upstream with AVX2 SILK kernels.** Feasible (the C AVX2
  source is the reference) but a large intrinsics port with a long review
  cycle; `opus1` already ships those kernels, so patching would duplicate
  work and still leave the encoder slower than `opus1`.
- **Switch only the encoder, keep `opus` for decode.** Halves the win and
  keeps `libopus` linked, so the dependency goal fails.
- **Feature-gate both codecs.** Doubles the code path and the test matrix for
  no user-visible benefit; a revert is a single dependency change anyway.

## Risks / Trade-offs

- **Young dependency.** `opus1` is a 2026-10 hard fork of `opus-rs` 0.1.37,
  first published as 0.3.0; a single maintainer. Mitigation: the crate is
  pure Rust (forkable/vendorable in a day), pinned by the lockfile, and the
  swap is one dependency plus one module. If it stalls, reverting is the same
  edit in reverse.
- **Not bit-exact with `libopus` on x86-64.** The fork documents correct but
  non-identical x86 output (bit-exact on aarch64). Encoder quality was checked
  by SNR on a real 22:49 sample (9.06 dB vs 9.04 dB) at the same size. The
  decoder is likewise not bit-identical there: against a clean 4 s source at
  32 kbps its decode measures 4.54 dB SNR versus libopus 5.28 dB, inside a
  codec noise floor of ~5 dB — the exported WAV stays a valid decode of the
  same stream.
- **Older CPUs without AVX2/SSE4.1** take `opus1`'s scalar kernels, which may
  be slower than `libopus`'s C fallback. Desktops targeted by RuVox are
  AVX2-capable; this is documented in the design history rather than gated.
- **Decoder regression risk** on export. Mitigation: the existing
  encode→decode round-trip tests exercise the new decoder, plus a manual
  export check on a real stored entry.
