# teratts

In-process TeraTTSv2 engine: a 99M-parameter Russian/English flow-matching
ONNX model running on the ONNX Runtime the app already vendors, with no Python
or PyTorch at synthesis time.

The crate owns the model bundle format and verification, the four ONNX
sessions, the Russian text frontend (language tags, RUAccent dictionary
stress, codepoint gate), sentence chunking under the model's 25-second call
limit, approximate word timestamps, and the 44.1 kHz WAV writer. The
application adapter (`src-tauri/src/tts/teratts/`) maps it to `TtsEngine`.

Check it with:

```bash
cargo test --manifest-path teratts/Cargo.toml
```

Bundle-gated tests need a bundle directory and are skipped otherwise:

```bash
TERATTS_BUNDLE=/path/to/bundle cargo test --manifest-path teratts/Cargo.toml
```

Behavior is specified in `openspec/specs/teratts-engine/`; the bundle exporter
and the update procedure live in `export/`.

## Bundle versioning and updates

The engine pins one bundle release tag (`BUNDLE_RELEASE_BASE_URL` in
`src-tauri/src/tts/teratts/download.rs`) and verifies every file against the
release's `manifest.json` before loading. A newer bundle is a new tag
(`teratts-models-v2`, …) assembled with a higher `bundle_version`; the
procedure is in `export/README.md`, "Publishing a bundle version". The app
only adopts a new tag when that pin is updated and shipped, so an upstream
model update can never change user-visible behavior silently.

Checking for updates, downloading a new bundle in-app, and deleting bundles
are tracked in issue #317. The bundle format itself is versioned
(`format_version` in the manifest) so a future incompatible layout is rejected
with a clear error instead of a partial load.

## Performance

Measured on the reference development machine (4 threads, `load-dynamic` ONNX
Runtime 1.26):

| Metric | Value |
|--------|-------|
| Model load (cold) | ~5.1 s |
| Synthesis RTF, 4 threads | 0.063 (17 s of audio) |
| Synthesis RTF, 12 threads | 0.049–0.071 |
| Peak RSS (4 threads) | ~1.3 GiB (weights + activations + RUAccent dictionaries) |

The RTF leaves ample headroom under the plan's ≤ 0.15 target; a long article
is synthesized faster than real time. Memory is the known cost of the
dictionary-mode accentizer and is the reason the bundle is optional.

## Parity with the Python reference

`export/parity_fixtures.py` runs the pinned upstream `teratts.py` on a fixed
text and writes `tests/fixtures/` (text embedding, raw duration, sampled
latent, the first 8 192 audio samples). The bundle-gated test
`synth::parity::matches_python_fixtures` replays the same arrays through the
Rust sessions and compares them; it skips when `TERATTS_BUNDLE` is unset.
Regenerate the fixtures only when the upstream revision or the bundle changes.

```bash
cd teratts/export
uv run python parity_fixtures.py \
  --snapshot /path/to/hf/snapshot \
  --bundle ../../tmp/teratts-bundle \
  --out ../tests/fixtures
TERATTS_BUNDLE=../../tmp/teratts-bundle cargo test --manifest-path ../../teratts/Cargo.toml parity
```

## Licensing

See `NOTICE`. The bundle is redistributed under the TeraTTSv2 Model License
(Open RAIL-M with a commercial attribution condition) and the BigScience Open
RAIL-M License; RUAccent assets are MIT. Public products using it must credit
"Text-to-speech powered by TeraTTSv2 by TeraSpace".
