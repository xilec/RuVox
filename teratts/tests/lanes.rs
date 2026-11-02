//! The lane pool must not change the audio: the same text synthesized on one
//! lane and on four lanes, with the same per-lane thread count, must produce
//! identical samples and timestamps.
//!
//! Needs a real model bundle and ONNX Runtime:
//!
//! ```text
//! TERATTS_BUNDLE=/path/to/bundle \
//! ORT_DYLIB_PATH=/path/to/libonnxruntime.so \
//!   cargo test -p teratts --test lanes
//! ```
//!
//! Without `TERATTS_BUNDLE` the tests report a skip instead of failing: the
//! bundle is a separate download and CI has no copy of it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

use teratts::{Engine, Error, Language, Segment};
use tts_contract::{Chunk, Limit, Pauses};

/// Long enough that the chunk budget splits it into several chunks; the split
/// points are sentence boundaries, so the text's own paragraphs do not matter.
const TEXT: &str = "Первый абзац объясняет, зачем нужен параллельный синтез. \
Второй абзац добавляет подробности о дорожках и бюджете потоков. \
Третий абзац проверяет, что порядок сборки не зависит от планировщика. \
Четвёртый абзац закрывает проверку и повторяет основной вывод.";

fn bundle() -> Option<PathBuf> {
    std::env::var_os("TERATTS_BUNDLE").map(PathBuf::from)
}

fn voice(engine: &Engine) -> String {
    engine
        .voices()
        .iter()
        .find(|voice| voice.language == "ru")
        .or_else(|| engine.voices().first())
        .expect("the bundle ships at least one voice")
        .id
        .clone()
}

/// A small chunk budget so the text becomes several chunks instead of one.
fn policy() -> Chunk {
    Chunk {
        limit: Limit::Chars(64),
        pauses: Pauses {
            paragraph_ms: 250,
            segment_ms: 200,
        },
    }
}

fn load(dir: &Path, lanes: usize) -> Engine {
    // One intra-op thread per lane keeps the two runs numerically identical:
    // ONNX Runtime's reduction order depends on the thread count, not on which
    // lane a chunk runs on.
    Engine::load_with_lanes(dir, lanes, 1).expect("bundle loads")
}

/// Compact comparison of two buffers: lengths and where they first diverge.
fn describe(left: &[f32], right: &[f32]) -> String {
    if left.len() != right.len() {
        return format!("length {} vs {}", left.len(), right.len());
    }
    let Some(first) = (0..left.len()).find(|&index| left[index] != right[index]) else {
        return "equal".to_string();
    };
    let last = (0..left.len())
        .rev()
        .find(|&index| left[index] != right[index])
        .unwrap_or(first);
    let max_diff = (first..=last)
        .filter(|&index| left[index] != right[index])
        .map(|index| (left[index] - right[index]).abs())
        .fold(0.0f32, f32::max);
    format!(
        "{} samples differ from {first} to {last} (max diff {max_diff:e})",
        (first..=last)
            .filter(|&index| left[index] != right[index])
            .count()
    )
}

/// The bundle-gated tests each load engines in this one process. Creating
/// ONNX Runtime sessions concurrently makes a session occasionally compute
/// differently: one four-lane load in ten produced a different chunk while
/// loading lanes one after another was stable, so the tests take turns.
static LANE_TESTS: Mutex<()> = Mutex::new(());

fn exclusive() -> MutexGuard<'static, ()> {
    LANE_TESTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn lane_count_does_not_change_the_audio() {
    let _guard = exclusive();
    let Some(dir) = bundle() else {
        eprintln!("TERATTS_BUNDLE is not set: skipping the lane parity test");
        return;
    };
    let single = load(&dir, 1);
    let four = load(&dir, 4);
    let voice = voice(&single);
    let policy = policy();
    let cancelled = AtomicBool::new(false);
    let segments = [Segment {
        text: TEXT,
        language: Language::Ru,
        text_start: 0,
    }];

    let one = single
        .synthesize_segments(&segments, &voice, &policy, &cancelled, 120)
        .expect("single lane");
    let parallel = four
        .synthesize_segments(&segments, &voice, &policy, &cancelled, 120)
        .expect("four lanes");

    assert!(
        one.timestamps.len() > 20,
        "the text must split into several chunks, got {} words",
        one.timestamps.len()
    );
    if one.samples != parallel.samples {
        // A mismatch is rare and would otherwise print two 18-second buffers:
        // re-synthesize each engine once more to say which side is unstable.
        let one_again = single
            .synthesize_segments(&segments, &voice, &policy, &cancelled, 120)
            .expect("single lane repeat");
        let four_again = four
            .synthesize_segments(&segments, &voice, &policy, &cancelled, 120)
            .expect("four lanes repeat");
        panic!(
            "lane count changed the samples: single_vs_four {}, single_vs_repeat {}, four_vs_repeat {}",
            describe(&one.samples, &parallel.samples),
            describe(&one.samples, &one_again.samples),
            describe(&parallel.samples, &four_again.samples),
        );
    }
    assert_eq!(
        one.timestamps, parallel.timestamps,
        "lane count changed the timestamps"
    );
    assert_eq!(one.duration_sec, parallel.duration_sec);
}

#[test]
fn a_cancelled_parallel_synthesis_returns_no_audio() {
    let _guard = exclusive();
    let Some(dir) = bundle() else {
        eprintln!("TERATTS_BUNDLE is not set: skipping the lane cancellation test");
        return;
    };
    let engine = load(&dir, 4);
    let voice = voice(&engine);
    let policy = policy();
    let cancelled = AtomicBool::new(true);
    let segments = [Segment {
        text: TEXT,
        language: Language::Ru,
        text_start: 0,
    }];

    let error = engine
        .synthesize_segments(&segments, &voice, &policy, &cancelled, 120)
        .expect_err("cancelled before the first chunk");

    assert!(matches!(error, Error::Cancelled), "{error:?}");
}

#[test]
fn cancellation_after_the_first_streamed_part_stops_the_lanes() {
    let _guard = exclusive();
    let Some(dir) = bundle() else {
        eprintln!("TERATTS_BUNDLE is not set: skipping the stream cancellation test");
        return;
    };
    let engine = load(&dir, 4);
    let voice = voice(&engine);
    let policy = policy();
    let cancelled = AtomicBool::new(false);
    let segments = [Segment {
        text: TEXT,
        language: Language::Ru,
        text_start: 0,
    }];

    let mut chunk_parts = 0usize;
    let result =
        engine.synthesize_stream(&segments, &voice, &policy, &cancelled, 120, &mut |part| {
            if part.text_start.is_some() {
                chunk_parts += 1;
            }
            // Cancel from the consumer while the other lanes are still
            // synthesizing, so the executor's dispatch and discard path runs.
            cancelled.store(true, Ordering::Relaxed);
            Ok(())
        });

    let error = result.expect_err("a cancel after the first part must reach the caller");
    assert!(matches!(error, Error::Cancelled), "{error:?}");
    assert_eq!(
        chunk_parts, 1,
        "no chunk is emitted after the cancellation, only the pause that follows the first one"
    );
}
