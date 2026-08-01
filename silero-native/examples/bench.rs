//! Benchmark: N syntheses of a fixed ~50-char phrase at each supported
//! sample rate, reporting engine load time (`load_ms`), mean / p95 wall
//! time per synthesis and a per-stage time breakdown (issue #164).
//! Results are recorded in `docs/benchmarks.md` next to the
//! Python `apply_tts` comparison (see tmp/bench_python.py) and the
//! engine-load / ttsd spawn-to-ready comparison (tmp/bench_ttsd_spawn.py).
//!
//! `RUST_LOG=silero_native=info` surfaces per-phase load timings
//! (manifest verify, per-session open, frontend).
//!
//! Usage: cargo run --release --example bench -- [bundle_dir]
//! (default: ../tmp/bundle-v5 relative to the crate, or SILERO_NATIVE_BUNDLE).

use std::time::{Duration, Instant};

use silero_native::{SileroNative, StageTimings};

mod common;

/// ~50 chars, mirrors the Python bench phrase in tmp/bench_python.py.
const PHRASE: &str = "Сервер обрабатывает запросы и сохраняет данные в базу.";
const RUNS: usize = 20;
const RATES: [u32; 3] = [24000, 48000, 8000];

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

/// Print the per-stage mean breakdown: each stage's share of the mean total
/// wall time. Stages sum to less than the total — the remainder is per-call
/// overhead outside the instrumented stages (tensor construction, strip /
/// chunking, validation).
fn print_stage_breakdown(totals: &StageTimings, mean_total_ms: f64) {
    let stages: [(&str, Duration); 9] = [
        ("frontend_text", totals.frontend_text),
        ("homosolver", totals.homosolver),
        ("accentor", totals.accentor),
        ("build_sequence", totals.build_sequence),
        ("tts_main", totals.tts_main),
        ("istft", totals.istft),
        ("pqmf", totals.pqmf),
        ("wav_encode", totals.wav_encode),
        ("concat_timestamps", totals.concat_timestamps),
    ];
    println!("  {:<18} {:>10} {:>6}", "stage", "mean_ms", "%");
    let mut accounted = 0.0;
    for (name, total) in stages {
        let mean_ms = ms(total) / RUNS as f64;
        accounted += mean_ms;
        println!(
            "  {name:<18} {mean_ms:>10.2} {:>5.1}%",
            mean_ms / mean_total_ms * 100.0
        );
    }
    println!(
        "  {:<18} {:>10.2} {:>5.1}%",
        "(unaccounted)",
        mean_total_ms - accounted,
        (mean_total_ms - accounted) / mean_total_ms * 100.0
    );
}

fn run_rate(engine: &SileroNative, rate: u32) {
    // Warmup (JIT-less, but first-run ORT optimizations/arena allocs, and
    // the lazy PQMF session open for this rate).
    engine
        .synthesize(PHRASE, "aidar", rate)
        .expect("warmup synthesis must succeed");

    let mut times = Vec::with_capacity(RUNS);
    let mut stage_totals = StageTimings::default();
    for _ in 0..RUNS {
        let t = Instant::now();
        let result = engine
            .synthesize(PHRASE, "aidar", rate)
            .expect("synthesis must succeed");
        times.push(t.elapsed());
        stage_totals += result.stage_timings;
    }
    times.sort();

    let mean = times.iter().sum::<Duration>() / RUNS as u32;
    // Nearest-rank p95.
    let p95 = times[((RUNS as f64) * 0.95).ceil() as usize - 1];
    let min = times[0];
    let max = times[RUNS - 1];
    println!("== {rate} Hz ==");
    println!("mean {mean:.1?} | p95 {p95:.1?} | min {min:.1?} | max {max:.1?}");
    println!(
        "mean_ms={:.1} p95_ms={:.1} min_ms={:.1} max_ms={:.1}",
        ms(mean),
        ms(p95),
        ms(min),
        ms(max),
    );
    print_stage_breakdown(&stage_totals, ms(mean));
    println!();
}

fn main() {
    // `RUST_LOG=silero_native=info` surfaces the per-session load timings.
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let dir = common::bundle_dir_arg();

    let t = Instant::now();
    let engine = SileroNative::load(&dir).expect("bundle must load");
    let load = t.elapsed();

    println!("phrase: {PHRASE:?} ({} chars)", PHRASE.chars().count());
    println!("runs: {RUNS} per rate, speaker aidar");
    println!("load_ms={:.1}", ms(load));
    println!();

    for rate in RATES {
        run_rate(&engine, rate);
    }
}
