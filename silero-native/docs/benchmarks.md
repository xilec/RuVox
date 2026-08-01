# Benchmarks

Date: 2026-07-27. CPU: AMD Ryzen 9 7900 (12 cores / 24 threads).
Toolchain: Rust engine via `ort` 2.0.0-rc.12 + nixpkgs onnxruntime
1.26.0 (`load-dynamic`); Python reference = torch CPU `apply_tts` from
`silero-native/export/.venv` on the same `v5_ru.pt` (after
`unpack_q_model`).

## Methodology

Phrase: `Сервер обрабатывает запросы и сохраняет данные в базу.` (54
chars), speaker `aidar`, 24000 Hz.

- **Native**: `cargo run --release --example bench` — 1 warmup + 20 timed
  runs of `SileroNative::synthesize` (full pipeline: frontend → tts_main →
  istft → pqmf_24k → WAV encode), engine load excluded.
- **Python**: `tmp/bench_python.py` — 1 warmup + 5 timed runs of
  `pack.apply_tts` on the same phrase/rate.

## Results

| Engine | mean | p95 | min | max |
|---|---|---|---|---|
| silero-native (release) | 108.8 ms | 125.5 ms | 71.9 ms | 128.4 ms |
| Python torch `apply_tts` | 144.4 ms | — | 142.5 ms | 146.0 ms |

**Speedup: ~1.3x** (warm, full pipeline on both sides).

## Note on the spike's "~10x"

`tmp/onnx-spike/REPORT.md` quoted ~10x (torch `apply_tts` ~0.5 s vs ORT
main+istft ~46 ms). That number does not reproduce as an apples-to-apples
comparison:

- the ~0.5 s was a **cold, first-call** `apply_tts` (JIT warmup included);
  a warmed torch `apply_tts` runs the same phrase in ~145 ms;
- the ~46 ms covered only `tts_main` + `istft` — no text frontend
  (homosolver BERT + ngram accentor), no PQMF downsample, no WAV encoding,
  all of which the native bench includes.

The honest warm full-pipeline comparison is **~1.3x** for the native
engine. The larger real-world win vs the current production path comes
from eliminating the Python `ttsd` subprocess (startup, IPC, env), not
from per-synthesis wall time.
