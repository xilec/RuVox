//! Manual listening probe for the per-engine native-reading decision (#311).
//!
//! Writes raw and today's narrated WAVs for each fragment kind for the engines
//! that can be driven in-process here: Piper and Silero Native. TeraTTSv2 has
//! its own probe (`teratts_en_probe.rs`) because its adapter needs language
//! tags; the Silero `ttsd` sidecar shares the Silero Native model alphabet, so
//! the Silero Native raw sample is the evidence for both Russian-only engines.
//!
//! Gated twice so it never runs incidentally: `#[ignore]` keeps it out of
//! normal test runs, and `RUVOX_NATIVE_PROBE=1` keeps it out of `--ignored`.
//!
//! ```sh
//! nix develop -c env RUVOX_NATIVE_PROBE=1 \
//!   RUVOX_NATIVE_PROBE_OUT=$PWD/tmp/native-reading-probe \
//!   cargo test --manifest-path src-tauri/Cargo.toml \
//!   --test native_reading_probe -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ruvox_tauri_lib::paths::voices_root;
use ruvox_tauri_lib::pipeline::TTSPipeline;
use ruvox_tauri_lib::tts::profiles::{PIPER_PROFILE, SILERO_NATIVE_PROFILE};
use ruvox_tauri_lib::tts::supervisor::Emitter;
use ruvox_tauri_lib::tts::{PiperEngine, SileroNativeEngine, TtsEngine};

/// Fragment kinds from the #311 method section, shared with `teratts_en_probe`.
const FIXTURES: &[(&str, &str)] = &[
    ("prose", "Kubernetes cluster"),
    ("brands", "PostgreSQL, Redis and Nginx"),
    ("identifier", "getUserData and snake_case_name"),
    ("abbrev", "API, HTTP/2 and IPv6"),
    ("url", "https://example.com/path?q=1"),
    ("email", "dev@example.com"),
    ("code", "fn main() { println!(\"hi\"); }"),
];

#[tokio::test]
#[ignore = "manual probe: needs RUVOX_NATIVE_PROBE=1 and installed engine models"]
async fn native_reading_probe() {
    if std::env::var("RUVOX_NATIVE_PROBE").as_deref() != Ok("1") {
        println!("skipped: set RUVOX_NATIVE_PROBE=1 to run the probe");
        return;
    }
    let out = PathBuf::from(
        std::env::var("RUVOX_NATIVE_PROBE_OUT")
            .expect("RUVOX_NATIVE_PROBE_OUT must name a directory"),
    );
    std::fs::create_dir_all(&out).expect("create probe output directory");

    let voices = voices_root().expect("cannot resolve the voices root");
    let emitter: Emitter = Arc::new(|event, payload| {
        if event == "model_error" {
            eprintln!("engine event {event}: {payload}");
        }
    });

    let mut pipeline = TTSPipeline::new();
    let mut rows = Vec::new();

    let piper_voice =
        std::env::var("RUVOX_PIPER_PROBE_VOICE").unwrap_or_else(|_| "ruslan".to_string());
    let piper_dir = voices.join("piper");
    let piper = PiperEngine::new(piper_dir.clone(), piper_voice.clone(), Arc::clone(&emitter));
    let piper_ready = piper_dir
        .join(&piper_voice)
        .join(format!("ru_RU-{piper_voice}-medium.onnx"))
        .exists();
    if piper_ready {
        piper.warmup().await.expect("Piper probe warmup");
    } else {
        println!("Piper voice {piper_voice:?} is not installed; skipping its samples");
    }

    let silero_dir = voices.join("silero-native");
    let silero_ready = silero_dir.join("manifest.json").exists();
    let silero = SileroNativeEngine::new(silero_dir, Arc::clone(&emitter));
    if silero_ready {
        silero.warmup().await.expect("Silero Native probe warmup");
    } else {
        println!("Silero Native bundle is not installed; skipping its samples");
    }

    for (kind, text) in FIXTURES {
        let piper_narrated = pipeline.process_for_engine(text, &PIPER_PROFILE).text;
        let silero_narrated = pipeline
            .process_for_engine(text, &SILERO_NATIVE_PROFILE)
            .text;

        let mut row = serde_json::json!({
            "kind": kind,
            "raw": text,
            "piper": { "narrated": piper_narrated },
            "silero_native": { "narrated": silero_narrated },
        });

        if piper_ready {
            for (variant, input) in [("raw", (*text).to_string()), ("narrated", piper_narrated)] {
                let file = format!("piper-{kind}-{variant}.wav");
                let path = out.join(&file);
                match piper
                    .synthesize(
                        input,
                        piper_voice.clone(),
                        22_050,
                        path.display().to_string(),
                        None,
                    )
                    .await
                {
                    Ok(result) => {
                        row["piper"][variant] =
                            serde_json::json!({ "file": file, "seconds": result.duration_sec })
                    }
                    Err(e) => row["piper"][variant] = serde_json::json!({ "error": e.to_string() }),
                }
            }
        }

        if silero_ready {
            for (variant, input) in [("raw", (*text).to_string()), ("narrated", silero_narrated)] {
                let file = format!("silero_native-{kind}-{variant}.wav");
                let path = out.join(&file);
                match silero
                    .synthesize(
                        input,
                        "xenia".to_string(),
                        48_000,
                        path.display().to_string(),
                        None,
                    )
                    .await
                {
                    Ok(result) => {
                        row["silero_native"][variant] =
                            serde_json::json!({ "file": file, "seconds": result.duration_sec })
                    }
                    Err(e) => {
                        row["silero_native"][variant] =
                            serde_json::json!({ "error": e.to_string() })
                    }
                }
            }
        }

        rows.push(row);
    }

    std::fs::write(
        out.join("probe.json"),
        serde_json::to_vec_pretty(&rows).expect("serialize probe rows"),
    )
    .expect("write probe.json");
    println!("wrote probe samples to {}", out.display());
    println!("listen per kind: raw vs narrated for each available engine");
}

#[allow(dead_code)]
fn _assert_paths(_: &Path) {}
