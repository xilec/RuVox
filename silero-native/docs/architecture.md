# silero-native — architecture notes

## Purpose

A third TTS engine («Silero (нативный)») that runs the exported Silero v5
ONNX graphs (`tts_main.onnx`, `istft.onnx`, `pqmf_*.onnx`, `homosolver.onnx`,
`accentor_tensor.onnx`) in-process via the `ort` crate, replacing the
Python `ttsd` subprocess for this engine. Piper stays the default; `ttsd`
remains as fallback.

## Synthesis pipeline

```
text (one chunk)
  → frontend/text.rs        prepare_text_input: lowercase, dash/symbol
                            normalization, filter against `symbols`,
                            strip unsupported markup ([[...]], SSML tags)
  → frontend/homosolver.rs  homograph words → [HOMO]…[/HOMO] spans →
                            bert.rs (Basic+WordPiece) → homosolver.onnx →
                            resolved stress variant (homodict.json)
  → frontend/accentor.rs    ngram lookup (ngrams.gz + exceptions.gz) →
                            accentor_tensor.onnx → `+` stress and `ё`
                            (explicit `+` in the input wins)
  → sequence ids            `|` + text + `~` → frontend.json symbol_to_id
  → tts_main.onnx           sequence, speaker_id, durs_rate=1, pitch=1
                            → mag/x/y (1,1201,T) + dur_hat
  → istft.onnx              mag/x/y → audio 48 kHz (n_fft 2400, hop 600)
  → pqmf_24k/8k.onnx        only for 24000/8000 output (48k skips this)
  → 16-bit PCM WAV + char-proportional word timestamps
```

Sessions for the always-needed models (`tts_main`, `istft`, `homosolver`,
`accentor_tensor`) are opened concurrently at `SileroNative::load`
(independent ORT sessions are thread-safe to create in parallel); the
rate-specific `pqmf_24k`/`pqmf_8k` sessions are lazy-opened on the first
synthesis requesting that rate. All sessions are guarded by mutexes
(`Session::run` needs `&mut`); a panic inside inference is contained with
`catch_unwind` and the poisoned mutex is recovered — the engine stays
usable after a failed call.

## Bundle format

`<data_local_dir>/ruvox/voices/silero-native/` (app), or any directory via
`SILERO_NATIVE_BUNDLE` (tests/examples). `manifest.json` records the
upstream model version, per-file sha256 + size, ONNX opset, source `.pt`
hash, export date (UTC). The loader verifies every file before opening
sessions; a mismatch fails with `EngineError::Bundle` naming the file.
The downloader (`src-tauri/src/tts/silero_native/download.rs`) fetches the
manifest first, streams files with on-the-fly hashing, and only renames
`.partial` → final after verification.

## Error codes (app-facing)

`bundle_not_installed`, `bundle_download_failed`, `bundle_manifest_invalid`,
`bundle_checksum_failed`, `bundle_path_unsafe`, `engine_unavailable`,
`silero_native_load_failed`, `bad_input` (empty text / unknown speaker /
unsupported sample rate), `synthesis_failed`.

## Debugging guide

- **Compare against the Python reference.** The exporter self-check
  (`silero-native/export`, `--self-check-only`) regenerates torch
  references; the parity fixtures live in `tests/fixtures/parity/`
  (regenerate with `tmp/gen_parity_fixtures.py`-style scripts — always
  after `unpack_q_model()`, or homograph resolution differs; this is a
  documented trap).
- **Inspect intermediate tensors.** `tts_main.onnx` exposes `dur_hat`
  (per-symbol frame durations, frame = 12.5 ms) as its 4th output;
  `EngineOutput.spoken_text` carries the accented text the frontend
  produced — check it first when pronunciation is wrong: the bug is then
  in the frontend, not the graphs.
- **Frontend bugs** reproduce without GPU/torch: the golden fixtures in
  `tests/fixtures/frontend/` pin `prepare_text_input` symbol-for-symbol.
- **Known upstream quirks** (deliberate, do not "fix"): pitch-zeroing on
  punctuation is not exported (measured e2e effect ≤ 1e-3 — see
  export/README.md); the `^` stress-skip marker is dropped (upstream
  raises KeyError); accentor parity is decision-level (argmax/softmax),
  raw logits may differ by ~1e-2 without audible effect.
- **Known edge cases:** zero predicted durations are clamped inside the
  exported graph (`dur + 0.5` before `repeat_interleave`), matching
  upstream; empty/punctuation-only input fails with `bad_input` before any
  inference.
- **Parity threshold flakiness:** the suite budget is 1e-3 with a worst
  observed case of 9.8e-4. If an onnxruntime version bump pushes a case
  over, raise the threshold to 2e-3 with a comment (and re-listen).
- **Chaotic divergence corners exist in the upstream model itself:** rare
  (text, voice) inputs (confirmed: kseniya + «в тысяча») produce valid but
  entirely different speech in torch vs ONNX, with identical durations and
  inputs. If a user reports "the native engine reads a phrase differently
  than the Python one" for a specific phrase+voice, check this first —
  compare `dur_hat` (identical => chaotic corner, not a port bug).

## ONNX Runtime linkage strategy

**Decision: system onnxruntime from nixpkgs + `ort` feature `load-dynamic`
+ `ORT_DYLIB_PATH`.**

- Crate: `ort = "2.0.0-rc.12"` (pykeio/ort v2; pinned with `=` while on rc),
  feature `load-dynamic`. Same version already sits in
  `src-tauri/Cargo.lock` via piper-rs — one ort version for the whole app.
- At runtime `ort` dlopens the shared library from the `ORT_DYLIB_PATH`
  environment variable:
  - dev shell: `nix/devshell.nix` sets
    `ORT_DYLIB_PATH = "${pkgs.onnxruntime}/lib/libonnxruntime.so"`;
  - production: `flake.nix` `preFixup` bakes the same `--set-default
    ORT_DYLIB_PATH` into the app wrapper.
- At build time `ort-sys` (with `load-dynamic`) does **not** link or
  download anything; it only needs the crate headers. The `onnxruntime`
  nixpkgs package in `buildInputs` is there so the runtime path exists in
  the closure, not for linking.
- Why not the default prebuilt-binary download: in the Nix build sandbox
  there is no network, and in the dev shell a downloaded prebuilt would be
  impure and non-reproducible (and would pull its own glibc/libstdc++
  expectations). The nixpkgs onnxruntime (1.26.0 at the current flake.lock
  pin) is the single source of truth.

Reference: https://ort.pyke.io/setup/linking

## How to re-verify (blocker check, task 1.1)

The throwaway spike lives in `tmp/ort-spike/` (not committed; `tmp/` is
gitignored). To reproduce:

```bash
nix develop -c bash -c "cd tmp/ort-spike && cargo run"
```

Expected: the session loads `tmp/onnx-spike/istft.onnx`, prints its
inputs (`mag`, `x`, `y` — float32 `[1, 1201, T]`, `T` dynamic) and runs a
dummy inference producing `audio` of shape `[1, 6000]`.

Model IO was inspected with Python `onnx` in an ad-hoc uv venv inside
`tmp/ort-spike/.venv` (never in the project's own environments):

```bash
nix develop -c bash -c "cd tmp/ort-spike && uv venv .venv && \
  uv pip install --python .venv/bin/python onnx && \
  .venv/bin/python -c 'import onnx; ...'"
```

## Production build check (task 1.2/1.3)

`onnxruntime` is in both `nix/devshell.nix` `buildInputs` and the flake
package `buildInputs` (slim and full alike — onnxruntime is allowed in the
slim build; only `ttsd`/Python is gated out by the CI slim/full check).
Verification:

```bash
nix eval .#ruvox.name          # flake eval passes
nix build .#ruvox -L           # full production build
```

Status of the first full build run: see `tmp/ort-spike/nix-build.log`
(recorded during the Phase 1 blocker check).

## Testing

Tests split into two tiers:

- **Unit tests** (`src/**`, `tests/frontend_text.rs`) need no model and run
  everywhere, including CI.
- **Bundle-gated tests** (`tests/bundle_load.rs`, `tests/edge_cases.rs`,
  `tests/frontend_accentor.rs`, `tests/parity.rs`) need the ~230 MB model
  bundle. They look up `SILERO_NATIVE_BUNDLE` first, then the default
  `tmp/bundle-v5`, and skip silently when no `manifest.json` is found. They
  run locally and on any CI machine with the bundle cached; there is no
  synthetic ONNX fixture — every plumbing path is covered either by unit
  tests or by the bundle-gated suite.

To simulate the no-bundle CI path locally:

```bash
SILERO_NATIVE_BUNDLE=/nonexistent nix develop -c \
  cargo test --manifest-path silero-native/Cargo.toml
```
