//! Approximate word timestamps.
//!
//! Every chunk is sentence-bounded, so its audio range is exact. Inside a
//! chunk, word boundaries come from running the duration predictor on
//! cumulative word prefixes and scaling the predicted timeline to the actual
//! chunk duration. Pauses are attributed to the preceding word, so a boundary
//! may lag an ASR reference by 0.2–0.4 s — acceptable for a reading aid, and
//! documented in the capability spec.

use crate::error::{Error, Result};

/// One highlighted word.
#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    /// The word as it appears in normalized text (punctuation attached).
    pub word: String,
    /// Start in seconds from the beginning of the whole utterance.
    pub start: f64,
    /// End in seconds.
    pub end: f64,
    /// Codepoint range in normalized text.
    pub start_char: usize,
    pub end_char: usize,
}

/// Threshold below which the leading samples are treated as silence.
const SILENCE_THRESHOLD: f32 = 1e-3;

/// Split text into whitespace-delimited words with codepoint ranges.
pub fn words(text: &str) -> Vec<(String, usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let mut words = Vec::new();
    let mut start: Option<usize> = None;
    for (index, character) in chars.iter().enumerate() {
        if character.is_whitespace() {
            if let Some(from) = start.take() {
                words.push((chars[from..index].iter().collect(), from, index));
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(from) = start {
        words.push((chars[from..].iter().collect(), from, chars.len()));
    }
    words
}

/// Seconds of leading silence, measured as the first sample above the
/// threshold.
pub fn leading_silence(samples: &[f32]) -> f64 {
    let index = samples
        .iter()
        .position(|sample| sample.abs() > SILENCE_THRESHOLD)
        .unwrap_or(0);
    index as f64 / crate::synth::SAMPLE_RATE as f64
}

/// Build timestamps for one chunk.
///
/// Where a chunk sits in the utterance and in the audio timeline.
#[derive(Debug, Clone, Copy)]
pub struct ChunkTiming {
    /// Codepoint offset of the chunk in the normalized text.
    pub chunk_start: usize,
    /// Audio start of the chunk, seconds from the beginning of the utterance.
    pub audio_start: f64,
    /// Audio end of the chunk, seconds.
    pub audio_end: f64,
    /// Leading silence inside the chunk's audio, seconds.
    pub leading_silence: f64,
}

/// `original` is the pipeline text of the chunk (used for the highlighted word
/// strings and char ranges), `prepared_inner` is the same chunk after tags,
/// stress and NFKD (used for the predictor prefixes, which must match what the
/// model saw), `dropped` lists the characters the alphabet gate removed,
/// `timing` places the chunk in the text and audio, and `predict` returns raw
/// predicted seconds for a prefix.
///
/// Tokens made only of dropped characters (a standalone em dash, a number when
/// the caller bypassed the pipeline) disappear from the prepared text, so they
/// are filtered out of the source list before the counts are compared; keeping
/// them would reject the whole chunk for a cosmetic difference.
pub fn chunk_timestamps(
    original: &str,
    prepared_inner: &str,
    dropped: &[char],
    timing: ChunkTiming,
    predict: impl Fn(&str) -> Result<f64>,
) -> Result<Vec<Word>> {
    let ChunkTiming {
        chunk_start,
        audio_start,
        audio_end,
        leading_silence,
    } = timing;
    let source: Vec<_> = words(original)
        .into_iter()
        .filter(|(word, _, _)| !word.chars().all(|character| dropped.contains(&character)))
        .collect();
    let prepared = words(prepared_inner);
    if source.is_empty() {
        return Ok(Vec::new());
    }
    if source.len() != prepared.len() {
        return Err(Error::Synthesis(format!(
            "word count changed during preparation: {} -> {}",
            source.len(),
            prepared.len()
        )));
    }

    let prepared_chars: Vec<char> = prepared_inner.chars().collect();
    let mut predicted = Vec::with_capacity(prepared.len());
    for (_, _, end) in &prepared {
        let prefix: String = prepared_chars[..*end].iter().collect();
        predicted.push(predict(&prefix)?);
    }
    let total = *predicted.last().unwrap_or(&0.0);
    if !total.is_finite() || total <= 0.0 {
        return Err(Error::Synthesis(format!(
            "prefix predictor returned {total}"
        )));
    }

    let speech = (audio_end - audio_start - leading_silence).max(0.001);
    let scale = speech / total;
    let mut result = Vec::with_capacity(source.len());
    let mut previous_end = audio_start + leading_silence;
    for (index, (word, start_char, end_char)) in source.into_iter().enumerate() {
        let end = (audio_start + leading_silence + predicted[index] * scale).min(audio_end);
        let start = previous_end.min(audio_end);
        let end = if end > start { end } else { start };
        result.push(Word {
            word,
            start,
            end,
            start_char: chunk_start + start_char,
            end_char: chunk_start + end_char,
        });
        previous_end = end;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_predict(prefix: &str) -> Result<f64> {
        Ok(prefix.chars().count() as f64 / 10.0)
    }

    #[test]
    fn splits_words_with_char_ranges() {
        let words = words("привет, мир!");
        assert_eq!(words.len(), 2);
        assert_eq!(words[0], ("привет,".to_string(), 0, 7));
        assert_eq!(words[1], ("мир!".to_string(), 8, 12));
    }

    /// A token made only of characters the alphabet gate dropped (a standalone
    /// em dash, a number when the caller bypassed the pipeline) must not fail
    /// the chunk: it has no counterpart in the prepared text by design.
    #[test]
    fn tokens_dropped_by_the_alphabet_gate_are_skipped() {
        let original = "цена — 15 рублей";
        let prepared = "цен+а  рубл+ей";
        let words = chunk_timestamps(
            original,
            prepared,
            &['—', '1', '5'],
            ChunkTiming {
                chunk_start: 0,
                audio_start: 0.0,
                audio_end: 2.0,
                leading_silence: 0.0,
            },
            fake_predict,
        )
        .unwrap();
        let words: Vec<&str> = words.iter().map(|w| w.word.as_str()).collect();
        assert_eq!(words, vec!["цена", "рублей"]);
    }

    #[test]
    fn timestamps_are_monotonic_and_inside_the_audio() {
        let original = "первое предложение здесь. второе предложение тут.";
        let prepared = original;
        let words = chunk_timestamps(
            original,
            prepared,
            &[],
            ChunkTiming {
                chunk_start: 0,
                audio_start: 0.0,
                audio_end: 4.0,
                leading_silence: 0.1,
            },
            fake_predict,
        )
        .unwrap();
        assert_eq!(words.len(), 6);
        let mut previous = 0.0;
        for word in &words {
            assert!(word.start >= previous - 1e-9);
            assert!(word.end >= word.start);
            assert!(word.end <= 4.0 + 1e-9);
            previous = word.end;
        }
        assert!((words.last().unwrap().end - 4.0).abs() < 1e-6);
    }

    #[test]
    fn char_ranges_are_shifted_by_the_chunk_offset() {
        let words = chunk_timestamps(
            "мир",
            "мир",
            &[],
            ChunkTiming {
                chunk_start: 100,
                audio_start: 0.0,
                audio_end: 1.0,
                leading_silence: 0.0,
            },
            fake_predict,
        )
        .unwrap();
        assert_eq!(words[0].start_char, 100);
        assert_eq!(words[0].end_char, 103);
    }

    #[test]
    fn leading_silence_is_measured() {
        let mut samples = vec![0.0f32; 4410];
        samples.push(0.5);
        let silence = leading_silence(&samples);
        assert!((silence - 0.1).abs() < 1e-6);
    }

    #[test]
    fn a_word_count_mismatch_is_an_error() {
        assert!(
            chunk_timestamps(
                "два слова",
                "одно",
                &[],
                ChunkTiming {
                    chunk_start: 0,
                    audio_start: 0.0,
                    audio_end: 1.0,
                    leading_silence: 0.0
                },
                fake_predict
            )
            .is_err()
        );
    }
}
