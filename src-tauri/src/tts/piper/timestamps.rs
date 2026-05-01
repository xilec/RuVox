//! Word-level timestamp estimation for Piper output.
//!
//! Piper does not natively emit per-word timing, so we follow the same
//! strategy as `ttsd/ttsd/timestamps.py`: distribute the total audio duration
//! proportionally by word character length.
//!
//! Single-chunk only — Piper synthesizes the whole text in one call. If a
//! future revision chunks long inputs, extend this with a chunked variant.

use regex::Regex;

use crate::tts::{CharMappingEntry, WordTimestamp};

/// Estimate per-word timestamps for `text` over a single audio chunk of length
/// `total_duration_sec`. `char_mapping`, when present, maps normalized-text
/// offsets back to original-text offsets (so the player highlights the right
/// span in the user's input).
///
/// Word offsets are reported in **Unicode codepoint** units (not UTF-8 bytes),
/// matching `CharMapping`/`CharMappingEntry` and the JS frontend (which indexes
/// into UTF-16 strings — equivalent to codepoints for BMP characters like all
/// Cyrillic). `regex::find_iter` returns byte spans, so we convert them in a
/// single forward pass that amortizes the char-counting work.
pub fn estimate_timestamps_single_chunk(
    text: &str,
    total_duration_sec: f64,
    char_mapping: Option<&[CharMappingEntry]>,
) -> Vec<WordTimestamp> {
    // `regex::Regex::new(r"\b\w+\b")` is Unicode-aware by default — `\w`
    // matches Cyrillic and other letter scripts.
    let mut words: Vec<(&str, usize, usize)> = Vec::new();
    let mut prev_byte = 0;
    let mut chars_so_far = 0;
    for m in static_re().find_iter(text) {
        chars_so_far += text[prev_byte..m.start()].chars().count();
        let char_start = chars_so_far;
        let word_chars = m.as_str().chars().count();
        chars_so_far += word_chars;
        let char_end = chars_so_far;
        prev_byte = m.end();
        words.push((m.as_str(), char_start, char_end));
    }

    if words.is_empty() {
        return Vec::new();
    }
    let total_chars: usize = words.iter().map(|(w, _, _)| w.chars().count()).sum();
    if total_chars == 0 {
        return Vec::new();
    }

    let mut current_time = 0.0;
    let mut out = Vec::with_capacity(words.len());

    for (word, norm_start, norm_end) in words {
        let word_chars = word.chars().count();
        let word_duration = (word_chars as f64 / total_chars as f64) * total_duration_sec;

        let original_pos = match char_mapping {
            Some(spans) => map_via_spans(spans, norm_start, norm_end),
            None => (norm_start, norm_end),
        };

        out.push(WordTimestamp {
            word: word.to_string(),
            start: round3(current_time),
            end: round3(current_time + word_duration),
            original_pos,
        });
        current_time += word_duration;
    }

    out
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

fn static_re() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b\w+\b").expect("valid word regex"))
}

/// Find the span(s) covering `[norm_start, norm_end)` and return the smallest
/// interval in original-text coordinates that contains them. Mirrors the
/// `_map_via_spans` helper in `ttsd/timestamps.py`.
fn map_via_spans(spans: &[CharMappingEntry], norm_start: usize, norm_end: usize) -> (usize, usize) {
    let mut best_start: Option<usize> = None;
    let mut best_end: Option<usize> = None;

    for span in spans {
        if span.norm_end <= norm_start {
            continue;
        }
        if span.norm_start >= norm_end {
            break;
        }
        match best_start {
            None => best_start = Some(span.orig_start),
            Some(s) if span.orig_start < s => best_start = Some(span.orig_start),
            _ => {}
        }
        match best_end {
            None => best_end = Some(span.orig_end),
            Some(e) if span.orig_end > e => best_end = Some(span.orig_end),
            _ => {}
        }
    }

    match (best_start, best_end) {
        (Some(s), Some(e)) => (s, e),
        _ => (norm_start, norm_end),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_returns_no_words() {
        let ts = estimate_timestamps_single_chunk("", 1.0, None);
        assert!(ts.is_empty());
    }

    #[test]
    fn punctuation_only_returns_no_words() {
        let ts = estimate_timestamps_single_chunk("..., !!", 1.0, None);
        assert!(ts.is_empty());
    }

    #[test]
    fn russian_words_split_by_whitespace() {
        let ts = estimate_timestamps_single_chunk("привет мир", 2.0, None);
        assert_eq!(ts.len(), 2);
        assert_eq!(ts[0].word, "привет");
        assert_eq!(ts[1].word, "мир");
        // 6 chars + 3 chars = 9 total. привет = 6/9 * 2 = 1.333…
        assert!((ts[0].end - ts[0].start - 1.333).abs() < 0.01);
        assert!((ts[1].end - 2.0).abs() < 0.01);
    }

    #[test]
    fn timestamps_advance_monotonically() {
        let ts = estimate_timestamps_single_chunk("один два три четыре", 4.0, None);
        for i in 1..ts.len() {
            assert!(ts[i].start >= ts[i - 1].end - 1e-6);
        }
    }

    #[test]
    fn char_mapping_maps_norm_to_orig() {
        // "API" (3 chars in original) was normalized to "эй пи ай" — 8 chars
        // in the normalized text. One span covers the whole normalized range
        // and points at the original 3-char word. Both ends are codepoint
        // (char) indices, matching the `CharMapping` contract.
        let spans = vec![CharMappingEntry {
            norm_start: 0,
            norm_end: 8,
            orig_start: 0,
            orig_end: 3,
        }];
        let ts = estimate_timestamps_single_chunk("эй пи ай", 1.0, Some(&spans));
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
        // emitted by `wrapWordsWithOrigPos`, which uses JS string indices
        // (UTF-16 code units = codepoints for BMP characters).
        let ts = estimate_timestamps_single_chunk("привет мир", 1.0, None);
        assert_eq!(ts.len(), 2);
        assert_eq!(ts[0].original_pos, (0, 6));
        assert_eq!(ts[1].original_pos, (7, 10));
    }
}
