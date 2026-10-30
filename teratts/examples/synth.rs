//! Synthesize a text with the TeraTTSv2 engine and write a WAV.
//!
//! Used for the manual listening pass and the perf smoke:
//!
//! ```bash
//! TERATTS_BUNDLE=tmp/teratts-bundle TERATTS_TEXT="Привет, мир." \
//! TERATTS_OUT=tmp/teratts-smoke.wav \
//!   cargo run --release --manifest-path teratts/Cargo.toml --example synth
//! ```
//!
//! `TERATTS_VOICE` selects the voice (`ru_m5` by default), `TERATTS_THREADS`
//! the ONNX intra-op thread count (default: half the cores). The example
//! prints load time, synthesis time and the real-time factor.

use std::sync::atomic::AtomicBool;
use std::time::Instant;

use teratts::Engine;
use tts_contract::{Chunk, Limit, Pauses};

fn main() {
    let bundle = std::env::var("TERATTS_BUNDLE").expect("TERATTS_BUNDLE is not set");
    let text = std::env::var("TERATTS_TEXT")
        .unwrap_or_else(|_| "Привет, мир. Это проверка синтеза TeraTTS.".to_string());
    let voice = std::env::var("TERATTS_VOICE").unwrap_or_else(|_| "ru_m5".to_string());
    let out = std::env::var("TERATTS_OUT").unwrap_or_else(|_| "teratts-smoke.wav".to_string());

    let threads = std::env::var("TERATTS_THREADS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or_else(teratts::default_threads);
    let load_started = Instant::now();
    let engine =
        Engine::load_with_threads(std::path::Path::new(&bundle), threads).expect("load bundle");
    println!(
        "load: {:.2} s ({} threads)",
        load_started.elapsed().as_secs_f64(),
        threads
    );

    let policy = Chunk {
        limit: Limit::Secs(20),
        pauses: Pauses {
            paragraph_ms: 250,
            segment_ms: 200,
        },
    };
    let cancelled = AtomicBool::new(false);
    let synthesis_started = Instant::now();
    let result = engine
        .synthesize(&text, &voice, &policy, &cancelled)
        .expect("synthesize");
    let elapsed = synthesis_started.elapsed().as_secs_f64();
    std::fs::write(&out, &result.wav).expect("write wav");
    println!(
        "synthesis: {:.2} s for {:.2} s of audio (RTF {:.3}), {} words, model {}",
        elapsed,
        result.duration_sec,
        elapsed / result.duration_sec,
        result.timestamps.len(),
        engine.model_info().name
    );
    println!("wrote {out}");
}
