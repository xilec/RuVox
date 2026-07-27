//! Text sanitization port of `ttsd/ttsd/chunking.py`.
//!
//! The app pipeline keeps `\n\n` paragraph breaks in the normalized text,
//! and ttsd strips them via `sanitize_for_silero` before handing text to the
//! model. The native frontend filters symbols against the model alphabet
//! (which has no `\n`), so without this step a newline glues the surrounding
//! words together ("строки\nновая" → "строкиновая"). The port below runs
//! before [`crate::frontend::text::prepare_text_input`] so the upstream-pinned
//! golden behavior of that function stays untouched.

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
}
