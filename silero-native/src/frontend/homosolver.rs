//! HomoSolver: homograph disambiguation via the BERT model.
//!
//! Faithful port of `models/homosolver.py` from the upstream package. The
//! word pattern `(?=.*[а-яё])[а-яё+]+` (case-insensitive) is implemented
//! without a regex engine: maximal runs of Cyrillic letters and `+` that
//! contain at least one letter.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use ort::session::Session;
use ort::value::Tensor;
use tracing::{debug, instrument};

use super::bert::BertTokenizer;
use super::HomosolverConfig;
use crate::error::{EngineError, Result};

/// Char classes of the upstream word pattern `[а-яё+]+` (IGNORECASE).
fn is_pattern_char(c: char) -> bool {
    ('а'..='я').contains(&c) || ('А'..='Я').contains(&c) || c == 'ё' || c == 'Ё' || c == '+'
}

fn is_pattern_letter(c: char) -> bool {
    is_pattern_char(c) && c != '+'
}

/// Find all matches of `(?=.*[а-яё])[а-яё+]+` as `(start, end)` char indices
/// into `chars`.
fn find_word_spans(chars: &[char]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if is_pattern_char(chars[i]) {
            let start = i;
            let mut has_letter = false;
            while i < chars.len() && is_pattern_char(chars[i]) {
                if is_pattern_letter(chars[i]) {
                    has_letter = true;
                }
                i += 1;
            }
            if has_letter {
                spans.push((start, i));
            }
        } else {
            i += 1;
        }
    }
    spans
}

/// `torch.round` semantics: round half to even (banker's rounding).
fn round_half_even(x: f32) -> f32 {
    let r = x.round();
    if (x - x.trunc() - 0.5).abs() < f32::EPSILON && (r as i64) % 2 != 0 {
        r - 1.0
    } else {
        r
    }
}

/// Homograph solver: tokenizer, homodict and the ONNX BERT session.
pub struct HomoSolver {
    tokenizer: BertTokenizer,
    /// homograph (lowercase) → accented variants (as shipped, unsorted).
    homodict: HashMap<String, Vec<String>>,
    session: Mutex<Session>,
}

impl HomoSolver {
    /// Load `vocab.txt` + `homodict.json` and take ownership of the
    /// homosolver ONNX session.
    pub fn load(bundle_dir: &Path, config: &HomosolverConfig, session: Session) -> Result<Self> {
        let tokenizer = BertTokenizer::load(
            &bundle_dir.join("vocab.txt"),
            config.never_split.iter().cloned().collect(),
            config,
        )?;
        let homodict_text = std::fs::read_to_string(bundle_dir.join("homodict.json"))
            .map_err(|e| EngineError::Bundle(format!("cannot read homodict.json: {e}")))?;
        let homodict: HashMap<String, Vec<String>> = serde_json::from_str(&homodict_text)
            .map_err(|e| EngineError::Bundle(format!("malformed homodict.json: {e}")))?;
        Ok(Self {
            tokenizer,
            homodict,
            session: Mutex::new(session),
        })
    }

    /// Variant list for a homograph, sorted as upstream does
    /// (`sorted(self.homodict[word])`).
    pub fn variants(&self, word_lower: &str) -> Option<Vec<String>> {
        let mut variants = self.homodict.get(word_lower)?.clone();
        variants.sort();
        Some(variants)
    }

    /// Port of `HomoSolver.__call__`: tag every homograph with
    /// `[HOMO]`/`[/HOMO]`, let the BERT pick the variant, splice the
    /// resolved words back into the sentence.
    ///
    /// Upstream's flags are fixed to the only values the pipeline (and
    /// ttsd) ever passes: put_stress = put_yo = stress_single_vowel = true.
    #[instrument(skip_all, fields(sentence_len = sentence.len()))]
    pub fn resolve(&self, sentence: &str) -> Result<String> {
        let chars: Vec<char> = sentence.chars().collect();
        let spans = find_word_spans(&chars);

        struct Homo {
            start: usize,
            word: String,
            ids: Vec<i64>,
            homo_start: usize,
            homo_end: usize,
        }
        let mut homos = Vec::new();
        for (start, end) in spans {
            let word: String = chars[start..end].iter().collect();
            if !self.homodict.contains_key(&word.to_lowercase()) {
                continue;
            }
            // raw_mark = sentence[:start] + ' [HOMO] ' + word + ' [/HOMO] ' + sentence[end:]
            let mut raw_mark = String::new();
            raw_mark.extend(&chars[..start]);
            raw_mark.push_str(" [HOMO] ");
            raw_mark.extend(&chars[start..end]);
            raw_mark.push_str(" [/HOMO] ");
            raw_mark.extend(&chars[end..]);
            let ids = self.tokenizer.encode(&raw_mark);
            // First occurrence of each marker (torch.where(...)[0][0]).
            let homo_start = ids
                .iter()
                .position(|id| *id == self.tokenizer.homo_start_id);
            let homo_end = ids.iter().position(|id| *id == self.tokenizer.homo_end_id);
            match (homo_start, homo_end) {
                (Some(homo_start), Some(homo_end)) => homos.push(Homo {
                    start,
                    word,
                    ids,
                    homo_start,
                    homo_end,
                }),
                _ => {
                    return Err(EngineError::Synthesis(format!(
                        "homosolver tokenizer dropped HOMO markers for word {word:?}"
                    )));
                }
            }
        }
        if homos.is_empty() {
            return Ok(sentence.to_string());
        }

        // Batch: pad to the longest sequence with pad_token_id.
        let batch = homos.len();
        let max_len = homos.iter().map(|h| h.ids.len()).max().unwrap_or(0);
        let mut input_ids = vec![self.tokenizer.pad_token_id; batch * max_len];
        let mut starts = Vec::with_capacity(batch);
        let mut ends = Vec::with_capacity(batch);
        for (row, homo) in homos.iter().enumerate() {
            input_ids[row * max_len..row * max_len + homo.ids.len()].copy_from_slice(&homo.ids);
            starts.push(homo.homo_start as i64);
            ends.push(homo.homo_end as i64);
        }

        let ids_t = Tensor::<i64>::from_array((vec![batch, max_len], input_ids))?;
        let starts_t = Tensor::<i64>::from_array((vec![batch], starts))?;
        let ends_t = Tensor::<i64>::from_array((vec![batch], ends))?;
        let mut session = crate::lock_session(&self.session);
        let outputs = session.run(
            ort::inputs!["input_ids" => ids_t, "homo_start_ids" => starts_t, "homo_end_ids" => ends_t],
        )?;
        let (logits_shape, logits) = outputs["logits"].try_extract_tensor::<f32>()?;
        if logits_shape.to_vec() != vec![batch as i64, 1] {
            return Err(EngineError::Synthesis(format!(
                "homosolver logits shape mismatch: {:?}, expected ({batch}, 1)",
                logits_shape.to_vec()
            )));
        }

        // Splice resolved words back; `offset` tracks insertions of '+'.
        // Upstream ordering: apply the CURRENT offset to the span, then
        // insert this word's '+' (which shifts all later spans by one).
        let mut out: Vec<char> = chars;
        let mut offset: i64 = 0;
        for (i, homo) in homos.iter().enumerate() {
            let logit = logits[i];
            // pred = round(sigmoid(logit)); torch.round is half-to-even.
            let sigmoid = 1.0 / (1.0 + (-logit).exp());
            let pred = round_half_even(sigmoid) as usize;
            let word_lower = homo.word.to_lowercase();
            let variants = self.variants(&word_lower).ok_or_else(|| {
                EngineError::Internal(format!("homodict entry vanished for {word_lower:?}"))
            })?;
            if pred >= variants.len() {
                return Err(EngineError::Synthesis(format!(
                    "homosolver prediction {pred} out of range for {word_lower:?} ({} variants)",
                    variants.len()
                )));
            }
            let word_pred = variants[pred].clone();
            let pred_chars: Vec<char> = word_pred.chars().collect();
            let stress_idx = pred_chars.iter().position(|c| *c == '+');
            let no_stress: Vec<char> = pred_chars.iter().copied().filter(|c| *c != '+').collect();
            // Case-map onto the original word's letters (zip truncation).
            let word_chars: Vec<char> = homo.word.chars().collect();
            let mut resolved: Vec<char> = word_chars
                .iter()
                .zip(no_stress.iter())
                .map(|(orig, pred)| {
                    if orig.is_lowercase() {
                        pred.to_lowercase().next().unwrap_or(*pred)
                    } else {
                        pred.to_uppercase().next().unwrap_or(*pred)
                    }
                })
                .collect();
            let start = (homo.start as i64 + offset) as usize;
            let end = start + word_chars.len();
            if let Some(idx) = stress_idx {
                resolved.insert(idx, '+');
                offset += 1;
            }
            out.splice(start..end, resolved);
        }
        debug!("homosolver done");
        Ok(out.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_spans_cover_cyrillic_and_stress_markers_only() {
        let chars: Vec<char> = "з+амок, test 123 слово".chars().collect();
        let spans = find_word_spans(&chars);
        let words: Vec<String> = spans
            .iter()
            .map(|(s, e)| chars[*s..*e].iter().collect())
            .collect();
        assert_eq!(words, vec!["з+амок", "слово"]);
    }

    #[test]
    fn word_spans_skip_lone_plus_runs() {
        let chars: Vec<char> = "а ++ б".chars().collect();
        let spans = find_word_spans(&chars);
        let words: Vec<String> = spans
            .iter()
            .map(|(s, e)| chars[*s..*e].iter().collect())
            .collect();
        assert_eq!(words, vec!["а", "б"]);
    }

    #[test]
    fn round_half_even_matches_torch_round() {
        assert_eq!(round_half_even(0.5), 0.0);
        assert_eq!(round_half_even(1.5), 2.0);
        assert_eq!(round_half_even(2.5), 2.0);
        assert_eq!(round_half_even(0.7), 1.0);
        assert_eq!(round_half_even(0.4), 0.0);
    }
}
