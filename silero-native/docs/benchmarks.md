# Benchmarks

Date: 2026-08-01 (headline numbers re-measured after the issue #164
optimization; previous 2026-07-27 baseline kept in the per-stage section
below). CPU: AMD Ryzen 9 7900 (12 cores / 24 threads).
Toolchain: Rust engine via `ort` 2.0.0-rc.12 + nixpkgs onnxruntime
1.26.0 (`load-dynamic`); Python reference = torch CPU `apply_tts` from
`silero-native/export/.venv` on the same `v5_ru.pt` (after
`unpack_q_model`).

## Methodology

Phrase: `Сервер обрабатывает запросы и сохраняет данные в базу.` (54
chars), speaker `aidar`, 24000 Hz.

- **Native**: `cargo run --release --example bench` — 1 warmup + 20 timed
  runs of `SileroNative::synthesize` per sample rate (full pipeline:
  frontend → tts_main → istft → pqmf → WAV encode), engine load excluded.
  The headline comparison number is the 24000 Hz run.
- **Python**: `tmp/bench_python.py` — 1 warmup + 5 timed runs of
  `pack.apply_tts` on the same phrase/rate.

## Results

| Engine | mean | p95 | min | max |
|---|---|---|---|---|
| silero-native (release) | 36.6 ms | 42.6 ms | 33.7 ms | 43.6 ms |
| Python torch `apply_tts` | 145.0 ms | — | 142.5 ms | 151.3 ms |

**Speedup: ~4.0x** (warm, full pipeline on both sides). Pre-#164 it was
~1.3x (108.8 ms native) — the win is the pinned ORT intra-op thread
count, see the per-stage section below.

## Per-stage breakdown (issue #164)

Date: 2026-08-01, same machine. Bench example now reports per-stage means
(`StageTimings` in the engine, summed over chunks) at each sample rate,
same phrase/speaker, 1 warmup + 20 timed runs per rate. Baseline below is
**before any optimization**, same commit that introduced the breakdown.

| stage | mean ms (24k) | % |
|---|---|---|
| frontend_text | 0.01 | 0.0% |
| homosolver | 0.00 | 0.0% |
| accentor | 0.29 | 0.3% |
| build_sequence | 0.01 | 0.0% |
| tts_main | 85.4 | 80.3% |
| istft | 16.5 | 15.5% |
| pqmf | 3.5 | 3.3% |
| wav_encode | 0.67 | 0.6% |
| concat_timestamps | 0.03 | 0.0% |

Totals per rate: 24k mean 106.4 ms / p95 122.3; 48k mean 88.8 ms / p95
98.7; 8k mean 103.6 ms / p95 123.3. Engine load ~437 ms (unchanged).

Findings:

- **tts_main dominates (~80%)**, istft is ~16%, everything else is noise.
  The text frontend (homosolver BERT + ngram accentor) is < 0.5% —
  frontend optimization is off the table.
- **48k PQMF path cost: zero** (48k bypasses PQMF; `pqmf` stage reads
  0.00). PQMF downsample to 24k costs ~3.5 ms, to 8k ~2.2 ms.
- tts_main/istft run the *same* graphs at every rate, yet read slower at
  the first-measured rate (24k: 85.4 vs 48k: 71.4) — CPU frequency ramp
  over the run order. Treat cross-rate deltas of rate-independent stages
  as noise; the headline methodology number stays 24k, same as before.

### Optimization: ORT intra-op threads = 8

The profile said the win must come from ORT itself. A/B of
`intra_op_num_threads` (bench mean at 24k, full pipeline):

| intra-op threads | mean ms | worst parity-suite case |
|---|---|---|
| ORT default (per logical core) | 104.1 | 9.8e-4 (pre-existing) |
| 4 | 40.0 | **1.5e-3 — over budget** |
| 6 | 34.0 | **2.2e-3 — over budget** |
| **8 (chosen)** | **34.6** | **9.8e-4 — inside budget** |
| 12 | 55.3 | — |
| 16 | 80.0 | — |
| 24 | 115.1 | — |

ORT defaults to one thread per logical core; these graphs are chains of
many small ops, so per-op fork/join sync across 24 threads costs more
than the compute. Parallel execution mode made it *worse* (42.6 ms at
intra=6), inter-op threads made no positive difference.

The thread count also constrains correctness: changing it changes float
reduction order, which drifts the waveform off the Python-ONNX parity
fixtures (generated at ORT defaults). The `stress_marker` case is the
canary — 1.5e-3 at 4 threads, 2.2e-3 at 6, both over the suite's 1e-3
budget; **8 is the only reduced count that keeps all 31 cases inside the
budget** (worst 9.8e-4, same class as the pre-existing default-thread
baseline) and ties 6 for speed within noise. So
`bundle.rs::open_session` pins `with_intra_threads(8)` for every session.

Post-optimization breakdown (24k): tts_main 25.0 ms (68.4%), istft
9.2 ms (25.0%), pqmf 1.1 ms, wav_encode 0.8 ms, frontend < 1%.
Totals: **24k mean 36.6 ms / p95 42.6**; 48k mean 30.2 / p95 31.0; 8k
mean 33.2 / p95 33.7. Side effect: engine load 437 → ~345 ms (fewer
pool threads to spawn at session creation).

Measured and rejected after the optimization:

- **Allocation/copy churn** (`take_f32` `.to_vec()`, chunk concat): all
  copies sit inside the ORT-stage timings and total well under a
  millisecond (~1 MB of tensor buffers); `concat_timestamps` reads
  0.05 ms, `(unaccounted)` 0.01 ms. Nothing to win.
- **Bulk WAV encode** (hound per-sample `write_sample`): 0.8 ms at 24k
  (2.2%). Not worth churning a parity-sensitive path (upstream
  truncates, we round) for a sub-millisecond effect.

The remaining ~34 ms is ORT inference inside the exported graphs
themselves (tts_main ~25 ms, istft ~9 ms) — irreducible from the Rust
client side without touching the graphs/exporter, which is out of scope
here (parity fixtures would have to be regenerated).

## Engine load time (issue #165)

Date: 2026-08-01, same machine. "Load" = `SileroNative::load` (manifest
verify + ONNX session creation + frontend), measured by the bench
example's `load_ms` (and the `RUST_LOG=silero_native=info` per-phase
timings). ttsd baseline = spawn `uv run python -m ttsd` → `warmup` →
ok-response → `shutdown`, 5 runs (`tmp/bench_ttsd_spawn.py`).

| Path | mean | min | max |
|---|---|---|---|
| ttsd spawn-to-ready (Python sidecar) | 1680.8 ms | 1654.1 ms | 1708.4 ms |
| silero-native load — baseline (sequential sessions) | 651.6 ms | — | — |
| silero-native load — optimized | ~440 ms | 416.7 ms | 464.7 ms |

Baseline breakdown (warm page cache): manifest verify 115 ms, six
sequential session opens 507 ms (tts_main 223, homosolver 214, istft 53,
accentor 13, pqmf ~3), frontend 30 ms.

Optimizations applied:

- **Concurrent session creation** (`Sessions::open`): the sessions are
  independent, so they open on scoped threads. 507 ms → ~340 ms.
- **Concurrent sha256 verify** (`Manifest::verify`): hashing is pure CPU
  once the page cache is warm. 115 ms → ~60 ms.
- **Lazy PQMF sessions**: `pqmf_24k`/`pqmf_8k` open on the first synthesis
  at that rate instead of at load (~3 ms saved at load; the real win is
  not paying for a rate that is never requested).

Measured and rejected:

- **ORT graph optimization level**: Level3 ≈ Level1 ≈ Disable (~310 ms
  sessions either way) — session creation is dominated by model parse /
  arena init, not the optimizers. This also rules out the **`.ort`
  compiled-model cache** (it only skips optimization).
- **Warm the engine at app start**: already implemented in the app layer
  (`spawn_initial_warmup` via the engine switcher, `src-tauri/src/lib.rs`)
  — the selected engine warms in the background at startup and on every
  engine switch.

The remaining floor is ORT session init for `tts_main` (~220 ms class)
and `homosolver` (~210 ms class), which overlap under concurrent open but
do not vanish. **Native load is ~3.8x faster than ttsd spawn-to-ready**
on the same machine, so the issue's acceptance target (native ≤ ttsd) is
met with margin.

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
