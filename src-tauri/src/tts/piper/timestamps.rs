//! Word-level timestamp estimation for Piper output.
//!
//! Piper does not natively emit per-word timing, so we follow the same
//! strategy as `ttsd/ttsd/timestamps.py`: distribute each chunk's audio
//! duration proportionally by word character length, shifting by the
//! accumulated duration of preceding chunks (synthesis runs in chunks — see
//! `piper/chunked.rs`).
//!
//! Word offsets are reported in **Unicode codepoint** units (not UTF-8 bytes),
//! matching `CharMapping`/`CharMappingEntry` and the JS frontend (which indexes
//! into UTF-16 strings — equivalent to codepoints for BMP characters like all
//! Cyrillic). `regex::find_iter` returns byte spans, so we convert them in a
//! single forward pass that amortizes the char-counting work.

use regex::Regex;

use crate::tts::{CharMappingEntry, WordTimestamp, map_via_spans};

/// Estimate per-word timestamps for chunked synthesis output.
///
/// `chunk_durations` carries `(norm_start, norm_end, duration_sec)` per chunk
/// — codepoint offsets into the full normalized `text` that was synthesized,
/// in chunk order. Words in each chunk are distributed across that chunk's
/// duration; `char_mapping`, when present, maps normalized-text offsets back
/// to original-text offsets (so the player highlights the right span in the
/// user's input).
pub fn estimate_timestamps_chunked(
    text: &str,
    chunk_durations: &[(usize, usize, f64)],
    char_mapping: Option<&[CharMappingEntry]>,
) -> Vec<WordTimestamp> {
    let mut out = Vec::new();
    let mut audio_offset = 0.0;
    for (chunk_start, chunk_end, chunk_duration) in chunk_durations {
        let chunk_text = char_slice(text, *chunk_start, *chunk_end);
        distribute_chunk_words(
            &chunk_text,
            *chunk_start,
            *chunk_duration,
            audio_offset,
            char_mapping,
            &mut out,
        );
        audio_offset += chunk_duration;
    }
    out
}

/// Distribute `chunk_duration` across the words of `chunk_text`
/// proportionally by codepoint length, appending each word at
/// `audio_offset + within-chunk time`. `base_offset` is the chunk's codepoint
/// position in the full normalized text; a chunk with no words (pure
/// punctuation/whitespace) still advances the timeline by its full duration.
fn distribute_chunk_words(
    chunk_text: &str,
    base_offset: usize,
    chunk_duration: f64,
    audio_offset: f64,
    char_mapping: Option<&[CharMappingEntry]>,
    out: &mut Vec<WordTimestamp>,
) {
    let mut words: Vec<(&str, usize, usize)> = Vec::new();
    let mut prev_byte = 0;
    let mut chars_so_far = 0;
    for m in static_re().find_iter(chunk_text) {
        chars_so_far += chunk_text[prev_byte..m.start()].chars().count();
        let local_start = chars_so_far;
        let word_chars = m.as_str().chars().count();
        chars_so_far += word_chars;
        let local_end = chars_so_far;
        prev_byte = m.end();
        words.push((m.as_str(), local_start, local_end));
    }

    let total_chars: usize = words.iter().map(|(w, _, _)| w.chars().count()).sum();
    if total_chars == 0 {
        return;
    }

    let mut current_time = 0.0;
    for (word, local_start, local_end) in words {
        let word_chars = word.chars().count();
        let word_duration = (word_chars as f64 / total_chars as f64) * chunk_duration;

        let norm_start = base_offset + local_start;
        let norm_end = base_offset + local_end;
        let original_pos = match char_mapping {
            Some(spans) => map_via_spans(spans, norm_start, norm_end),
            None => (norm_start, norm_end),
        };

        out.push(WordTimestamp {
            word: word.to_string(),
            start: round3(audio_offset + current_time),
            end: round3(audio_offset + current_time + word_duration),
            original_pos,
        });
        current_time += word_duration;
    }
}

/// Codepoint-range slice of `text` (`start` inclusive, `end` exclusive).
fn char_slice(text: &str, start: usize, end: usize) -> String {
    text.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

fn static_re() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    // `\b\w+\b` is Unicode-aware by default — `\w` matches Cyrillic and other
    // letter scripts.
    RE.get_or_init(|| Regex::new(r"\b\w+\b").expect("valid word regex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_chunks_return_no_words() {
        let ts = estimate_timestamps_chunked("", &[], None);
        assert!(ts.is_empty());
    }

    #[test]
    fn punctuation_only_chunk_advances_audio_offset() {
        // The first chunk holds no words; the second chunk's words must start
        // after the first chunk's full duration, not at zero.
        let text = "... !! привет мир";
        let chunks = vec![(0, 6, 1.5), (6, 16, 2.0)];
        let ts = estimate_timestamps_chunked(text, &chunks, None);
        assert_eq!(ts.len(), 2);
        assert!((ts[0].start - 1.5).abs() < 0.01);
        assert!(ts.iter().all(|w| w.start >= 1.5));
    }

    #[test]
    fn timestamps_advance_monotonically_across_chunk_boundaries() {
        let text = "один два три четыре пять шесть";
        // Split mid-text: chunks (0, 8) and (9, 30) in codepoint offsets.
        let chunks = vec![(0, 8, 2.0), (9, 30, 3.0)];
        let ts = estimate_timestamps_chunked(text, &chunks, None);
        assert_eq!(ts.len(), 6);
        for i in 1..ts.len() {
            assert!(ts[i].start >= ts[i - 1].end - 1e-6);
        }
        // First word after the boundary starts at or after chunk one's
        // accumulated duration.
        assert!(ts[2].start >= 2.0 - 1e-6);
    }

    #[test]
    fn char_mapping_maps_norm_to_orig() {
        // "API" (3 chars in original) was normalized to "эй пи ай" — 8 chars
        // in the normalized text. One span covers the whole normalized range
        // and points at the original 3-char word.
        let spans = vec![CharMappingEntry {
            norm_start: 0,
            norm_end: 8,
            orig_start: 0,
            orig_end: 3,
        }];
        let ts = estimate_timestamps_chunked("эй пи ай", &[(0, 8, 1.0)], Some(&spans));
        assert_eq!(ts.len(), 3);
        for w in &ts {
            assert_eq!(w.original_pos, (0, 3));
        }
    }

    #[test]
    fn original_pos_is_codepoint_indexed_for_cyrillic() {
        // Without char_mapping, original_pos falls back to the normalized
        // offsets. Those must be codepoint indices, not UTF-8 byte offsets —
        // the JS frontend matches them against `data-orig-start` attributes
        // emitted by `wrapWordsWithOrigPos`, which uses JS string indices.
        let ts = estimate_timestamps_chunked("привет мир", &[(0, 10, 1.0)], None);
        assert_eq!(ts.len(), 2);
        assert_eq!(ts[0].original_pos, (0, 6));
        assert_eq!(ts[1].original_pos, (7, 10));
    }

    #[test]
    fn second_chunk_offsets_are_full_text_codepoints() {
        // Cyrillic chars are 2 bytes each — offsets into the full text must
        // still be codepoint indices, so the chunk base offset must not
        // inherit any byte counting.
        let text = "первая часть, вторая часть";
        let chunks = vec![(0, 13, 1.0), (14, 26, 1.0)];
        let ts = estimate_timestamps_chunked(text, &chunks, None);
        let вторая: Vec<&WordTimestamp> = ts.iter().filter(|w| w.word == "вторая").collect();
        assert_eq!(вторая.len(), 1);
        assert_eq!(вторая[0].original_pos, (14, 20));
    }
}
