//! Text sanitization and long-text chunking, ported from
//! `ttsd/ttsd/chunking.py`.
//!
//! The app pipeline keeps `\n\n` paragraph breaks in the normalized text,
//! and ttsd strips them via `sanitize_for_silero` before handing text to the
//! model. The native frontend filters symbols against the model alphabet
//! (which has no `\n`), so without this step a newline glues the surrounding
//! words together ("строки\nновая" → "строкиновая"). The port below runs
//! before [`crate::frontend::text::prepare_text_input`] so the upstream-pinned
//! golden behavior of that function stays untouched.
//!
//! `split_into_chunks` mirrors ttsd's long-text handling: the model degrades
//! on very long inputs, so text beyond [`MAX_CHUNK_SIZE`] chars is
//! synthesized in sentence-boundary chunks and the audio concatenated.

/// Maximum characters per synthesis chunk (Silero limit is ~1000-1500).
pub const MAX_CHUNK_SIZE: usize = 900;

/// Port of `ttsd.chunking.sanitize_for_silero`:
/// `re.sub(r"\s*\n\s*", " ", text)`, then `re.sub(r" +", " ", text)`,
/// then `.strip()`.
///
/// Silero's char-level tokenizer does not handle control characters;
/// newlines must become plain spaces so words across a line break stay
/// separated.
pub fn sanitize_for_silero(text: &str) -> String {
    // re.sub(r"\s*\n\s*", " ", text) — every newline, together with the
    // whitespace around it, collapses into a single space.
    let mut stage1 = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            while stage1.ends_with(|p: char| p.is_whitespace()) {
                stage1.pop();
            }
            while matches!(chars.peek(), Some(p) if p.is_whitespace()) {
                chars.next();
            }
            stage1.push(' ');
        } else {
            stage1.push(c);
        }
    }
    // re.sub(r" +", " ", text).strip()
    let mut out = String::with_capacity(stage1.len());
    let mut prev_space = false;
    for c in stage1.chars() {
        if c == ' ' {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

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

/// Port of `ttsd.chunking.split_into_chunks`: split `text` into chunks of at
/// most [`MAX_CHUNK_SIZE`] chars, preferring breaks after sentence-ending
/// punctuation, then clause punctuation, then any whitespace. Returns
/// `(chunk_text, start)` pairs where `start` is the chunk's char offset in
/// `text` (matching ttsd's Python string indices — char, not byte, offsets).
pub fn split_into_chunks(text: &str) -> Vec<(String, usize)> {
    split_with_limit(text, MAX_CHUNK_SIZE)
}

/// [`split_into_chunks`] with an explicit per-chunk char limit. Used by the
/// synthesis fallback that re-splits chunks the exported decoder cannot fit
/// (see [`crate::SileroNative::synthesize`]).
pub(crate) fn split_with_limit(text: &str, limit: usize) -> Vec<(String, usize)> {
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

    #[test]
    fn newlines_replaced_by_space() {
        assert_eq!(sanitize_for_silero("один\nдва"), "один два");
    }

    #[test]
    fn paragraph_breaks_collapse_to_single_space() {
        assert_eq!(
            sanitize_for_silero("абзац один.\n\n  абзац два"),
            "абзац один. абзац два"
        );
    }

    #[test]
    fn multiple_spaces_collapsed() {
        assert_eq!(sanitize_for_silero("один   два"), "один два");
    }

    #[test]
    fn leading_trailing_stripped() {
        assert_eq!(sanitize_for_silero("  текст  "), "текст");
        assert_eq!(sanitize_for_silero("\n\nтекст\n"), "текст");
    }

    #[test]
    fn words_across_newline_are_not_glued() {
        // Regression guard for the review finding: the symbol filter in
        // `prepare_text_input` drops '\n' entirely, so without sanitization
        // "строки\nновая" became "строкиновая".
        assert_eq!(sanitize_for_silero("строки\nновая"), "строки новая");
    }

    #[test]
    fn newline_run_with_mixed_whitespace() {
        assert_eq!(sanitize_for_silero("а \n \n\t б"), "а б");
    }

    /// Every chunk must sit at its declared start position, stay within
    /// MAX_CHUNK_SIZE, and the gaps between (and after) chunks must be
    /// whitespace-only — i.e. the chunks collectively cover all
    /// non-whitespace content of the source text, in order, with nothing
    /// dropped or duplicated. Port of ttsd's `_assert_covers_source`.
    fn assert_covers_source(text: &str, chunks: &[(String, usize)]) {
        let chars: Vec<char> = text.chars().collect();
        for (chunk_text, start) in chunks {
            let chunk_chars: Vec<char> = chunk_text.chars().collect();
            assert_eq!(
                chars[*start..*start + chunk_chars.len()],
                chunk_chars[..],
                "chunk must sit at its declared start"
            );
            assert!(chunk_chars.len() <= MAX_CHUNK_SIZE);
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
        assert_eq!(split_into_chunks(text), vec![(text.to_string(), 0)]);
    }

    #[test]
    fn long_text_splits_on_sentence_boundaries() {
        let text = "Это предложение. ".repeat(60);
        let chunks = split_into_chunks(&text);
        assert!(chunks.len() > 1);
        for (chunk_text, _) in &chunks {
            assert!(!chunk_text.trim().is_empty());
        }
        assert_covers_source(&text, &chunks);
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
        let chunks = split_into_chunks(&text);
        let starts: Vec<usize> = chunks.iter().map(|(_, s)| *s).collect();
        assert!(starts.windows(2).all(|w| w[0] < w[1]));
        assert_covers_source(&text, &chunks);
    }

    #[test]
    fn unbroken_text_falls_back_to_hard_split() {
        let text = "A".repeat(2000);
        let chunks = split_into_chunks(&text);
        assert!(chunks.len() > 1);
        for (_, start) in &chunks {
            assert_eq!(*start % MAX_CHUNK_SIZE, 0);
        }
        assert_covers_source(&text, &chunks);
    }

    #[test]
    fn char_offsets_not_byte_offsets() {
        // Cyrillic is 2 bytes/char: ttsd positions are Python str indices,
        // so the port must count chars, not bytes.
        let text = format!("{} в конце", "ё".repeat(950));
        let chunks = split_into_chunks(&text);
        let starts: Vec<usize> = chunks.iter().map(|(_, s)| *s).collect();
        assert_eq!(starts[0], 0);
        assert!(starts.iter().all(|&s| s <= text.chars().count()));
        assert_covers_source(&text, &chunks);
    }
}
