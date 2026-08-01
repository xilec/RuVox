//! Benchmark: N syntheses of a fixed ~50-char phrase at 24000 Hz,
//! reporting engine load time (`load_ms`) plus mean / p95 wall time per
//! synthesis. Results are recorded in `docs/benchmarks.md` next to the
//! Python `apply_tts` comparison (see tmp/bench_python.py) and the
//! engine-load / ttsd spawn-to-ready comparison (tmp/bench_ttsd_spawn.py).
//!
//! `RUST_LOG=silero_native=info` surfaces per-phase load timings
//! (manifest verify, per-session open, frontend).
//!
//! Usage: cargo run --release --example bench -- [bundle_dir]
//! (default: ../tmp/bundle-v5 relative to the crate, or SILERO_NATIVE_BUNDLE).

use std::time::Instant;

use silero_native::SileroNative;

mod common;

/// ~50 chars, mirrors the Python bench phrase in tmp/bench_python.py.
const PHRASE: &str = "Сервер обрабатывает запросы и сохраняет данные в базу.";
const RUNS: usize = 20;

fn main() {
    // `RUST_LOG=silero_native=info` surfaces the per-session load timings.
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let dir = common::bundle_dir_arg();

    let t = Instant::now();
    let engine = SileroNative::load(&dir).expect("bundle must load");
    let load = t.elapsed();

    // Warmup (JIT-less, but first-run ORT optimizations/arena allocs).
    engine
        .synthesize(PHRASE, "aidar", 24000)
        .expect("warmup synthesis must succeed");

    let mut times = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let t = Instant::now();
        engine
            .synthesize(PHRASE, "aidar", 24000)
            .expect("synthesis must succeed");
        times.push(t.elapsed());
    }
    times.sort();

    let mean = times.iter().sum::<std::time::Duration>() / RUNS as u32;
    // Nearest-rank p95.
    let p95 = times[((RUNS as f64) * 0.95).ceil() as usize - 1];
    let min = times[0];
    let max = times[RUNS - 1];
    println!("phrase: {PHRASE:?} ({} chars)", PHRASE.chars().count());
    println!("runs: {RUNS} @ 24000 Hz, speaker aidar");
    println!("mean {mean:.1?} | p95 {p95:.1?} | min {min:.1?} | max {max:.1?}");
    println!(
        "load_ms={:.1} mean_ms={:.1} p95_ms={:.1} min_ms={:.1} max_ms={:.1}",
        load.as_secs_f64() * 1e3,
        mean.as_secs_f64() * 1e3,
        p95.as_secs_f64() * 1e3,
        min.as_secs_f64() * 1e3,
        max.as_secs_f64() * 1e3,
    );
}
