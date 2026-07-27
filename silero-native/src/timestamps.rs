//! Word-level timestamps, v1: char-proportional estimation.
//!
//! Semantics ported from `ttsd/ttsd/timestamps.py`: each chunk's duration is
//! distributed across its words proportionally to the word's char count.
//! Precise `dur_hat`-based timestamps are deferred (issue #145).

use serde::{Deserialize, Serialize};

/// Word-level timestamp, mirroring `ttsd.protocol.WordTimestamp`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WordTimestamp {
    pub word: String,
    pub start: f32,
    pub end: f32,
    /// Char offsets of the word in the text passed to `synthesize`
    /// (already the "original" positions at this layer; mapping back through
    /// the normalization pipeline is the caller's job, as in ttsd).
    pub original_pos: (usize, usize),
}

/// `\b\w+\b` equivalent: maximal runs of alphanumeric chars or `_`.
/// Positions are char indices, like Python's `match.start()`.
fn extract_words_with_positions(text: &str) -> Vec<(String, usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let mut words = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_alphanumeric() || chars[i] == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            words.push((chars[start..i].iter().collect(), start, i));
        } else {
            i += 1;
        }
    }
    words
}

/// Round to milliseconds, as ttsd does (`round(x, 3)`).
fn round3(x: f32) -> f32 {
    (x * 1000.0).round() / 1000.0
}

/// Distribute `chunk_duration` across the words of `text`, char-proportionally.
/// Returns an empty vec when the text has no words.
pub fn estimate_timestamps(text: &str, chunk_duration: f32) -> Vec<WordTimestamp> {
    estimate_timestamps_chunked(&[(text, 0, chunk_duration)])
}

/// Chunked variant of [`estimate_timestamps`], porting ttsd's
/// `estimate_timestamps_chunked`: each entry is
/// `(chunk_text, text_offset, chunk_duration)` — the chunk's duration is
/// distributed across its words char-proportionally, `text_offset` shifts
/// `original_pos` back into full-text coordinates, and the audio timeline
/// advances by the accumulated chunk durations.
pub fn estimate_timestamps_chunked(chunks: &[(&str, usize, f32)]) -> Vec<WordTimestamp> {
    let mut timestamps = Vec::new();
    let mut audio_offset = 0.0f32;
    for &(chunk_text, text_offset, chunk_duration) in chunks {
        let words = extract_words_with_positions(chunk_text);
        let total_chars: usize = words.iter().map(|(w, _, _)| w.chars().count()).sum();
        if total_chars == 0 || chunk_duration <= 0.0 {
            audio_offset += chunk_duration.max(0.0);
            continue;
        }
        let mut current = 0.0f32;
        for (word, start, end) in words {
            let word_duration = (word.chars().count() as f32 / total_chars as f32) * chunk_duration;
            timestamps.push(WordTimestamp {
                word,
                start: round3(audio_offset + current),
                end: round3(audio_offset + current + word_duration),
                original_pos: (text_offset + start, text_offset + end),
            });
            current += word_duration;
        }
        audio_offset += chunk_duration;
    }
    timestamps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_split_on_punctuation() {
        let words = extract_words_with_positions("привет, мир! get_user");
        let texts: Vec<&str> = words.iter().map(|(w, _, _)| w.as_str()).collect();
        assert_eq!(texts, vec!["привет", "мир", "get_user"]);
        assert_eq!(words[0].1..words[0].2, 0..6);
    }

    #[test]
    fn durations_are_proportional_and_monotonic() {
        let ts = estimate_timestamps("аа бббб в", 1.0);
        assert_eq!(ts.len(), 3);
        assert!((ts[0].end - ts[0].start - 2.0 / 7.0).abs() < 1e-3);
        assert!((ts[1].end - ts[1].start - 4.0 / 7.0).abs() < 1e-3);
        for pair in ts.windows(2) {
            assert!(pair[0].end <= pair[1].start + 1e-3);
        }
        assert!((ts.last().map(|t| t.end).unwrap_or(0.0) - 1.0).abs() < 2e-3);
    }

    #[test]
    fn no_words_no_timestamps() {
        assert!(estimate_timestamps("… — !", 1.0).is_empty());
        assert!(estimate_timestamps("текст", 0.0).is_empty());
    }

    #[test]
    fn chunked_shifts_time_and_positions() {
        let ts = estimate_timestamps_chunked(&[("аа бб", 0, 1.0), ("вв гг", 100, 2.0)]);
        assert_eq!(ts.len(), 4);
        // Second chunk starts where the first ended.
        assert!((ts[2].start - 1.0).abs() < 2e-3);
        assert!((ts[3].end - 3.0).abs() < 2e-3);
        // Positions are shifted into full-text coordinates.
        assert_eq!(ts[2].original_pos, (100, 102));
        assert_eq!(ts[3].original_pos, (103, 105));
        for pair in ts.windows(2) {
            assert!(pair[0].end <= pair[1].start + 1e-3);
        }
    }

    #[test]
    fn chunked_skips_wordless_chunks_but_keeps_the_timeline() {
        let ts = estimate_timestamps_chunked(&[("… !", 0, 1.5), ("слова", 10, 1.0)]);
        assert_eq!(ts.len(), 1);
        assert!((ts[0].start - 1.5).abs() < 1e-3);
        assert_eq!(ts[0].original_pos, (10, 15));
    }
}
