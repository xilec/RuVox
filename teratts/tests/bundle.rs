//! Bundle-gated integration tests: real ONNX sessions against a real bundle.
//!
//! Skipped unless `TERATTS_BUNDLE` points at a verified bundle directory, so
//! the default `cargo test` stays hermetic. The CI job that downloads the
//! bundle runs this file.

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use teratts::{Engine, Error, Language, SAMPLE_RATE, Segment};
use tts_contract::{Chunk, Limit, Pauses};

fn bundle() -> Option<PathBuf> {
    std::env::var_os("TERATTS_BUNDLE").map(PathBuf::from)
}

fn policy() -> Chunk {
    Chunk {
        limit: Limit::Secs(20),
        pauses: Pauses {
            paragraph_ms: 250,
            segment_ms: 200,
        },
    }
}

#[test]
fn synthesizes_russian_text() {
    let Some(dir) = bundle() else {
        eprintln!("skipping: TERATTS_BUNDLE is not set");
        return;
    };
    let engine = Engine::load(&dir).expect("load bundle");
    let cancelled = AtomicBool::new(false);
    let out = engine
        .synthesize(
            "Привет, мир. Это проверка синтеза TeraTTS.",
            "ru_m5",
            &policy(),
            &cancelled,
        )
        .expect("synthesize");

    assert!(out.duration_sec > 1.0, "duration {}", out.duration_sec);
    let reader = hound::WavReader::new(Cursor::new(&out.wav)).expect("wav");
    assert_eq!(reader.spec().channels, 1);
    assert_eq!(reader.spec().sample_rate, SAMPLE_RATE);
    assert_eq!(reader.spec().bits_per_sample, 16);

    assert!(!out.timestamps.is_empty());
    let mut previous = 0.0;
    for word in &out.timestamps {
        assert!(word.start >= previous - 1e-9, "{word:?}");
        assert!(word.end >= word.start, "{word:?}");
        assert!(word.end <= out.duration_sec + 1e-6, "{word:?}");
        previous = word.end;
    }
    assert!(
        out.timestamps
            .iter()
            .any(|word| word.word.contains("Привет"))
    );
}

/// Raw engine input (the app pipeline would expand the numbers): digits and a
/// standalone em dash are outside the model alphabet, so the alphabet gate
/// drops them and they have no counterpart in the prepared text. The timestamp
/// estimator must skip those tokens instead of rejecting the chunk
/// (regression: "word count changed during preparation: 36 -> 29").
#[test]
fn synthesizes_raw_text_with_tokens_dropped_by_the_alphabet() {
    let Some(dir) = bundle() else {
        eprintln!("skipping: TERATTS_BUNDLE is not set");
        return;
    };
    let engine = Engine::load(&dir).expect("load bundle");
    let cancelled = AtomicBool::new(false);
    let out = engine
        .synthesize(
            "В 2026 году API вернул 404 — ошибку, а скидка 15 %.",
            "ru_m5",
            &policy(),
            &cancelled,
        )
        .expect("synthesize");

    assert!(out.duration_sec > 0.5, "duration {}", out.duration_sec);
    assert!(!out.timestamps.is_empty());
}

#[test]
fn synthesizes_mixed_language_segments() {
    let Some(dir) = bundle() else {
        eprintln!("skipping: TERATTS_BUNDLE is not set");
        return;
    };
    let engine = Engine::load(&dir).expect("load bundle");
    let cancelled = AtomicBool::new(false);
    let segments = [
        Segment {
            text: "Привет. ",
            language: Language::Ru,
            text_start: 0,
        },
        Segment {
            text: "Kubernetes cluster",
            language: Language::En,
            text_start: 8,
        },
        Segment {
            text: " готов.",
            language: Language::Ru,
            text_start: 26,
        },
    ];
    let out = engine
        .synthesize_segments(&segments, "ru_m5", &policy(), &cancelled, 200)
        .expect("synthesize mixed");

    assert!(out.duration_sec > 2.0, "duration {}", out.duration_sec);
    assert!(!out.samples.is_empty());
    let english = out
        .timestamps
        .iter()
        .find(|word| word.word.contains("Kubernetes"))
        .expect("english word timestamp");
    assert!(english.start_char >= 8, "{english:?}");
    assert!(english.start > 0.0, "{english:?}");
    let mut previous = 0.0;
    for word in &out.timestamps {
        assert!(word.start >= previous - 1e-9, "{word:?}");
        previous = word.end;
    }
}

#[test]
fn cancels_before_synthesis() {
    let Some(dir) = bundle() else {
        eprintln!("skipping: TERATTS_BUNDLE is not set");
        return;
    };
    let engine = Engine::load(&dir).expect("load bundle");
    let cancelled = AtomicBool::new(true);
    let error = engine
        .synthesize("Привет, мир.", "ru_m5", &policy(), &cancelled)
        .expect_err("cancelled");
    assert!(matches!(error, Error::Cancelled), "{error:?}");
}

#[test]
fn rejects_an_unknown_voice() {
    let Some(dir) = bundle() else {
        eprintln!("skipping: TERATTS_BUNDLE is not set");
        return;
    };
    let engine = Engine::load(&dir).expect("load bundle");
    let cancelled = AtomicBool::new(false);
    let error = engine
        .synthesize("Привет, мир.", "ru_m9", &policy(), &cancelled)
        .expect_err("unknown voice");
    assert!(matches!(error, Error::VoiceUnknown { .. }), "{error:?}");
}
