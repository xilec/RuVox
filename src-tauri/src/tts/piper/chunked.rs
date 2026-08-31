//! Chunked synthesis driver for the Piper engine.
//!
//! VITS encoder activation memory grows quadratically with input length, so a
//! single `Piper::create` call over long text can request multi-GB tensors and
//! freeze the machine (#155). [`synthesize_chunked`] splits the normalized
//! text (via [`crate::tts::chunking`]), runs the caller's synthesis closure
//! once per chunk, concatenates the samples, and records each chunk's
//! `(norm_start, norm_end, duration_sec)` for the chunked timestamp
//! estimator.
//!
//! The closure is injected so the chunk/cancel/concatenation logic is
//! unit-testable without a real ONNX model.

use std::sync::atomic::{AtomicBool, Ordering};

use super::super::chunking::split_with_limit;
use crate::tts::TtsError;

/// Chunks per synthesis, in order: accumulated `(norm_start, norm_end,
/// duration_sec)` — codepoint offsets into the full normalized text, matching
/// the ttsd `estimate_timestamps_chunked` contract.
#[derive(Debug, Default)]
pub struct ChunkedSynthesis {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub chunk_durations: Vec<(usize, usize, f64)>,
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
/// A chunk inference failure aborts the whole synthesis; no partial audio is
/// returned.
pub fn synthesize_chunked(
    text: &str,
    limit: usize,
    cancelled: &AtomicBool,
    mut synth: impl FnMut(&str) -> Result<(Vec<f32>, u32), TtsError>,
) -> Result<ChunkedSynthesis, TtsError> {
    let mut out = ChunkedSynthesis::default();
    for (chunk_text, chunk_start) in split_with_limit(text, limit) {
        if cancelled.load(Ordering::Relaxed) {
            return Err(cancelled_error());
        }
        let (mut samples, sample_rate) = synth(&chunk_text)?;
        if out.chunk_durations.is_empty() {
            out.sample_rate = sample_rate;
        } else {
            // Every chunk of one voice runs through the same model config, so
            // the rates must agree; a mismatch would mean the model was
            // swapped mid-synthesis, which the engine's lock prevents.
            debug_assert_eq!(out.sample_rate, sample_rate);
        }
        let duration_sec = if sample_rate == 0 {
            0.0
        } else {
            samples.len() as f64 / sample_rate as f64
        };
        let norm_end = chunk_start + chunk_text.chars().count();
        out.samples.append(&mut samples);
        out.chunk_durations
            .push((chunk_start, norm_end, duration_sec));
    }
    if cancelled.load(Ordering::Relaxed) {
        return Err(cancelled_error());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fake synthesizer: emits one sample per char at a fixed rate, and can
    /// fail or observe the cancel flag on a chosen chunk.
    struct FakeSynth {
        sample_rate: u32,
        fail_on_chunk: Option<usize>,
        calls: Vec<String>,
    }

    impl FakeSynth {
        fn new(sample_rate: u32) -> Self {
            Self {
                sample_rate,
                fail_on_chunk: None,
                calls: Vec::new(),
            }
        }

        fn closure(&mut self) -> impl FnMut(&str) -> Result<(Vec<f32>, u32), TtsError> + '_ {
            move |chunk: &str| {
                if Some(self.calls.len()) == self.fail_on_chunk {
                    self.calls.push(chunk.to_string());
                    return Err(TtsError::Ttsd {
                        code: "piper_synthesis_failed".to_string(),
                        message: "boom".to_string(),
                    });
                }
                self.calls.push(chunk.to_string());
                let samples = vec![0.5f32; chunk.chars().count()];
                Ok((samples, self.sample_rate))
            }
        }
    }

    #[test]
    fn short_text_is_a_single_call() {
        let cancelled = AtomicBool::new(false);
        let mut fake = FakeSynth::new(22_050);
        let out = synthesize_chunked("привет мир", 600, &cancelled, fake.closure()).expect("ok");
        assert_eq!(fake.calls, vec!["привет мир"]);
        assert_eq!(out.samples.len(), 10);
        assert_eq!(out.sample_rate, 22_050);
        assert_eq!(out.chunk_durations, vec![(0, 10, 10.0 / 22_050.0)]);
    }

    #[test]
    fn long_text_concatenates_chunks_in_order() {
        let cancelled = AtomicBool::new(false);
        let text = "Первое предложение. Второе предложение. Третье предложение.";
        let mut fake = FakeSynth::new(100);
        let out = synthesize_chunked(text, 25, &cancelled, fake.closure()).expect("ok");
        assert!(fake.calls.len() > 1, "must be chunked: {:?}", fake.calls);
        // Every chunk call got the chunk text verbatim; samples concatenated.
        let total_chars: usize = fake.calls.iter().map(|c| c.chars().count()).sum();
        assert_eq!(out.samples.len(), total_chars);
        // Chunk durations are ordered and non-decreasing in start offsets.
        let starts: Vec<usize> = out.chunk_durations.iter().map(|(s, _, _)| *s).collect();
        assert!(starts.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn cancel_before_first_chunk_produces_cancelled_error() {
        let cancelled = AtomicBool::new(true);
        let mut fake = FakeSynth::new(100);
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
        let mut fake = FakeSynth::new(100);
        let mut first_call = {
            |chunk: &str| {
                if fake.calls.is_empty() {
                    cancelled.store(true, Ordering::Relaxed);
                }
                fake.calls.push(chunk.to_string());
                Ok((vec![0.5f32; chunk.chars().count()], 100))
            }
        };
        let err = synthesize_chunked(text, 25, &cancelled, &mut first_call).unwrap_err();
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
        let mut fake = FakeSynth::new(100);
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
        let mut observe = |chunk: &str| {
            let r = Ok((vec![0.5f32; chunk.chars().count()], 100));
            cancelled.store(true, Ordering::Relaxed);
            r
        };
        let err = synthesize_chunked("короткий текст", 600, &cancelled, &mut observe).unwrap_err();
        assert!(matches!(err, TtsError::Ttsd { code, .. } if code == "piper_cancelled"));
    }
}
