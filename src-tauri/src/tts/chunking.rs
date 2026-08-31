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

/// Offset of the first blank line in `window` (two consecutive newline-ish
/// characters, `\r` counting as a newline for Windows text) — a paragraph
/// break. A paragraph break always ends the current chunk, so the engine can
/// insert an audible pause between paragraphs (espeak-ng reads a blank line
/// as a plain space).
fn first_paragraph_break(window: &[char]) -> Option<usize> {
    let is_nl = |c: char| c == '\n' || c == '\r';
    window.windows(2).position(|w| is_nl(w[0]) && is_nl(w[1]))
}

/// Split `text` into chunks of at most `limit` codepoints, preferring breaks
/// after sentence-ending punctuation, then clause punctuation, then any
/// whitespace. A paragraph break (blank line) always ends the current chunk
/// even before the limit is reached, so each paragraph is synthesized on its
/// own and the engine can pause between paragraphs. Returns `(chunk_text,
/// start)` pairs where `start` is the chunk's codepoint offset in `text`
/// (matching ttsd's Python string indices). Chunks are trimmed to their
/// actual content, and `start` points at the first content codepoint — so a
/// chunk always sits exactly at its declared position; whitespace between
/// chunks is not synthesized. The single-chunk fast path is the exception:
/// a text within the limit is returned verbatim from offset 0 (no leading
/// whitespace in real pipeline output).
pub fn split_with_limit(text: &str, limit: usize) -> Vec<(String, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len <= limit && first_paragraph_break(&chars).is_none() {
        return vec![(text.to_string(), 0)];
    }

    let mut chunks = Vec::new();
    let mut current_pos = 0;
    while current_pos < len {
        let chunk_end = (current_pos + limit).min(len);
        let window = &chars[current_pos..chunk_end];

        // A blank line inside the window always ends the chunk. Leading
        // whitespace is skipped — it belongs to the previous gap (after a
        // paragraph split the window opens with the blank line itself), and
        // the search starts from the first content character.
        let content_start = window
            .iter()
            .position(|c| !c.is_whitespace())
            .unwrap_or(window.len());
        if let Some(at) = first_paragraph_break(&window[content_start..])
            .map(|p| p + content_start)
            .filter(|&at| at > content_start)
        {
            let raw: String = chars[current_pos..current_pos + at].iter().collect();
            let lead = raw.chars().count() - raw.trim_start().chars().count();
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                chunks.push((trimmed.to_string(), current_pos + lead));
            }
            current_pos += at;
            continue;
        }

        if chunk_end >= len {
            let last: String = chars[current_pos..].iter().collect();
            let lead = last.chars().count() - last.trim_start().chars().count();
            let trimmed = last.trim();
            if !trimmed.is_empty() {
                chunks.push((trimmed.to_string(), current_pos + lead));
            }
            break;
        }

        let best_split = last_break_after(window, &['.', '!', '?'])
            .or_else(|| last_break_after(window, &[',', ';', ':']))
            .or_else(|| last_break_after(window, &[]))
            .filter(|&split| split >= window.len() / 2)
            .unwrap_or(limit);

        let raw: String = chars[current_pos..current_pos + best_split]
            .iter()
            .collect();
        // `start` must point at the chunk's first content codepoint: a hard
        // split can land right after a whitespace run (or the window can open
        // with one), and the trimmed chunk then begins later than the raw
        // window — consumers (timestamps) slice the source by this offset.
        let lead = raw.chars().count() - raw.trim_start().chars().count();
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            chunks.push((trimmed.to_string(), current_pos + lead));
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
    fn hard_split_after_whitespace_offsets_shift_with_trim() {
        // A hard split landing right after a whitespace run: the next chunk's
        // declared start must move to its first content codepoint, otherwise
        // consumers slicing the source by the offset get off-by-one word
        // positions (review finding on the #155 branch).
        let text = "aa bbbbccc ddddddddddd eee";
        let chunks = split_with_limit(text, 10);
        assert!(chunks.len() > 1);
        for (chunk_text, start) in &chunks {
            let start_byte = text
                .char_indices()
                .nth(*start)
                .map(|(b, _)| b)
                .expect("start in range");
            assert!(
                text[start_byte..].starts_with(chunk_text.as_str()),
                "chunk {chunk_text:?} must sit at its declared offset {start}"
            );
        }
        assert_covers_source(text, 10, &chunks);
    }

    #[test]
    fn paragraph_break_always_ends_the_chunk() {
        // Even far below the limit, a blank line ends the chunk so the engine
        // can pause between paragraphs.
        let text = "Первая часть.\n\nВторая часть.\n\nТретья.";
        let chunks = split_with_limit(text, 300);
        assert_eq!(
            chunks,
            vec![
                ("Первая часть.".to_string(), 0),
                ("Вторая часть.".to_string(), 15),
                ("Третья.".to_string(), 30),
            ]
        );
        assert_covers_source(text, 300, &chunks);
    }

    #[test]
    fn long_paragraph_still_splits_by_sentences() {
        // A paragraph longer than the limit falls back to sentence splitting
        // inside the paragraph, and the trailing blank line starts a fresh
        // chunk for the last short paragraph.
        let paragraph = format!(
            "{}. {}. {}.\n\n{}",
            "Слово".repeat(30),
            "Ещё".repeat(30),
            "И".repeat(30),
            "Хвост."
        );
        let chunks = split_with_limit(&paragraph, 200);
        assert!(chunks.len() >= 3, "long paragraph must split: {chunks:?}");
        assert_covers_source(&paragraph, 200, &chunks);
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
