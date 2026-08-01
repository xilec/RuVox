//! Performance regression lock for the normalization pipeline (openspec
//! change `fix-pipeline-quadratic`).
//!
//! Pre-fix `TrackedText` was O(M·n + M²): ~2 s at 256 KB and ~28 s at 1 MB
//! of dense markup in release, minutes in debug. The reworked batch apply
//! normalizes 1 MB in ~0.5 s (release). Budgets sit far above post-fix
//! measurements so the tests stay green on loaded CI machines running debug
//! builds, while a quadratic implementation blows straight through them.
//!
//! The 30 s budget is load-bearing: GitHub runners take 12-16 s for the 1 MB
//! debug-build pass (vs ~3 s on a dev machine), so the original 10 s budget
//! failed deterministically in CI (see PR #157's own red run).

use ruvox_tauri_lib::pipeline::TTSPipeline;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Serializes the two timing tests: `cargo test` runs tests of one binary in
/// parallel threads, which would skew wall-time measurements.
static PERF_LOCK: Mutex<()> = Mutex::new(());

/// One dense, replacement-heavy HTML fragment: nested tags, attributes, a
/// URL with an entity, Cyrillic text mixed with Latin identifiers and a
/// version string.
const HTML_UNIT: &str = concat!(
    "<div class=\"container content-block user-card\" id=\"card-1234\" data-user-id=\"5678\">",
    "<a href=\"https://example.com/users/profile?id=5678&amp;tab=settings\">",
    "<span class=\"username\">Иван Петров</span>",
    "<img src=\"https://cdn.example.com/images/avatar_256x256.png\" alt=\"Аватар\">",
    "</a><p class=\"description\">Текст &amp; данные, версия v2.3.1.</p></div>\n"
);

fn dense_markup(target_bytes: usize) -> String {
    let mut s = String::with_capacity(target_bytes + HTML_UNIT.len());
    while s.len() < target_bytes {
        s.push_str(HTML_UNIT);
    }
    s
}

/// Spec scenario "Large replacement-heavy input normalizes within budget".
#[test]
fn dense_markup_1mb_normalizes_within_budget() {
    let _guard = PERF_LOCK
        .lock()
        .expect("perf test serialization lock poisoned");
    let mut pipeline = TTSPipeline::new();
    let input = dense_markup(1_000_000);

    let start = Instant::now();
    let (normalized, mapping) = pipeline.process_with_char_mapping(&input);
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(30),
        "1 MB dense markup normalized in {elapsed:?}, budget is 30 s"
    );
    assert_eq!(
        mapping.char_map.len(),
        normalized.chars().count(),
        "char_map length must equal the transformed codepoint count"
    );
}

/// Spec scenario "Doubling the input scales near-linearly".
#[test]
fn dense_markup_scaling_is_near_linear() {
    let _guard = PERF_LOCK
        .lock()
        .expect("perf test serialization lock poisoned");
    let mut pipeline = TTSPipeline::new();
    let input_n = dense_markup(128_000);
    let input_2n = dense_markup(256_000);

    let start = Instant::now();
    let _ = pipeline.process(&input_n);
    let time_n = start.elapsed();

    let start = Instant::now();
    let _ = pipeline.process(&input_2n);
    let time_2n = start.elapsed();

    // A quadratic implementation grows ~4x or worse per doubling; near-linear
    // stays close to 2x, so the 4x threshold separates the two with margin.
    assert!(
        time_2n < time_n * 4,
        "time(2n)={time_2n:?} must be < 4x time(n)={time_n:?}"
    );
}
