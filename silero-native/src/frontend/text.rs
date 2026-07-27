//! Text normalization: a faithful port of `prepare_text_input` /
//! `clean_star_text` from the upstream `multi_acc_v3_package.py` (non-SSML,
//! non-phonetic path), plus conversion of the normalized sentence into the
//! model's `sequence` tensor.
//!
//! Upstream reference: `tmp/onnx-spike/pkg/multi_acc_v3_package.py`.

use std::collections::HashMap;

use tracing::debug;

use crate::error::{EngineError, Result};

/// Result of [`prepare_text_input`]: the filtered sentence fed to the
/// accentor, the letter-only "clean" sentence used for the emptiness check,
/// and whether any Cyrillic letters survived.
pub struct PreparedText {
    pub sentence: String,
    pub clean_sentence: String,
    pub has_text: bool,
}

/// Port of `PartTTSModelMultiAcc_v3.clean_star_text`.
///
/// Handles the `^` "skip stress" marker: `^` is not part of the model
/// alphabet, so markers are normalized here and removed before sequence
/// construction (see [`build_sequence`]).
pub fn clean_star_text(text: &str) -> String {
    // text.replace('^', ' ^')
    let text = text.replace('^', " ^");
    // re.sub(r'\s+', ' ', text).strip()
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    // text.replace('^ ', '^')
    let text = text.replace("^ ", "^");
    // re.sub(r'\^+', '^', text).strip()
    let mut out = String::with_capacity(text.len());
    let mut prev_caret = false;
    for c in text.chars() {
        if c == '^' {
            if !prev_caret {
                out.push(c);
            }
            prev_caret = true;
        } else {
            out.push(c);
            prev_caret = false;
        }
    }
    let text = out.trim().to_string();
    // text[:-1].strip() if text ends with '^'
    match text.strip_suffix('^') {
        Some(stripped) => stripped.trim().to_string(),
        None => text,
    }
}

/// Port of `PartTTSModelMultiAcc_v3.prepare_text_input` for the char-based
/// (non-phons) model: lowercase, dash normalization, symbol filtering against
/// the model alphabet tail (`symbols[3:]` plus `^`), star-marker cleanup.
///
/// `symbols_tail` must be the char set of `symbols[3:]` from `frontend.json`.
/// Upstream builds the keep-set with the regex `[^<symbols[3:]>\^]`; the
/// `-` between `,` and `.` forms a range in that class but adds no new chars
/// (both are already in `symbols`), so a plain set lookup is equivalent.
pub fn prepare_text_input(text: &str, symbols_tail: &std::collections::HashSet<char>) -> PreparedText {
    let lowered = text.to_lowercase();
    // replace('—', '–').replace('–', '–').replace('‑', '-')
    // (the middle replace is a no-op upstream and only documents intent)
    let dashed = lowered.replace('—', "–").replace('\u{2011}', "-");
    let filtered: String = dashed
        .chars()
        .filter(|c| symbols_tail.contains(c) || *c == '^')
        .collect();
    let sentence = clean_star_text(&filtered);
    // re.sub(r'[^а-я\- ]', '', sentence) — note: 'ё' (U+0451) is outside the
    // 'а'-'я' range in Python regex, exactly as in Rust char ranges.
    let clean_sentence: String = sentence
        .chars()
        .filter(|c| ('а'..='я').contains(c) || *c == '-' || *c == ' ')
        .collect();
    let has_text = clean_sentence.chars().any(|c| c != ' ');
    debug!(sentence = %sentence, has_text, "text normalized");
    PreparedText {
        sentence,
        clean_sentence,
        has_text,
    }
}

/// Build the model input sequence: `sos + sentence + eos` mapped through
/// `symbol_to_id`.
///
/// Chars missing from `symbol_to_id` (currently only the `^` stress-skip
/// marker, which survives `clean_star_text` but has no model id) are dropped
/// with a debug log. Upstream would raise `KeyError` on such input; dropping
/// is the typed-error-friendly equivalent — the marker only affects stress
/// placement, which our accentor port does not implement either (v5 upstream
/// ignores `^` the same way).
pub fn build_sequence(
    sentence: &str,
    symbol_to_id: &HashMap<String, i64>,
    sos_token: &str,
    eos_token: &str,
) -> Result<Vec<i64>> {
    let sos = symbol_to_id.get(sos_token).ok_or_else(|| {
        EngineError::Bundle(format!("sos token {sos_token:?} missing from symbol_to_id"))
    })?;
    let eos = symbol_to_id.get(eos_token).ok_or_else(|| {
        EngineError::Bundle(format!("eos token {eos_token:?} missing from symbol_to_id"))
    })?;
    let mut ids = Vec::with_capacity(sentence.chars().count() + 2);
    ids.push(*sos);
    for c in sentence.chars() {
        let key = c.to_string();
        match symbol_to_id.get(&key) {
            Some(id) => ids.push(*id),
            None => debug!(char = %c, "dropping char without symbol id"),
        }
    }
    ids.push(*eos);
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn symbols_tail() -> HashSet<char> {
        // symbols[3:] of the v5_ru bundle frontend.json
        "!+,-.:;?абвгдежзийклмнопрстуфхцчшщъыьэюяё–… ".chars().collect()
    }

    #[test]
    fn lowercases_and_filters_unknown_symbols() {
        let out = prepare_text_input("Привет, World! 123", &symbols_tail());
        assert_eq!(out.sentence, "привет, !");
        assert_eq!(out.clean_sentence, "привет ");
        assert!(out.has_text);
    }

    #[test]
    fn normalizes_dashes() {
        let out = prepare_text_input("раз—два\u{2011}три–четыре", &symbols_tail());
        assert_eq!(out.sentence, "раз–два-три–четыре");
    }

    #[test]
    fn empty_after_filtering_has_no_text() {
        let out = prepare_text_input("12345 !!!", &symbols_tail());
        assert!(!out.has_text);
    }

    #[test]
    fn star_markers_are_collapsed_and_trimmed() {
        assert_eq!(clean_star_text("слово^"), "слово");
        assert_eq!(clean_star_text("^^сло^^во^ ^"), "^сло ^во");
        assert_eq!(clean_star_text("а  ^  б"), "а ^б");
        assert_eq!(clean_star_text("^"), "");
    }

    #[test]
    fn sequence_wraps_with_sos_eos() {
        let map: HashMap<String, i64> = [
            ("|".to_string(), 2),
            ("~".to_string(), 1),
            ("а".to_string(), 11),
            ("б".to_string(), 12),
        ]
        .into_iter()
        .collect();
        let ids = build_sequence("аб", &map, "|", "~").expect("sequence");
        assert_eq!(ids, vec![2, 11, 12, 1]);
    }
}
