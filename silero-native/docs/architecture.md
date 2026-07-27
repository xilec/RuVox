# silero-native — architecture notes

> Stub document for the silero-native subproject (native Silero TTS engine
> on ONNX Runtime, no Python). Will grow through Phases 2–5 of
> `docs/plans/silero-native-port.md`.

## Purpose

A third TTS engine («Silero (нативный)») that runs the exported Silero v5
ONNX graphs (`tts_main.onnx`, `istft.onnx`, `pqmf.onnx`, `homosolver.onnx`,
`accentor_tensor.onnx`) in-process via the `ort` crate, replacing the
Python `ttsd` subprocess for this engine. Piper stays the default; `ttsd`
remains as fallback.

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
