//! A streamed synthesis must produce exactly the samples the buffered call
//! does, in the same order — otherwise switching a consumer to streaming would
//! silently change the audio.
//!
//! Needs a real model bundle and ONNX Runtime:
//!
//! ```text
//! TERATTS_BUNDLE=/path/to/bundle \
//! ORT_DYLIB_PATH=/path/to/libonnxruntime.so \
//!   cargo test -p teratts --test stream
//! ```
//!
//! Without `TERATTS_BUNDLE` the tests report a skip instead of failing: the
//! bundle is a separate download and CI has no copy of it. The pause arithmetic
//! they exercise is unit-tested without a bundle in `src/engine.rs`.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use teratts::{Engine, Language, SAMPLE_RATE, Segment};
use tts_contract::{Chunk, Limit, Pauses};

/// Long enough that the chunk budget splits it into several parts with pauses.
const TEXT: &str = "Привет, это короткая проверка потоковой синтезации. \
Второе предложение подлиннее, чтобы чанкер разрезал текст на несколько кусков \
и вставил паузы между ними.";

fn bundle() -> Option<PathBuf> {
    std::env::var_os("TERATTS_BUNDLE").map(PathBuf::from)
}

fn load(dir: &Path) -> (Engine, String) {
    let engine = Engine::load(dir).expect("bundle loads");
    let voice = engine
        .voices()
        .iter()
        .find(|voice| voice.language == "ru")
        .or_else(|| engine.voices().first())
        .expect("the bundle ships at least one voice")
        .id
        .clone();
    (engine, voice)
}

/// A small chunk budget so the stream has several chunks (and therefore chunk
/// pauses) instead of one; the pause durations are the profile's.
fn policy() -> Chunk {
    Chunk {
        limit: Limit::Chars(40),
        pauses: Pauses {
            paragraph_ms: 250,
            segment_ms: 200,
        },
    }
}

fn seconds(samples: usize) -> f64 {
    samples as f64 / f64::from(SAMPLE_RATE)
}

#[test]
fn stream_matches_buffered_samples() {
    let Some(dir) = bundle() else {
        eprintln!("TERATTS_BUNDLE is not set: skipping the streaming parity test");
        return;
    };
    let (engine, voice) = load(&dir);
    let policy = policy();
    let cancelled = AtomicBool::new(false);
    let segments = [Segment {
        text: TEXT,
        language: Language::Ru,
        text_start: 0,
    }];

    let buffered = engine
        .synthesize_segments(&segments, &voice, &policy, &cancelled, 0)
        .expect("buffered synthesis");

    let mut streamed: Vec<f32> = Vec::new();
    let mut parts = 0usize;
    let mut pauses = 0usize;
    let mut text_starts: Vec<usize> = Vec::new();
    let summary = engine
        .synthesize_stream(&segments, &voice, &policy, &cancelled, 0, &mut |part| {
            parts += 1;
            assert_eq!(
                part.start_sec,
                seconds(streamed.len()),
                "each part starts where the emitted audio ended"
            );
            match part.text_start {
                Some(text_start) => text_starts.push(text_start),
                None => pauses += 1,
            }
            streamed.extend_from_slice(part.samples);
            Ok(())
        })
        .expect("streamed synthesis");

    assert!(
        parts > 2 && pauses > 0,
        "the chunk budget must split the text and insert pauses, got {parts} parts / {pauses} pauses"
    );
    assert!(
        text_starts.windows(2).all(|pair| pair[0] < pair[1]),
        "chunk offsets advance through the text: {text_starts:?}"
    );
    assert_eq!(summary.parts, parts, "summary counts emitted parts");
    assert_eq!(summary.samples, streamed.len(), "summary counts samples");
    assert_eq!(
        summary.duration_sec,
        seconds(streamed.len()),
        "duration is derived from the sample count"
    );
    assert_eq!(streamed, buffered.samples, "streaming changed the audio");
    assert_eq!(summary.duration_sec, buffered.duration_sec);
}

#[test]
fn stream_emits_segment_pause_between_languages() {
    let Some(dir) = bundle() else {
        eprintln!("TERATTS_BUNDLE is not set: skipping the streaming pause test");
        return;
    };
    let (engine, voice) = load(&dir);
    let policy = policy();
    let cancelled = AtomicBool::new(false);
    let segments = [
        Segment {
            text: "Привет.",
            language: Language::Ru,
            text_start: 0,
        },
        Segment {
            text: "Hello.",
            language: Language::En,
            text_start: 8,
        },
    ];

    let buffered = engine
        .synthesize_segments(&segments, &voice, &policy, &cancelled, 120)
        .expect("buffered synthesis");

    let mut streamed: Vec<f32> = Vec::new();
    let mut parts = 0usize;
    let mut paused = 0usize;
    let summary = engine
        .synthesize_stream(&segments, &voice, &policy, &cancelled, 120, &mut |part| {
            parts += 1;
            if part.text_start.is_none() {
                paused += 1;
                assert_eq!(
                    part.start_sec,
                    seconds(streamed.len()),
                    "the segment pause starts where the last segment ended"
                );
                assert!(part.samples.iter().all(|sample| *sample == 0.0));
            }
            streamed.extend_from_slice(part.samples);
            Ok(())
        })
        .expect("streamed synthesis");

    assert_eq!(summary.parts, parts);
    assert_eq!(paused, 1, "one pause part between the two segments");
    assert_eq!(
        streamed, buffered.samples,
        "the segment pause matches the buffered path"
    );
}
