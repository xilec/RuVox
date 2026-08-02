//! Word-timestamp parity suite (issue #145): the engine's dur_hat-derived
//! per-symbol frame counts must match the reference `dur_hat` captured in
//! the parity fixtures by the Python ONNX pipeline, their sum must equal the
//! rendered waveform length, and word onsets must follow the reference
//! cumulative durations. Timestamps must also be invariant to the output
//! sample rate (the frame timeline is pre-ISTFT at 48 kHz; PQMF downsampling
//! is time-preserving).
//!
//! Bundle-gated like `parity.rs` (and needs the regenerated fixtures with
//! `sequence`/`dur_hat` fields — see `tests/tools/gen_parity_fixtures.py`).

use serde::Deserialize;
use silero_native::SileroNative;
use silero_native::engine::Engine;

mod common;

/// One `dur_hat` frame in seconds (600 samples @ 48 kHz).
const FRAME_SEC: f32 = 600.0 / 48000.0;
/// Acceptance: word onsets within 50 ms of the model reference (issue #145).
const ONSET_TOL_SEC: f32 = 0.050;

#[derive(Deserialize)]
struct FixtureFile {
    cases: Vec<FixtureCase>,
}

#[derive(Deserialize)]
struct FixtureCase {
    id: String,
    input: String,
    speaker: String,
    sample_rate: u32,
    /// Reference model input sequence (sos/eos included).
    sequence: Vec<i64>,
    /// Reference per-symbol frame durations from the Python ONNX pipeline.
    dur_hat: Vec<f32>,
}

fn fixtures() -> FixtureFile {
    let raw = include_str!("fixtures/parity/parity.json");
    serde_json::from_str(raw).expect("fixture JSON must parse")
}

#[test]
fn dur_hat_frames_match_reference_and_waveform() {
    let Some(dir) = common::gated_bundle_dir() else {
        return;
    };
    let engine = Engine::load(&dir).expect("bundle must load");

    for case in &fixtures().cases {
        let out = engine
            .synthesize(&case.input, &case.speaker, case.sample_rate)
            .unwrap_or_else(|e| panic!("synthesis failed for {}: {e}", case.id));

        assert_eq!(
            out.durations.len(),
            case.sequence.len(),
            "symbol count mismatch for {}",
            case.id
        );
        assert_eq!(
            case.dur_hat.len(),
            case.sequence.len(),
            "fixture dur_hat/sequence length mismatch for {}",
            case.id
        );

        // Per-symbol integer frame counts: same trunc(dur + 0.5) conversion
        // in f32 on both sides (JSON round-trips f32 exactly through f64).
        let mut total_frames = 0usize;
        for (i, (sd, &ref_dur)) in out.durations.iter().zip(&case.dur_hat).enumerate() {
            let expected = (ref_dur + 0.5).trunc().max(0.0) as u32;
            assert_eq!(
                sd.frames, expected,
                "frame count mismatch for {} at symbol {i}",
                case.id
            );
            total_frames += sd.frames as usize;
        }

        // The frame counts the graph rendered must reproduce the waveform
        // length (±1 frame: PQMF downsampling rounds the sample count).
        let rendered_48k = out.samples.len() * 48000 / case.sample_rate as usize;
        let diff = (total_frames * 600).abs_diff(rendered_48k);
        assert!(
            diff <= 600,
            "waveform length mismatch for {}: {total_frames} frames vs {rendered_48k} samples@48k",
            case.id
        );
    }
    eprintln!("dur_hat parity: all cases match reference frames and waveform");
}

#[test]
fn word_onsets_follow_reference_within_50ms() {
    let Some(dir) = common::gated_bundle_dir() else {
        return;
    };
    let native = SileroNative::load(&dir).expect("bundle must load");
    let engine = Engine::load(&dir).expect("bundle must load");

    for case in &fixtures().cases {
        let result = native
            .synthesize(&case.input, &case.speaker, case.sample_rate)
            .unwrap_or_else(|e| panic!("synthesis failed for {}: {e}", case.id));
        let ts = &result.timestamps;
        assert!(!ts.is_empty(), "no timestamps for {}", case.id);

        // Contract invariants on real model output: sorted, non-overlapping,
        // inside [0, duration_sec].
        for pair in ts.windows(2) {
            assert!(
                pair[0].end <= pair[1].start + 1e-3,
                "timestamps overlap for {}: {:?} vs {:?}",
                case.id,
                pair[0],
                pair[1]
            );
        }
        for w in ts {
            assert!(w.start <= w.end, "inverted range for {}: {w:?}", case.id);
            assert!(
                w.end <= result.duration_sec + 1e-3,
                "end past duration for {}: {w:?} (duration {})",
                case.id,
                result.duration_sec
            );
        }

        // Every fixture phrase starts with a letter, so the first word
        // starts right after the sos frames of the reference dur_hat.
        let sos_frames = (case.dur_hat[0] + 0.5).trunc().max(0.0);
        let expected_start = sos_frames * FRAME_SEC;
        let diff = (ts[0].start - expected_start).abs();
        assert!(
            diff <= ONSET_TOL_SEC,
            "first word onset mismatch for {}: got {:.3}s, reference sos implies {:.3}s",
            case.id,
            ts[0].start,
            expected_start
        );

        // Anchor the other end of the timeline too: the last word ends at
        // the reference cumsum up to the last letter symbol (trailing
        // punctuation/eos frames belong to no word). Symbol identity comes
        // from the engine's provenance, frame counts from the fixture, so
        // the reference stays independent of the alignment code.
        let out = engine
            .synthesize(&case.input, &case.speaker, case.sample_rate)
            .unwrap_or_else(|e| panic!("synthesis failed for {}: {e}", case.id));
        let last_letter = out
            .durations
            .iter()
            .rposition(|sd| sd.ch.is_alphabetic())
            .expect("fixture phrases end with a spoken word");
        let ref_end_frames: f32 = case.dur_hat[..=last_letter]
            .iter()
            .map(|d| (d + 0.5).trunc().max(0.0))
            .sum();
        let expected_end = ref_end_frames * FRAME_SEC;
        let last = ts.last().expect("timestamps");
        let diff = (last.end - expected_end).abs();
        assert!(
            diff <= ONSET_TOL_SEC,
            "last word end mismatch for {}: got {:.3}s, reference implies {:.3}s",
            case.id,
            last.end,
            expected_end
        );
    }
    eprintln!("word onsets: all cases within {ONSET_TOL_SEC}s of reference");
}

#[test]
fn highlight_never_jumps_ahead_on_tricky_text() {
    let Some(dir) = common::gated_bundle_dir() else {
        return;
    };
    let native = SileroNative::load(&dir).expect("bundle must load");

    // Regression for the literal-'+' misalignment: a standalone '+' in the
    // text ("правило + команда") used to cascade fourteen words into
    // zero-length timestamps at one instant, so highlighting raced ahead.
    // The second sentence covers the latin-prefix variant: letters the
    // frontend drops ("get_variables" in "get_variablesслэш") must not
    // shift the alignment of the following words either.
    let text = "Тот же паттерн «правило + команда» работает и для документации \
                дизайн-системы. Отдельное правило прямо запрещает агенту \
                выдумывать усе кейс. Он верен только для одного конкретного \
                сценария — когда эй ай получает текстовый промпт. \
                У пенкил эм си пи есть прямые функции get_variablesслэш \
                сет вариаблес - можно буквально попросить агента пройдись \
                по файлу и приведи переменные в соответствие.";
    let result = native.synthesize(text, "aidar", 24000).expect("synthesis");
    let ts = &result.timestamps;
    assert!(!ts.is_empty());

    // Emulate playback highlighting in 150 ms steps, mirroring
    // `findActiveTimestamp` (src/lib/wordHighlight.ts): the active word is
    // the one containing t, or the closest upcoming word in a gap. Speech
    // is slower than 2 words per 150 ms, so a larger advance means broken
    // timestamps.
    let mut prev_idx = 0usize;
    let mut t = 0.0f32;
    while t < result.duration_sec {
        let idx = ts.partition_point(|w| t >= w.end);
        if idx < ts.len() {
            let advance = idx.saturating_sub(prev_idx);
            assert!(
                advance <= 2,
                "highlight jumped {prev_idx} -> {idx} ({:?}) at {t:.2}s",
                ts[idx].word
            );
            prev_idx = prev_idx.max(idx);
        }
        t += 0.15;
    }
    eprintln!("playback emulation: {} words, no jumps", ts.len());
}

#[test]
fn timestamps_are_sample_rate_invariant() {
    let Some(dir) = common::gated_bundle_dir() else {
        return;
    };
    let native = SileroNative::load(&dir).expect("bundle must load");

    let phrases = [
        "Вызови функцию гет юзер дата через эй пи ай.",
        "Ёжик в тумане нашёл ёлку и съел всё.",
        "Стоп! Кто идёт? Отвечай быстро: друг, враг; время — деньги...",
    ];
    for phrase in phrases {
        let base = native
            .synthesize(phrase, "aidar", 48000)
            .expect("48k synthesis")
            .timestamps;
        for rate in [24000, 8000] {
            let ts = native
                .synthesize(phrase, "aidar", rate)
                .unwrap_or_else(|e| panic!("{rate} Hz synthesis failed: {e}"))
                .timestamps;
            assert_eq!(ts.len(), base.len(), "timestamp count differs at {rate} Hz");
            for (a, b) in base.iter().zip(&ts) {
                assert_eq!(a.word, b.word);
                assert_eq!(a.original_pos, b.original_pos);
                // Both sides are ms-rounded from the same frame timeline.
                assert!(
                    (a.start - b.start).abs() <= 0.002,
                    "start differs at {rate} Hz for {:?}: {} vs {}",
                    a.word,
                    a.start,
                    b.start
                );
                assert!(
                    (a.end - b.end).abs() <= 0.002,
                    "end differs at {rate} Hz for {:?}: {} vs {}",
                    a.word,
                    a.end,
                    b.end
                );
            }
        }
    }
    eprintln!("sample-rate invariance: 48000/24000/8000 Hz timestamps agree");
}
