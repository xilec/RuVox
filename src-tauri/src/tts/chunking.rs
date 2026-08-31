//! Long-text chunking for synthesis, ported from `ttsd/ttsd/chunking.py` (the
//! same logic the `silero-native` crate carries for Silero).
//!
//! Both in-process engines must synthesize long text in bounded chunks: model
//! encoders degrade or blow up memory on very long inputs (Piper's VITS
//! encoder grows activation memory quadratically — #155). This is a
//! Piper-owned copy rather than a dependency on the `silero-native` crate's
//! internals: that crate is a standalone engine whose chunker is documented
//! in Silero terms, and cross-engine coupling is worse than the small
//! duplication.
//!
//! [`split_with_limit`] returns `(chunk_text, char_offset)` pairs where
//! `char_offset` is the chunk's codepoint position in `text` — the same
//! contract ttsd's timestamp estimator consumes.

/// End of the whitespace run starting at `from` (`from` itself may be
/// non-whitespace — then the run is empty and `from` is returned).
fn whitespace_run_end(window: &[char], from: usize) -> usize {
    let mut end = from;
    while end < window.len() && window[end].is_whitespace() {
        end += 1;
    }
    end
}

/// End offset (within `window`) of the last `punct + whitespace` break, like
/// the last `re.finditer(r"[.!?]\s+", ...)` match. An empty `punct` set means
/// a bare whitespace run (`\s+`).
fn last_break_after(window: &[char], punct: &[char]) -> Option<usize> {
    let mut best = None;
    for (i, &c) in window.iter().enumerate() {
        if punct.is_empty() {
            // `\s+`: a whitespace run matches on its own.
            if c.is_whitespace() {
                best = Some(whitespace_run_end(window, i));
            }
        } else if punct.contains(&c) {
            // `[.!?]\s+`: the punctuation must be followed by whitespace.
            let end = whitespace_run_end(window, i + 1);
            if end > i + 1 {
                best = Some(end);
            }
        }
    }
    best
}

/// Split `text` into chunks of at most `limit` codepoints, preferring breaks
/// after sentence-ending punctuation, then clause punctuation, then any
/// whitespace. Returns `(chunk_text, start)` pairs where `start` is the
/// chunk's codepoint offset in `text` (matching ttsd's Python string
/// indices). Chunks are trimmed; whitespace between chunks is not
/// synthesized.
pub fn split_with_limit(text: &str, limit: usize) -> Vec<(String, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len <= limit {
        return vec![(text.to_string(), 0)];
    }

    let mut chunks = Vec::new();
    let mut current_pos = 0;
    while current_pos < len {
        let chunk_end = (current_pos + limit).min(len);
        if chunk_end >= len {
            chunks.push((chars[current_pos..].iter().collect(), current_pos));
            break;
        }

        let window = &chars[current_pos..chunk_end];
        let best_split = last_break_after(window, &['.', '!', '?'])
            .or_else(|| last_break_after(window, &[',', ';', ':']))
            .or_else(|| last_break_after(window, &[]))
            .filter(|&split| split >= window.len() / 2)
            .unwrap_or(limit);

        let actual: String = chars[current_pos..current_pos + best_split]
            .iter()
            .collect::<String>()
            .trim()
            .to_string();
        if !actual.is_empty() {
            chunks.push((actual, current_pos));
        }
        current_pos += best_split;
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every chunk must sit at its declared start position, stay within the
    /// limit, and the gaps between (and after) chunks must be whitespace-only
    /// — i.e. the chunks collectively cover all non-whitespace content of the
    /// source text, in order, with nothing dropped or duplicated. Port of
    /// ttsd's `_assert_covers_source`.
    fn assert_covers_source(text: &str, limit: usize, chunks: &[(String, usize)]) {
        let chars: Vec<char> = text.chars().collect();
        for (chunk_text, start) in chunks {
            let chunk_chars: Vec<char> = chunk_text.chars().collect();
            assert_eq!(
                chars[*start..*start + chunk_chars.len()],
                chunk_chars[..],
                "chunk must sit at its declared start"
            );
            assert!(chunk_chars.len() <= limit);
        }
        for (idx, (_, start)) in chunks.iter().enumerate().skip(1) {
            let (prev_text, prev_start) = &chunks[idx - 1];
            let gap: String = chars[prev_start + prev_text.chars().count()..*start]
                .iter()
                .collect();
            assert!(
                gap.trim().is_empty(),
                "gap between chunks must be whitespace-only: {gap:?}"
            );
        }
        let (last_text, last_start) = &chunks[chunks.len() - 1];
        let tail: String = chars[last_start + last_text.chars().count()..]
            .iter()
            .collect();
        assert!(tail.trim().is_empty(), "tail must be whitespace-only");
    }

    #[test]
    fn short_text_is_a_single_chunk() {
        let text = "Привет мир";
        assert_eq!(split_with_limit(text, 900), vec![(text.to_string(), 0)]);
    }

    #[test]
    fn long_text_splits_on_sentence_boundaries() {
        let text = "Это предложение. ".repeat(60);
        let chunks = split_with_limit(&text, 300);
        assert!(chunks.len() > 1);
        for (chunk_text, _) in &chunks {
            assert!(!chunk_text.trim().is_empty());
        }
        assert_covers_source(&text, 300, &chunks);
        for (chunk_text, _) in &chunks[..chunks.len() - 1] {
            assert!(
                chunk_text.trim_end().ends_with('.'),
                "chunk must end on a sentence boundary: {chunk_text:?}"
            );
        }
    }

    #[test]
    fn chunk_starts_are_ordered() {
        let text = "Слово слово слово. ".repeat(60);
        let chunks = split_with_limit(&text, 300);
        let starts: Vec<usize> = chunks.iter().map(|(_, s)| *s).collect();
        assert!(starts.windows(2).all(|w| w[0] < w[1]));
        assert_covers_source(&text, 300, &chunks);
    }

    #[test]
    fn whitespace_fallback_prefers_word_boundaries_over_midword_cut() {
        // No sentence or clause punctuation: the splitter must cut on a space,
        // never inside a word.
        let text = "слово ".repeat(100);
        let chunks = split_with_limit(&text, 24);
        assert!(chunks.len() > 1);
        for (chunk_text, _) in &chunks {
            let words: Vec<&str> = chunk_text.split_whitespace().collect();
            assert!(
                words.iter().all(|w| *w == "слово"),
                "chunk must not cut inside a word: {chunk_text:?}"
            );
        }
        assert_covers_source(&text, 24, &chunks);
    }

    #[test]
    fn unbroken_single_token_falls_back_to_hard_split() {
        // A single token longer than the limit has no whitespace to cut on;
        // the hard split is the only way to stay bounded.
        let text = "A".repeat(2000);
        let chunks = split_with_limit(&text, 300);
        assert!(chunks.len() > 1);
        for (_, start) in &chunks {
            assert_eq!(start % 300, 0);
        }
        assert_covers_source(&text, 300, &chunks);
    }

    #[test]
    fn char_offsets_not_byte_offsets() {
        // Cyrillic is 2 bytes/char: ttsd positions are Python str indices, so
        // the port must count chars, not bytes.
        let text = format!("{} в конце", "ё".repeat(400));
        let chunks = split_with_limit(&text, 300);
        let starts: Vec<usize> = chunks.iter().map(|(_, s)| *s).collect();
        assert_eq!(starts[0], 0);
        assert!(starts.iter().all(|&s| s <= text.chars().count()));
        assert_covers_source(&text, 300, &chunks);
    }
}
