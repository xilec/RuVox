//! Chunking under the model's seconds limit.
//!
//! The profile declares `Secs(20)`; the shared splitter only understands
//! codepoints, so the engine converts the seconds limit into a conservative
//! codepoint budget, splits, and then validates every chunk with the model's
//! own duration predictor. A chunk predicted above the hard 25-second call
//! limit is split again at an inner sentence boundary (the shared splitter's
//! preference order) and re-validated. Keeping the estimator here means the
//! shared crate stays free of inference calls.

use tts_contract::{Chunk as ChunkPolicy, Limit};

use crate::error::{Error, Result};

/// Conservative Russian narration rate used to turn `Secs` into a codepoint
/// window. Measured on the golden corpus: 11–13 chars/s, so 12 leaves headroom
/// and the duration predictor catches the rest.
pub const CHARS_PER_SEC: f64 = 12.0;
/// The model's hard call limit; a longer single call degrades badly.
pub const MAX_CHUNK_SECS: f64 = 25.0;

/// One synthesis unit.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub text: String,
    /// Codepoint offset of the chunk's first character in the source text.
    pub start: usize,
    /// Whether the source had a paragraph break right after this chunk.
    pub paragraph_break_after: bool,
}

/// Split `text` according to `policy`, validating each chunk with `estimate`.
pub fn split(
    text: &str,
    policy: &ChunkPolicy,
    estimate: impl Fn(&str) -> Result<f64>,
) -> Result<Vec<Chunk>> {
    let budget = match policy.limit {
        Limit::Chars(chars) => chars,
        Limit::Secs(seconds) => ((seconds as f64 * CHARS_PER_SEC) as usize).max(1),
    };
    let mut parts = Vec::new();
    for (part, start) in tts_contract::split_with_limit(text, budget) {
        split_validated(&part, start, budget, &estimate, &mut parts, 0)?;
    }
    // A sentence split can leave a punctuation-only part (the period after an
    // English fragment, followed by a paragraph break). Synthesizing it alone
    // makes the model vocalize the punctuation (an audible "пмснееер" between
    // sentences), so attach it to the previous word-bearing chunk and drop a
    // leading one.
    let mut merged: Vec<Chunk> = Vec::with_capacity(parts.len());
    for chunk in parts {
        if chunk.text.chars().any(char::is_alphanumeric) {
            merged.push(chunk);
        } else if let Some(previous) = merged.last_mut() {
            previous.text.push_str(&chunk.text);
        }
    }
    let mut parts = merged;
    if parts.is_empty() {
        return Err(Error::InvalidText("text produced no chunks".into()));
    }

    let chars: Vec<char> = text.chars().collect();
    let total = parts.len();
    for index in 0..total {
        let end = parts[index].start + parts[index].text.chars().count();
        let next_start = parts
            .get(index + 1)
            .map(|chunk| chunk.start)
            .unwrap_or(chars.len());
        parts[index].paragraph_break_after = paragraph_break_between(&chars, end, next_start);
    }
    Ok(parts)
}

fn split_validated(
    part: &str,
    start: usize,
    budget: usize,
    estimate: &impl Fn(&str) -> Result<f64>,
    out: &mut Vec<Chunk>,
    depth: usize,
) -> Result<()> {
    let duration = estimate(part)?;
    if duration <= MAX_CHUNK_SECS || part.chars().count() <= 1 || depth >= 8 {
        out.push(Chunk {
            text: part.to_string(),
            start,
            paragraph_break_after: false,
        });
        return Ok(());
    }
    // The shared splitter already prefers sentence, then clause, then
    // whitespace, then a hard split; halving the budget drives the same
    // preference order on the over-budget part.
    let smaller = (part.chars().count() / 2)
        .max(1)
        .min(budget.saturating_sub(1).max(1));
    let sub_parts = tts_contract::split_with_limit(part, smaller);
    if sub_parts.len() <= 1 {
        out.push(Chunk {
            text: part.to_string(),
            start,
            paragraph_break_after: false,
        });
        return Ok(());
    }
    for (sub, offset) in sub_parts {
        split_validated(&sub, start + offset, smaller, estimate, out, depth + 1)?;
    }
    Ok(())
}

fn paragraph_break_between(chars: &[char], end: usize, next_start: usize) -> bool {
    if next_start <= end || next_start > chars.len() {
        return false;
    }
    let mut newlines = 0;
    for character in &chars[end..next_start] {
        if *character == '\n' || *character == '\r' {
            newlines += 1;
        }
    }
    newlines >= 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use tts_contract::{
        Charset, Digits, Markup, Pauses, Reading, Route, Stress, TextProfile, Units,
    };

    fn policy(limit: Limit) -> ChunkPolicy {
        ChunkPolicy {
            limit,
            pauses: Pauses {
                paragraph_ms: 250,
                segment_ms: 200,
            },
        }
    }

    fn short_estimate(_: &str) -> Result<f64> {
        Ok(5.0)
    }

    #[test]
    fn splits_a_long_text_into_budgeted_chunks() {
        let text = "Первое предложение. ".repeat(80);
        let chunks = split(&text, &policy(Limit::Secs(20)), short_estimate).unwrap();
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.text.chars().count() <= (20.0 * CHARS_PER_SEC) as usize);
            assert!(!chunk.text.trim().is_empty());
        }
    }

    #[test]
    fn paragraph_breaks_are_marked() {
        let text = "Первый абзац.\n\nВторой абзац.";
        let chunks = split(text, &policy(Limit::Chars(1000)), short_estimate).unwrap();
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].paragraph_break_after);
        assert!(!chunks[1].paragraph_break_after);
    }

    #[test]
    fn over_budget_parts_are_resplit() {
        let text = "Очень длинное предложение. ".repeat(20);
        let estimate = |part: &str| {
            if part.chars().count() > 100 {
                Ok(30.0)
            } else {
                Ok(5.0)
            }
        };
        let chunks = split(&text, &policy(Limit::Chars(1000)), estimate).unwrap();
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.text.chars().count() <= 100);
        }
    }

    #[test]
    fn punctuation_only_parts_are_merged_into_a_neighbour() {
        let chunks = split(
            ".\n\nДокументация.",
            &policy(Limit::Chars(1000)),
            short_estimate,
        )
        .unwrap();
        assert!(
            chunks
                .iter()
                .all(|chunk| chunk.text.chars().any(char::is_alphanumeric)),
            "wordless chunk survived: {chunks:?}"
        );
    }

    #[test]
    fn empty_text_is_rejected() {
        assert!(split("   ", &policy(Limit::Secs(20)), short_estimate).is_err());
    }

    #[test]
    fn a_single_char_still_produces_a_chunk() {
        let chunks = split("а", &policy(Limit::Secs(20)), |_| Ok(30.0)).unwrap();
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn profile_types_are_constructible() {
        // Keeps the test imports honest: the policy types come from the shared
        // contract, not from an engine-local copy.
        let profile = TextProfile {
            languages: &["ru"],
            reads_latin_natively: false,
            route: Route {
                identifier: Reading::Narrate,
                code_block: Reading::Narrate,
            },
            digits: Digits::Expanded,
            units: Units::Expanded,
            stress: Stress::Manual,
            charset: Charset::ModelAlphabet,
            chunk: policy(Limit::Secs(20)),
            markup: Markup::RuEnTags,
        };
        profile.validate().unwrap();
    }
}
