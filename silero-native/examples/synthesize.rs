//! End-to-end smoke example: load the bundle, synthesize a few phrases at
//! 24000 and 48000 Hz, write tmp/native-*.wav next to the bundle.
//!
//! Usage: cargo run --release --example synthesize -- [bundle_dir]
//! (default: ../tmp/bundle-v5 relative to the crate, or SILERO_NATIVE_BUNDLE).

use std::path::PathBuf;
use std::time::Instant;

use silero_native::SileroNative;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let dir = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        std::env::var("SILERO_NATIVE_BUNDLE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tmp/bundle-v5"))
    });
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tmp");

    println!("loading bundle from {}", dir.display());
    let t0 = Instant::now();
    let engine = SileroNative::load(&dir).expect("bundle must load");
    println!("loaded in {:.1?}", t0.elapsed());

    let phrases = [
        ("homograph", "Открыть замок было непросто, ведь ключи от этого замка потерялись."),
        ("yo", "Ёлка и ёжик стояли под ёлкой, всё было как всегда."),
        ("plain", "Сервер обрабатывает запросы пользователей и сохраняет данные в базу."),
    ];
    for (name, text) in phrases {
        for rate in [24000u32, 48000] {
            let t = Instant::now();
            let result = engine
                .synthesize(text, "aidar", rate)
                .expect("synthesis must succeed");
            let path = out_dir.join(format!("native-{name}-{rate}.wav"));
            std::fs::write(&path, &result.wav).expect("wav must be written");
            println!(
                "{name} @ {rate} Hz: {:.2}s audio, {} wav bytes, {} timestamps, synth took {:.1?} -> {}",
                result.duration_sec,
                result.wav.len(),
                result.timestamps.len(),
                t.elapsed(),
                path.display()
            );
        }
    }
}
