//! Manual measurement probe for the Piper per-chunk synthesis limit (#155).
//!
//! Runs a real `Piper::create` on synthetic Russian texts of increasing size
//! and prints peak process memory (`VmHWM`) plus current RSS around each run,
//! so `PIPER_MAX_CHUNK_CHARS` is picked from numbers rather than guesses.
//!
//! Requires a downloaded Piper voice (the app fetches it on demand) and
//! `ORT_DYLIB_PATH` pointing at `libonnxruntime.so` — the devshell sets it
//! (`nix/devshell.nix`); without it ort's `load-dynamic` hangs instead of
//! erroring. Gated twice so it never runs incidentally: `#[ignore]` keeps it
//! out of normal test runs, and `RUVOX_PIPER_LIMIT_PROBE=1` keeps it out of
//! `--ignored` runs. Voice: `ruslan` by default, override with
//! `RUVOX_PIPER_PROBE_VOICE`.
//!
//! ```sh
//! nix develop -c env RUVOX_PIPER_LIMIT_PROBE=1 cargo test \
//!   --manifest-path src-tauri/Cargo.toml --test piper_chunk_limit_probe \
//!   -- --ignored --nocapture
//! ```

use ruvox_tauri_lib::paths::voices_root;
use ruvox_tauri_lib::tts::chunking::split_with_limit;

/// Chunk sizes to probe, in codepoints. Spans the production constant (500)
/// and the freeze threshold direction (the unchunked #155 reproduction was
/// ~22 000 codepoints).
const PROBE_SIZES: &[usize] = &[300, 600, 900, 1200, 1800];

#[test]
#[ignore = "manual probe: needs a downloaded Piper voice and RUVOX_PIPER_LIMIT_PROBE=1"]
fn piper_chunk_limit_probe() {
    if std::env::var("RUVOX_PIPER_LIMIT_PROBE").as_deref() != Ok("1") {
        println!("skipped: set RUVOX_PIPER_LIMIT_PROBE=1 to run the probe");
        return;
    }

    let voice = std::env::var("RUVOX_PIPER_PROBE_VOICE").unwrap_or_else(|_| "ruslan".to_string());
    let Some(voices_root) = voices_root() else {
        panic!("cannot resolve the voices root");
    };
    let voices_dir = voices_root.join("piper");
    let config_path = voices_dir
        .join(&voice)
        .join(format!("ru_RU-{voice}-medium.onnx.json"));
    assert!(
        config_path.exists(),
        "voice not installed: {} (open RuVox with the Piper engine once to download it)",
        config_path.display()
    );

    let mut piper = new_piper(&config_path);

    println!("=== Piper chunk-limit probe (voice: {voice}) ===");
    for &size in PROBE_SIZES {
        let text = synthetic_text(size);
        let (before_rss, before_hwm) = memory_kb();
        let started = std::time::Instant::now();
        // The probe measures a single unchunked `create` per size — exactly
        // the operation the production chunk loop bounds.
        let (samples, _sr) = piper
            .create(&text, false, None, Some(0.8), None, None)
            .expect("probe inference must succeed");
        let elapsed = started.elapsed();
        let (after_rss, after_hwm) = memory_kb();
        println!(
            "size {size:>5} cp | {} samples | {elapsed:>7.1?} | RSS {before_rss}->{after_rss} kB | peak HWM {after_hwm} kB (Δ{})",
            samples.len(),
            after_hwm.saturating_sub(before_hwm),
        );
    }
    println!("=== pick PIPER_MAX_CHUNK_CHARS from the peak deltas above ===");
}

/// Build a synthetic Russian text of at least `size` codepoints from
/// sentence-shaped input, so the probe text resembles real normalized prose
/// (sentence-bounded, mixed word lengths).
fn synthetic_text(size: usize) -> String {
    let sentence = "Проверка синтеза длинного текста движком Piper в приложении Рувокс. ";
    let mut text = String::with_capacity(size + sentence.len());
    while text.chars().count() < size {
        text.push_str(sentence);
    }
    // Cut on a sentence boundary at or below `size` so the probe inputs are
    // comparable across sizes.
    let chunks = split_with_limit(&text, size);
    chunks.into_iter().next().map(|(c, _)| c).unwrap_or(text)
}

fn new_piper(config_path: &std::path::Path) -> piper_rs::Piper {
    // piper-rs wants the config path with the trailing `.json` stripped for
    // the model (see `load_voice_blocking` in the engine).
    let model_path = config_path.with_extension("");
    piper_rs::Piper::new(&model_path, config_path).expect("probe voice must load")
}

/// (current RSS, peak RSS) of this process in kB; `None` outside Linux.
fn memory_kb() -> (u64, u64) {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").expect("read /proc/self/status");
        let mut rss = None;
        let mut hwm = None;
        for line in status.lines() {
            if let Some(v) = line.strip_prefix("VmRSS:") {
                rss = parse_kb(v);
            } else if let Some(v) = line.strip_prefix("VmHWM:") {
                hwm = parse_kb(v);
            }
        }
        (rss.unwrap_or(0), hwm.unwrap_or(0))
    }
    #[cfg(not(target_os = "linux"))]
    {
        (0, 0)
    }
}

#[cfg(target_os = "linux")]
fn parse_kb(value: &str) -> Option<u64> {
    value.split_whitespace().next()?.parse().ok()
}
