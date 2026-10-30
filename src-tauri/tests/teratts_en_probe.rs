//! Manual listening probe for the TeraTTSv2 English routing decision (#311).
//!
//! It writes `probe.json` with, per fragment kind, the raw fragment, today's
//! `TERATTS_PROFILE` narration and the `NATIVE_EN_PROFILE` output (spans
//! included). `teratts/export/en_probe.py` turns that file into native
//! (`eng_f5`) and narrated (`ru_m5`) WAVs for listening.
//!
//! The verdict (which kinds may flip to `Reading::Native`) is recorded in
//! #311; this test only produces the inputs.
//!
//! ```sh
//! nix develop -c env TERATTS_EN_PROBE=1 \
//!   TERATTS_EN_PROBE_OUT=$PWD/tmp/teratts-en-probe \
//!   cargo test --manifest-path src-tauri/Cargo.toml --test teratts_en_probe \
//!   -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use ruvox_tauri_lib::pipeline::TTSPipeline;
use ruvox_tauri_lib::tts::TtsEngine;
use ruvox_tauri_lib::tts::profiles::{NATIVE_EN_PROFILE, TERATTS_PROFILE};

/// Fragment kinds from the #311 method section.
const FIXTURES: &[(&str, &str)] = &[
    ("prose", "Kubernetes cluster"),
    ("brands", "PostgreSQL, Redis and Nginx"),
    ("identifier", "getUserData and snake_case_name"),
    ("abbrev", "API, HTTP/2 and IPv6"),
    ("url", "https://example.com/path?q=1"),
    ("email", "dev@example.com"),
    ("code", "fn main() { println!(\"hi\"); }"),
];

#[test]
#[ignore = "manual probe: needs TERATTS_EN_PROBE=1 and a downloaded TeraTTS bundle"]
fn teratts_en_probe() {
    if std::env::var("TERATTS_EN_PROBE").as_deref() != Ok("1") {
        println!("skipped: set TERATTS_EN_PROBE=1 to run the probe");
        return;
    }
    let out = PathBuf::from(
        std::env::var("TERATTS_EN_PROBE_OUT").expect("TERATTS_EN_PROBE_OUT must name a directory"),
    );
    std::fs::create_dir_all(&out).expect("create probe output directory");

    let mut pipeline = TTSPipeline::new();
    let mut rows = Vec::new();
    for (kind, text) in FIXTURES {
        let narrated = pipeline.process_for_engine(text, &TERATTS_PROFILE);
        let native = pipeline.process_for_engine(text, &NATIVE_EN_PROFILE);
        println!(
            "{kind}: narrated={:?} spans={:?} | native={:?} spans={:?}",
            narrated.text, narrated.spans, native.text, native.spans
        );
        rows.push(serde_json::json!({
            "kind": kind,
            "raw": text,
            "narrated": narrated.text,
            "native": native.text,
        }));
    }
    std::fs::write(
        out.join("probe.json"),
        serde_json::to_vec_pretty(&rows).expect("serialize probe rows"),
    )
    .expect("write probe.json");
    println!("wrote probe inputs to {}", out.display());
}

/// End-to-end app-adapter smoke: pipeline spans -> language segments -> mixed
/// WAV through `TeraTtsEngine`. Writes `mixed-app.wav` next to the probe
/// inputs and prints the mapped timestamps.
#[tokio::test]
#[ignore = "manual smoke: needs TERATTS_EN_PROBE=1 and a downloaded TeraTTS bundle"]
async fn teratts_mixed_app() {
    if std::env::var("TERATTS_EN_PROBE").as_deref() != Ok("1") {
        println!("skipped: set TERATTS_EN_PROBE=1 to run the probe");
        return;
    }
    let bundle = std::env::var("TERATTS_BUNDLE").expect("TERATTS_BUNDLE must point at the bundle");
    let out = PathBuf::from(
        std::env::var("TERATTS_EN_PROBE_OUT").expect("TERATTS_EN_PROBE_OUT must name a directory"),
    );
    std::fs::create_dir_all(&out).expect("create probe output directory");

    let mut pipeline = TTSPipeline::new();
    let input = std::env::var("TERATTS_EN_PROBE_TEXT").unwrap_or_else(|_| {
        "Мы используем Kubernetes cluster и getUserData() в API. IPv6 и HTTP/2.".to_string()
    });
    let input = input.as_str();
    let processed = pipeline.process_for_engine(input, &TERATTS_PROFILE);
    println!("spans: {:?}", processed.spans);
    println!("text: {:?}", processed.text);

    let emitter: ruvox_tauri_lib::tts::supervisor::Emitter =
        std::sync::Arc::new(|event, payload| println!("event {event}: {payload}"));
    let engine = ruvox_tauri_lib::tts::TeraTtsEngine::new(PathBuf::from(&bundle), emitter);
    let result = engine
        .synthesize_spans(
            processed.text,
            processed.spans,
            "ru_m5".to_string(),
            44_100,
            out.join("mixed-app.wav").display().to_string(),
            None,
        )
        .await
        .expect("mixed synthesis");
    println!(
        "mixed-app.wav: {:.2}s, {} timestamps",
        result.duration_sec,
        result.timestamps.len()
    );
    for word in &result.timestamps {
        println!(
            "  {:?} {:.2}-{:.2} orig={:?}",
            word.word, word.start, word.end, word.original_pos
        );
    }
}
