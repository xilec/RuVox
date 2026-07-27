//! Edge-case tests (spec "Edge Cases" + "Speakers and Sample Rates"):
//! invalid inputs must fail with typed `BadInput` before any inference runs.
//! Bundle-gated like the other integration tests.

use std::path::PathBuf;

use silero_native::{EngineError, SileroNative};

fn engine() -> Option<SileroNative> {
    let dir = match std::env::var("SILERO_NATIVE_BUNDLE") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tmp/bundle-v5"),
    };
    if !dir.join("manifest.json").exists() {
        eprintln!("bundle not found, skipping (set SILERO_NATIVE_BUNDLE)");
        return None;
    }
    SileroNative::load(&dir).ok()
}

#[test]
fn empty_and_whitespace_input_rejected() {
    let Some(engine) = engine() else { return };
    for text in ["", "   ", "\n\t "] {
        let err = engine
            .synthesize(text, "aidar", 24000)
            .expect_err("empty input must fail");
        assert!(
            matches!(err, EngineError::BadInput(_)),
            "expected BadInput for {text:?}, got {err}"
        );
    }
}

#[test]
fn unknown_speaker_rejected_before_inference() {
    let Some(engine) = engine() else { return };
    let err = engine
        .synthesize("привет", "unknown", 24000)
        .expect_err("unknown speaker must fail");
    match err {
        EngineError::BadInput(msg) => assert!(msg.contains("unknown"), "msg: {msg}"),
        other => panic!("expected BadInput, got {other}"),
    }
}

#[test]
fn unsupported_sample_rate_rejected() {
    let Some(engine) = engine() else { return };
    let err = engine
        .synthesize("привет", "aidar", 16000)
        .expect_err("unsupported rate must fail");
    assert!(matches!(err, EngineError::BadInput(_)), "got {err}");
}

#[test]
fn punctuation_only_input_rejected() {
    let Some(engine) = engine() else { return };
    let err = engine
        .synthesize("?!.,", "aidar", 24000)
        .expect_err("punctuation-only input must fail");
    assert!(matches!(err, EngineError::BadInput(_)), "got {err}");
}

#[test]
fn markup_is_stripped_and_synthesized() {
    let Some(engine) = engine() else { return };
    let result = engine
        .synthesize("<speak>привет [[zzz]] мир</speak>", "aidar", 24000)
        .expect("markup input must synthesize after stripping");
    assert!(!result.wav.is_empty());
    assert!(result.duration_sec > 0.0);
    // Timestamps are computed over the markup-stripped text.
    let words: Vec<&str> = result.timestamps.iter().map(|t| t.word.as_str()).collect();
    assert_eq!(words, vec!["привет", "мир"]);
}

#[test]
fn multiline_text_synthesizes_without_gluing_words() {
    let Some(engine) = engine() else { return };
    // The pipeline keeps `\n\n` paragraph breaks; the engine must turn them
    // into word separators (ttsd `sanitize_for_silero` parity), not drop
    // them and glue the surrounding words into one.
    let result = engine
        .synthesize("строки\n\nновая", "aidar", 24000)
        .expect("multiline input must synthesize");
    assert!(!result.wav.is_empty());
    let words: Vec<&str> = result.timestamps.iter().map(|t| t.word.as_str()).collect();
    assert_eq!(words, vec!["строки", "новая"]);
}
