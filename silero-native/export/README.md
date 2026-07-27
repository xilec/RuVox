# Silero ONNX bundle exporter

Production exporter that converts a Silero TTS v5-style `torch.package`
(`.pt`) into a self-contained ONNX bundle consumed by the native Rust engine
(`silero-native`). Replaces the spike scripts in `tmp/onnx-spike/` (those were
proof-of-concept; this is the clean, reproducible implementation).

## Requirements

- `uv` (inside the repo dev shell: run everything via `nix develop -c ...`)
- Python >= 3.12 (managed by uv)
- Dependencies are pinned in `pyproject.toml` (torch CPU via the
  `https://download.pytorch.org/whl/cpu` index, onnx, onnxruntime, scipy,
  pyyaml). The JIT-graph surgery in `export.py` depends on torch/onnx
  internals — do not bump pins without re-validating the parity self-check.

## Usage

```bash
cd silero-native/export
nix develop -c uv sync
nix develop -c uv run python export.py --model v5_ru --out ../../tmp/bundle-v5
```

CLI:

- `--model <id>` — model id from Silero's `models.yml` (default `v5_ru`).
- `--model-path <file>` — use a specific `.pt` file, skipping cache/URL
  resolution.
- `--out <dir>` — output bundle directory (required).
- `--speaker <name>` — speaker for export examples and self-check
  (default `aidar`).
- `--self-check` / `--no-self-check` — torch-vs-ONNX end-to-end waveform
  parity check (default: on; exits non-zero when max abs diff > 1e-3 or the
  waveform lengths differ). The check runs in a **fresh subprocess**
  automatically: the ONNX export passes mutate the loaded torch model's
  graph state, so in-process torch inference drifts after the export and
  cannot serve as a reference.
- `--self-check-only` — only (re)run the self-check against an existing
  bundle in `--out`, without re-exporting.

Each exported graph is additionally validated inline right after its export
(ORT parity assert: `tts_main` dur_hat exact + mel head < 5e-2, `istft` <
1e-4, `pqmf_*` < 1e-5, `homosolver` < 1e-4, `accentor_tensor` decision-level:
argmax equal + softmax diff < 1e-4 — raw logits carry amplified float noise,
see the note in `export.py`).

Model resolution order:

1. `--model-path` if given.
2. Local torch hub cache:
   `$TORCH_HOME/hub/snakers4_silero-models_master/src/silero/model/<model>.pt`
   (i.e. `~/.cache/torch/hub/...` by default — present if you ever loaded the
   model via `torch.hub`).
3. Download: the URL is looked up in
   [models.yml](https://raw.githubusercontent.com/snakers4/silero-models/master/models.yml)
   (`tts_models.<lang>.<model>.latest.package`) and fetched into
   `~/.cache/silero-onnx-export/`.

## Bundle contents

| File | Contents | IO contract |
|---|---|---|
| `tts_main.onnx` | FastPitch (dur/pitch predictors + tacotron encoder + HourGlass decoder) + vocoder head. 5 speakers in one graph, opset 17 | in: `sequence` (1,L) i64, `speaker_ids` (1,) i64, `durs_rate` (1,L) f32, `pitch_coefs` (1,L) f32 → out: `mag`,`x`,`y` (1,1201,T) f32, `dur_hat` (1,L) f32 |
| `istft.onnx` | inverse STFT (n_fft 2400, hop 600, win 2400, hann) as DFT-matmul + static-pad OLA | in: `mag`,`x`,`y` (1,1201,T) f32 → out: `audio` (1,600·T) f32, 48 kHz |
| `pqmf_24k.onnx` | PQMF analysis filterbank (N=2, taps 62, cutoff 0.25, beta 10), band 0 only | in: `audio` (1,1,T) f32 48 kHz → out: `band0` (1,1,⌈T/2⌉) f32, 24 kHz |
| `pqmf_8k.onnx` | PQMF analysis filterbank (N=6, taps 62, cutoff 0.12, beta 9), band 0 only | in: `audio` (1,1,T) f32 48 kHz → out: `band0` (1,1,⌈T/6⌉) f32, 8 kHz |
| `homosolver.onnx` | homograph disambiguation BERT (word embeddings dequantized from int8) | in: `input_ids` (B,L) i64, `homo_start_ids` (B,) i64, `homo_end_ids` (B,) i64 → out: `logits` (B,1) f32; prediction = `round(sigmoid(logit))` indexes the sorted variant list in `homodict.json` |
| `accentor_tensor.onnx` | stress/ё classifiers over a fastText-style ngram embedding-bag (mean) | in: `ind` (N,) i64 ngram ids, `offsets` (W,) i64 → out: `stress_logits` (W,10), `yo_logits` (W,7) f32 |
| `ngrams.gz` | accentor ngram dictionary: space-separated grams, **id = position in file** | consumer builds `gram -> id` map |
| `exceptions.gz` | accentor exceptions: `word stress_vowel_idx yo_char_idx` per line (utf-8, gzipped) | `yo_char_idx == -1` means "no ё" |
| `homodict.json` | homograph → accented variants (`{"тому": ["т+ому", "том+у"]}`) | variant chosen by homosolver prediction |
| `vocab.txt` | BERT WordPiece vocab, **id = line number** (added tokens `[HOMO]`/`[/HOMO]` included) | tokenizer contract in `frontend.json` |
| `frontend.json` | `symbols`, `symbol_to_id`, `alphabet`, speakers + ids, frame window (0.0125 s), accentor/homosolver constants | everything the Rust text frontend needs |
| `manifest.json` | model id + source `.pt` sha256, per-file sha256/size, opset, tool versions, export date (UTC) | integrity check for the engine |
| `selfcheck/` | `report.json` + torch/ONNX wav pairs for the fixed phrase set | parity evidence, regenerated on each export |

PQMF filters are computed once at export time (`scipy.firwin`, same code as
`tts_package/package_utils.py`) and baked into the ONNX graphs as conv1d
weights — no scipy needed at runtime. The recomputed filters are asserted
against the package's own buffers (< 1e-6) during export.

Audio path (all sample rates): `tts_main → istft` (48 kHz); for 24 kHz / 8 kHz
append `pqmf_24k` / `pqmf_8k`. Frame-level word timestamps can be recovered
from `dur_hat` (cumsum × 0.0125 s).

## Re-exporting for a new Silero version

1. `nix develop -c uv run python export.py --model <new_id> --out <dir>`
   (e.g. `v5_5_ru`). The exporter resolves the URL from `models.yml` itself.
2. Check the self-check output — all phrases must be `OK`. If a new model
   changes graph structure (it happened between v4 → v5), the JIT surgery in
   `export.py` may need adjustments; the failure mode is an ONNX export error
   or a self-check `FAIL`.
3. Publish: run the GitHub workflow **Export Silero Bundle**
   (`.github/workflows/export-silero-bundle.yml`, manual `workflow_dispatch`)
   with the model id. It exports, runs the self-check and uploads
   `silero-onnx-bundle-<model>.tar.gz` + `manifest` + `selfcheck` report to a
   release tagged `silero-models-<model>`. Release upload uses
   `softprops/action-gh-release` (re-runnable via `overwrite: true`).

**Note:** the workflow has not been executed on GitHub yet (written and
reviewed locally only) — first run may need small fixes.

## License

The Silero models are published by the Silero Team under
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/).
The ONNX conversion is a technical format conversion of the model
(§2(a)(4) of the license); the converted weights remain under the same
license — **non-commercial use only**, attribution to the Silero Team.
