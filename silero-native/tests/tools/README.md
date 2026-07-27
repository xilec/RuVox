# Fixture generators for the silero-native test suite

Golden/parity fixtures under `silero-native/tests/fixtures/` are the output of
the upstream Python reference code (torch frontend, ttsd chunking, ONNX
pipeline). Regenerate them when the reference changes:

- the upstream v5_ru model or the exporter (`silero-native/export`) changes
  the frontend/accentor/bundle semantics;
- `ttsd/ttsd/chunking.py` changes (chunking fixtures);
- the phrase/case lists in the generators themselves are extended.

## How to run

```bash
nix develop -c silero-native/tests/tools/regenerate_fixtures.sh
```

This runs all four generators inside the `silero-native/export` uv
environment (pinned torch CPU + onnxruntime) and rewrites the fixtures in
place. Review the result with `git diff silero-native/tests/fixtures/`, then
run `nix develop -c cargo test --manifest-path silero-native/Cargo.toml`.

For non-default paths run the generators directly:

```bash
nix develop -c bash -c "cd silero-native/export && \
  uv run python ../tests/tools/gen_parity_fixtures.py --model-path /path/v5_ru.pt --bundle /path/bundle"
```

## Requirements

- **Model**: `v5_ru.pt`. Resolved from the torch hub cache
  (`~/.cache/torch/hub/snakers4_silero-models_master/src/silero/model/v5_ru.pt`)
  or downloaded via upstream `models.yml` — same resolution as the exporter.
  Override with `--model-path`.
- **Bundle** (parity generator only): the exported ONNX bundle, default
  `tmp/bundle-v5`, override with `--bundle`. Export it first via
  `silero-native/export` if absent. The frontend/accentor generators do NOT
  need the bundle — symbols/symbol_to_id are read from the model itself.
- **Environment**: the `silero-native/export` uv project (torch CPU,
  onnxruntime). The chunking generator needs neither torch nor the model
  (`ttsd.chunking` is pure stdlib) and can run under any Python.

## The `unpack_q_model` trap

References MUST be taken from the dequantized model: the quantized
homosolver resolves stress differently, and production `apply_tts`
dequantizes the word embeddings before the first inference (the ONNX export
was traced after dequantization too). All model-based generators load the
package via `export.load_package()` (through `_common.load_pack()`), which
calls `pack.unpack_q_model()` — do not bypass it with a raw
`torch.package.PackageImporter` load.

## Determinism

All generators are deterministic: the frontend/accentor/chunking outputs are
pure text processing, and the ONNX + torch inference in the parity generator
is deterministic on CPU. Regenerating from the same model + bundle +
generator inputs reproduces the fixtures byte-for-byte (verified via sha256
over `tests/fixtures/**` before/after a regeneration).
