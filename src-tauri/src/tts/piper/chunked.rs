//! Chunked synthesis driver for the Piper engine.
//!
//! VITS encoder activation memory grows quadratically with input length, so a
//! single `Piper::create` call over long text can request multi-GB tensors and
//! freeze the machine (#155). [`synthesize_chunked`] splits the normalized
//! text (via [`crate::tts::chunking`]) and drives the caller's synthesis
//! closure once per chunk; the closure synthesizes AND persists the chunk
//! (the engine streams samples into the WAV file instead of accumulating the
//! whole text in memory), and this module records each chunk's
//! `(norm_start, norm_end, duration_sec)` for the chunked timestamp estimator.
//!
//! A paragraph break between two chunks (blank line in the source) earns a
//! fixed silence pause before the second chunk — the model itself collapses
//! `\n\n` into a plain space, so without the inserted pause paragraphs would
//! be unreadable as paragraphs. The pause is folded into the preceding
//! chunk's duration, keeping the timeline and the WAV aligned.
//!
//! The closure is injected so the chunk/cancel/pause logic is unit-testable
//! without a real ONNX model.

use std::sync::atomic::{AtomicBool, Ordering};

use super::super::chunking::split_with_limit;
use crate::tts::TtsError;

/// Silence inserted before a chunk that follows a paragraph break (blank
/// line) in the source text, in seconds.
pub const PARAGRAPH_PAUSE_SEC: f64 = 0.45;

/// Per-chunk timeline: `(norm_start, norm_end, duration_sec)` in order —
/// codepoint offsets into the full normalized text, matching the ttsd
/// `estimate_timestamps_chunked` contract. Durations include the paragraph
/// pause that followed each chunk, so their sum equals the WAV length.
#[derive(Debug, Default)]
pub struct ChunkedTimeline {
    pub chunk_durations: Vec<(usize, usize, f64)>,
}

impl ChunkedTimeline {
    /// Total audio duration: chunk audio plus the pauses folded into it.
    pub fn total_duration_sec(&self) -> f64 {
        self.chunk_durations.iter().map(|(_, _, d)| d).sum()
    }
}

/// Piper cancellation error, surfaced when the cancel flag is observed
/// between chunks. The engine's `kill_current` sets the flag; the abort path
/// in `commands` may also have dropped the async task already — in that case
/// this error is simply discarded with the task.
pub fn cancelled_error() -> TtsError {
    TtsError::Ttsd {
        code: "piper_cancelled".to_string(),
        message: "Синтез отменён".to_string(),
    }
}

/// Synthesize `text` in chunks of at most `limit` codepoints, checking
/// `cancelled` before each chunk inference and again after the last one (so a
/// cancel arriving during a final single chunk still discards the result).
/// A chunk inference failure aborts the whole synthesis.
///
/// `synth` receives the chunk text and the pause (seconds of silence) it must
/// write *before* the chunk's own audio (non-zero only right after a
/// paragraph break), and returns the chunk's own audio duration.
pub fn synthesize_chunked(
    text: &str,
    limit: usize,
    cancelled: &AtomicBool,
    mut synth: impl FnMut(&str, f64) -> Result<f64, TtsError>,
) -> Result<ChunkedTimeline, TtsError> {
    let mut out = ChunkedTimeline::default();
    let mut prev_content_end: Option<usize> = None;
    for (chunk_text, chunk_start) in split_with_limit(text, limit) {
        if cancelled.load(Ordering::Relaxed) {
            return Err(cancelled_error());
        }
        let pause = match prev_content_end {
            None => 0.0,
            Some(prev_end) if is_paragraph_break(text, prev_end, chunk_start) => {
                PARAGRAPH_PAUSE_SEC
            }
            Some(_) => 0.0,
        };
        if let Some(last) = out.chunk_durations.last_mut().filter(|_| pause > 0.0) {
            // The pause sits between the previous chunk's audio and this
            // chunk's; folding it into the previous duration keeps the
            // timeline (and the WAV) aligned for the timestamp estimator.
            last.2 += pause;
        }
        let duration_sec = synth(&chunk_text, pause)?;
        let norm_end = chunk_start + chunk_text.chars().count();
        out.chunk_durations
            .push((chunk_start, norm_end, duration_sec));
        prev_content_end = Some(norm_end);
    }
    if cancelled.load(Ordering::Relaxed) {
        return Err(cancelled_error());
    }
    Ok(out)
}

/// Whether the whitespace between two consecutive chunks contains a blank
/// line (a paragraph break). The chunker guarantees this gap is
/// whitespace-only.
fn is_paragraph_break(text: &str, from: usize, to: usize) -> bool {
    if to <= from {
        return false;
    }
    let gap: String = text.chars().skip(from).take(to - from).collect();
    let mut newline_run = 0usize;
    for c in gap.chars() {
        if c == '\n' || c == '\r' {
            newline_run += 1;
            if newline_run >= 2 {
                return true;
            }
        } else if c.is_whitespace() {
            continue;
        } else {
            newline_run = 0;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fake synthesizer: reports each chunk's char count as its duration (at
    /// 100 Hz) and can fail on a chosen chunk.
    struct FakeSynth {
        fail_on_chunk: Option<usize>,
        calls: Vec<(String, f64)>,
    }

    impl FakeSynth {
        fn new() -> Self {
            Self {
                fail_on_chunk: None,
                calls: Vec::new(),
            }
        }

        fn closure(&mut self) -> impl FnMut(&str, f64) -> Result<f64, TtsError> + '_ {
            move |chunk: &str, pause: f64| {
                if Some(self.calls.len()) == self.fail_on_chunk {
                    self.calls.push((chunk.to_string(), pause));
                    return Err(TtsError::Ttsd {
                        code: "piper_synthesis_failed".to_string(),
                        message: "boom".to_string(),
                    });
                }
                self.calls.push((chunk.to_string(), pause));
                let dur = chunk.chars().count() as f64 / 100.0;
                Ok(dur)
            }
        }
    }

    #[test]
    fn short_text_is_a_single_call_without_pause() {
        let cancelled = AtomicBool::new(false);
        let mut fake = FakeSynth::new();
        let out = synthesize_chunked("привет мир", 600, &cancelled, fake.closure()).expect("ok");
        assert_eq!(fake.calls, vec![("привет мир".to_string(), 0.0)]);
        assert_eq!(out.chunk_durations, vec![(0, 10, 0.1)]);
    }

    #[test]
    fn long_text_splits_in_order() {
        let cancelled = AtomicBool::new(false);
        let text = "Первое предложение. Второе предложение. Третье предложение.";
        let mut fake = FakeSynth::new();
        let out = synthesize_chunked(text, 25, &cancelled, fake.closure()).expect("ok");
        assert!(fake.calls.len() > 1, "must be chunked: {:?}", fake.calls);
        let starts: Vec<usize> = out.chunk_durations.iter().map(|(s, _, _)| *s).collect();
        assert!(starts.windows(2).all(|w| w[0] < w[1]));
        // Without paragraph breaks no pause is ever requested, so durations
        // are the chunks' own audio only.
        for (chunk_text, pause) in &fake.calls {
            assert_eq!(*pause, 0.0, "unexpected pause before {chunk_text:?}");
        }
    }

    #[test]
    fn paragraph_break_gets_a_pause_folded_into_the_previous_chunk() {
        let cancelled = AtomicBool::new(false);
        let text = "Первая часть.\n\nВторая часть.";
        let mut fake = FakeSynth::new();
        let out = synthesize_chunked(text, 300, &cancelled, fake.closure()).expect("ok");
        assert_eq!(fake.calls.len(), 2, "paragraph break forces a chunk split");
        assert_eq!(fake.calls[0].1, 0.0, "no pause before the first chunk");
        assert_eq!(
            fake.calls[1].1, PARAGRAPH_PAUSE_SEC,
            "pause before the chunk after a blank line"
        );
        // First chunk's duration carries the pause; the second is its own.
        assert_eq!(out.chunk_durations[0].2, 13.0 / 100.0 + PARAGRAPH_PAUSE_SEC);
        assert_eq!(out.chunk_durations[1].2, 13.0 / 100.0);
    }

    #[test]
    fn sentence_boundary_gets_no_inserted_pause() {
        let cancelled = AtomicBool::new(false);
        let text = "Первое предложение. Второе предложение. Третье предложение.";
        let mut fake = FakeSynth::new();
        synthesize_chunked(text, 25, &cancelled, fake.closure()).expect("ok");
        assert!(fake.calls.len() > 1);
        assert!(
            fake.calls.iter().all(|(_, pause)| *pause == 0.0),
            "plain whitespace between chunks must not pause: {:?}",
            fake.calls
        );
    }

    #[test]
    fn cancel_before_first_chunk_produces_cancelled_error() {
        let cancelled = AtomicBool::new(true);
        let mut fake = FakeSynth::new();
        let err = synthesize_chunked("текст", 600, &cancelled, fake.closure()).unwrap_err();
        match err {
            TtsError::Ttsd { code, message } => {
                assert_eq!(code, "piper_cancelled");
                assert!(
                    message.chars().any(|c| matches!(c, 'А'..='я' | 'ё')),
                    "user-facing message must be Russian: {message}"
                );
            }
            other => panic!("expected piper_cancelled, got {other:?}"),
        }
        assert!(fake.calls.is_empty(), "no inference may run after cancel");
    }

    #[test]
    fn cancel_during_chunk_stops_before_the_next_one() {
        let cancelled = AtomicBool::new(false);
        let text = "Первое предложение. Второе предложение. Третье предложение.";
        let mut fake = FakeSynth::new();
        let mut observe = |chunk: &str, pause: f64| {
            let r = Ok(chunk.chars().count() as f64 / 100.0);
            if fake.calls.is_empty() {
                cancelled.store(true, Ordering::Relaxed);
            }
            fake.calls.push((chunk.to_string(), pause));
            r
        };
        let err = synthesize_chunked(text, 25, &cancelled, &mut observe).unwrap_err();
        assert!(matches!(err, TtsError::Ttsd { code, .. } if code == "piper_cancelled"));
        assert_eq!(
            fake.calls.len(),
            1,
            "no chunk after the one where cancel was observed"
        );
    }

    #[test]
    fn chunk_failure_fails_the_whole_synthesis() {
        let cancelled = AtomicBool::new(false);
        let text = "Первое предложение. Второе предложение. Третье предложение.";
        let mut fake = FakeSynth::new();
        fake.fail_on_chunk = Some(1);
        let err = synthesize_chunked(text, 25, &cancelled, fake.closure()).unwrap_err();
        assert!(matches!(err, TtsError::Ttsd { code, .. } if code == "piper_synthesis_failed"));
        assert_eq!(fake.calls.len(), 2, "failed chunk aborts the loop");
    }

    #[test]
    fn cancel_after_last_chunk_discards_samples() {
        // Single-chunk text: the cancel arrives while the only chunk is being
        // synthesized. The post-loop check must still discard the audio.
        let cancelled = AtomicBool::new(false);
        let mut observe = |chunk: &str, _pause: f64| {
            let r = Ok(chunk.chars().count() as f64 / 100.0);
            cancelled.store(true, Ordering::Relaxed);
            r
        };
        let err = synthesize_chunked("короткий текст", 600, &cancelled, &mut observe).unwrap_err();
        assert!(matches!(err, TtsError::Ttsd { code, .. } if code == "piper_cancelled"));
    }
}
