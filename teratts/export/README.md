# TeraTTSv2 bundle exporter

Assembles the RuVox model bundle from the upstream
[`TeraSpace/TeraTTSv2`](https://huggingface.co/TeraSpace/TeraTTSv2) release and
writes the `manifest.json` the Rust engine and the downloader verify.

## What goes into the bundle

- `models/text_encoder.onnx`, `models/duration_predictor.onnx`,
  `models/sampler_distilled_cfg3_8step.onnx`, `models/vocoder.onnx`
- `unicode_indexer.json`
- `styles/<voice>/{style_ttl,style_dp}.npy` for all ten voices
- `ruaccent/dictionary/{accents,omographs,yo_words,yo_homographs}.json.gz`
  (dictionary mode only)
- `LICENSE`, `LICENSE-OPENRAIL`, `RUACCENT_NOTICE.txt`

Not shipped: `sampler_teacher_8step.onnx` (2–3× slower, no quality gain in the
research) and `ruaccent/nn/**` (483 MB of neural RUAccent assets the dictionary
mode does not use). Total is ~410 MiB.

`manifest.json` records `format_version`, `bundle_version`, `model_id`,
`model_version`, `upstream_repo`, `upstream_revision`, `opset`, `sample_rate`,
`created_at_utc`, `voices[]` and `files[]` with `path`, `asset`, `size` and
`sha256` per file. `asset` is the flat GitHub release name (`/` is not allowed
in release assets); `path` is the bundle-relative location.

## Requirements

`uv` and Python 3.12 (both in the repo dev shell). Dependencies are pinned in
`pyproject.toml`; `uv.lock` records the exact resolution.

## Usage

```bash
cd teratts/export
nix develop -c uv sync
nix develop -c uv run python export.py \
  --snapshot /path/to/hf/snapshot \
  --out ../../tmp/teratts-bundle
```

or let the exporter fetch the pinned revision (only the needed files):

```bash
nix develop -c uv run python export.py --download --out ../../tmp/teratts-bundle
```

The exporter validates every graph's opset and input/output names, the 65 536
entry indexer, and the `(1, 50, 256)` / `(1, 8, 16)` style shapes before it
copies anything. It refuses a non-empty output directory unless `--force` is
given.

## Publishing a bundle version

1. Check upstream: compare the latest revision of `TeraSpace/TeraTTSv2` with
   `UPSTREAM_REVISION` in `export.py`. If it moved, read the model card for
   changes, bump `UPSTREAM_REVISION` and `DEFAULT_MODEL_VERSION`, re-run the
   exporter, and listen to the new voices before publishing.
2. Assemble with a new `--bundle-version` (monotonic) into a fresh directory.
3. Publish a new tag (`teratts-models-v2`, …) with `--publish-tag`; the release
   notes template is `RELEASE_NOTES.md`.
4. The application pins the release tag in
   `src-tauri/src/tts/teratts/download.rs`; update it when the bundle is meant
   to ship. Version discovery, in-app update and deletion are issue #317.

## Parity fixtures

`parity_fixtures.py` runs the pinned upstream `teratts.py` against the bundle
and writes the arrays the Rust parity test compares (`teratts/tests/fixtures/`).
It needs the upstream snapshot directory in addition to the bundle:

```bash
nix develop -c uv run python parity_fixtures.py \
  --snapshot /path/to/hf/snapshot \
  --bundle ../../tmp/teratts-bundle \
  --out ../tests/fixtures
```

## License

The bundle redistributes TeraTTSv2 weights under the TeraTTSv2 Model License
(Open RAIL-M with a commercial attribution condition) and the BigScience Open
RAIL-M License, plus RUAccent assets under MIT. Keep the three license files in
the bundle and the attribution in the product's public description; see
`teratts/NOTICE`.
