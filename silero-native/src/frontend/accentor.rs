//! Ngram accentor: stress (`+`) and `ё` placement.
//!
//! Faithful port of `models/accentor.py` (`AccentorNgram`) from the upstream
//! package. The tensor part (embedding-bag + two linear classifiers) runs as
//! `accentor_tensor.onnx`; the string logic (tokenize, exceptions, placement)
//! lives here. Ngram extraction matches `word_ngrams` in
//! `silero-native/export/export.py`, which is an exact copy of the JIT
//! model's internal implementation.

use std::collections::{HashMap, HashSet};
use std::io::Read as _;
use std::path::Path;
use std::sync::Mutex;

use ort::session::Session;
use ort::value::Tensor;
use tracing::{debug, instrument};

use super::AccentorConfig;
use crate::error::{EngineError, Result};

/// One gzipped dictionary of the accentor bundle.
fn read_gzip(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)
        .map_err(|e| EngineError::Bundle(format!("cannot open {}: {e}", path.display())))?;
    let mut text = String::new();
    flate2::read::GzDecoder::new(file)
        .read_to_string(&mut text)
        .map_err(|e| EngineError::Bundle(format!("cannot decompress {}: {e}", path.display())))?;
    Ok(text)
}

/// `ngrams.gz`: space-separated grams, id = position in the stream.
fn load_ngrams(path: &Path) -> Result<HashMap<String, i64>> {
    let text = read_gzip(path)?;
    Ok(text
        .split_whitespace()
        .enumerate()
        .map(|(i, gram)| (gram.to_string(), i as i64))
        .collect())
}

/// `exceptions.gz`: `word stress_char_idx yo_char_idx` per line
/// (`yo_char_idx == -1` means "no ё"; both indices are char positions).
fn load_exceptions(path: &Path) -> Result<HashMap<String, (usize, i64)>> {
    let text = read_gzip(path)?;
    let mut map = HashMap::new();
    for (lineno, line) in text.lines().enumerate() {
        let mut parts = line.split_whitespace();
        let (Some(word), Some(stress), Some(yo)) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(EngineError::Bundle(format!(
                "malformed exceptions.gz line {}: {line:?}",
                lineno + 1
            )));
        };
        let stress: usize = stress.parse().map_err(|_| {
            EngineError::Bundle(format!(
                "bad stress index on exceptions.gz line {}",
                lineno + 1
            ))
        })?;
        let yo: i64 = yo.parse().map_err(|_| {
            EngineError::Bundle(format!("bad yo index on exceptions.gz line {}", lineno + 1))
        })?;
        map.insert(word.to_string(), (stress, yo));
    }
    Ok(map)
}

/// Exact copy of the upstream `word_ngrams` (see module docs).
fn word_ngrams(text: &[char], min_len: usize, max_len: usize) -> Vec<String> {
    let mut ext = Vec::with_capacity(text.len() + 2);
    ext.push('<');
    ext.extend_from_slice(text);
    ext.push('>');
    let mut grams = Vec::new();
    for i in min_len..=max_len {
        if i > ext.len() {
            break;
        }
        for j in 0..=(ext.len() - i) {
            grams.push(ext[j..j + i].iter().collect());
        }
    }
    if text.len() < min_len {
        grams.push(text.iter().collect());
    }
    grams
}

/// Numerically stable softmax, matching `torch.softmax` closely enough for
/// the 0.5 decision thresholds (exact bit parity is not required: the export
/// self-check gates decisions, not raw logits).
fn softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = logits.iter().map(|x| (x - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    exps.iter().map(|x| x / sum).collect()
}

/// First index of the maximum (torch.argmax tie behavior is irrelevant here:
/// float logits never tie exactly in practice).
fn argmax(values: &[f32]) -> usize {
    values
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Token triple produced by [`Accentor::tokenize`].
struct Tokens {
    raw: Vec<String>,
    clean: Vec<String>,
    mask: Vec<bool>,
}

/// Model predictions per input word: `(stress_probs, stress_preds, yo_probs, yo_preds)`.
type ModelPreds = (Vec<Vec<f32>>, Vec<usize>, Vec<Vec<f32>>, Vec<usize>);

/// The ngram accentor: dictionaries plus the ONNX tensor model.
pub struct Accentor {
    ngrams: HashMap<String, i64>,
    unk_id: i64,
    exceptions: HashMap<String, (usize, i64)>,
    session: Mutex<Session>,
    stress_token: char,
    vowels: HashSet<char>,
    stop_words: HashSet<String>,
    stress_threshold: f32,
    yo_threshold: f32,
    stress_dim: usize,
    yo_dim: usize,
}

impl Accentor {
    /// Load dictionaries and take ownership of the accentor ONNX session.
    pub fn load(bundle_dir: &Path, config: &AccentorConfig, session: Session) -> Result<Self> {
        let ngrams = load_ngrams(&bundle_dir.join("ngrams.gz"))?;
        let unk_id = *ngrams.get(&config.unk_token).ok_or_else(|| {
            EngineError::Bundle(format!("ngrams.gz has no {:?} entry", config.unk_token))
        })?;
        let exceptions = load_exceptions(&bundle_dir.join("exceptions.gz"))?;
        let stress_token = config
            .stress_token
            .chars()
            .next()
            .ok_or_else(|| EngineError::Bundle("accentor stress_token is empty".to_string()))?;
        Ok(Self {
            ngrams,
            unk_id,
            exceptions,
            session: Mutex::new(session),
            stress_token,
            vowels: config.vowels.chars().collect(),
            stop_words: config.stop_words.iter().cloned().collect(),
            stress_threshold: config.stress_threshold,
            yo_threshold: config.yo_threshold,
            stress_dim: config.stress_logits_dim,
            yo_dim: config.yo_logits_dim,
        })
    }

    /// Port of `AccentorNgram._tokenize`: split on whitespace (separators
    /// kept as own tokens), then on `-`; clean tokens keep only Cyrillic.
    fn tokenize(&self, sentence: &str) -> Tokens {
        let mut raw = Vec::new();
        let mut clean = Vec::new();
        let mut mask = Vec::new();

        // re.split(r'(\s+)', sentence) — whitespace runs become own tokens.
        let mut words: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut cur_ws: Option<bool> = None;
        for c in sentence.chars() {
            let ws = c.is_whitespace();
            if cur_ws == Some(ws) {
                cur.push(c);
            } else {
                if !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                }
                cur.push(c);
                cur_ws = Some(ws);
            }
        }
        if !cur.is_empty() {
            words.push(cur);
        }

        for word in &words {
            let parts: Vec<&str> = word.split('-').collect();
            let (cur_tokens, cur_mask): (Vec<String>, Vec<bool>) = if parts.len() == 1 {
                (vec![word.clone()], vec![true])
            } else {
                let mut toks: Vec<String> = parts[..parts.len() - 1]
                    .iter()
                    .map(|p| format!("{p}-"))
                    .collect();
                toks.push(parts[parts.len() - 1].to_string());
                let mut m: Vec<bool> = vec![true; parts.len() - 1];
                m.push(!self.stop_words.contains(parts[parts.len() - 1]));
                (toks, m)
            };
            for (tok, m) in cur_tokens.into_iter().zip(cur_mask) {
                // re.sub(r'[^А-Яа-яёЁ]', '', token.lower())
                let cleaned: String = tok
                    .to_lowercase()
                    .chars()
                    .filter(|c| {
                        ('а'..='я').contains(c) || ('А'..='Я').contains(c) || *c == 'ё' || *c == 'Ё'
                    })
                    .collect();
                mask.push(!cleaned.is_empty() && m);
                raw.push(tok);
                clean.push(cleaned);
            }
        }
        Tokens { raw, clean, mask }
    }

    /// Build `ind`/`offsets` for `accentor_tensor.onnx` from clean tokens.
    fn model_inputs(&self, words: &[String]) -> (Vec<i64>, Vec<i64>) {
        let mut ind = Vec::new();
        let mut offsets = Vec::with_capacity(words.len());
        for word in words {
            offsets.push(ind.len() as i64);
            let chars: Vec<char> = word.chars().collect();
            let grams = word_ngrams(&chars, 1, chars.len() + 3);
            let mut count = 0usize;
            for gram in &grams {
                if let Some(id) = self.ngrams.get(gram) {
                    ind.push(*id);
                    count += 1;
                }
            }
            if count == 0 {
                ind.push(self.unk_id);
            }
        }
        (ind, offsets)
    }

    /// Run the tensor model; returns `(stress_probs, stress_preds, yo_probs,
    /// yo_preds)` per input word.
    fn model_preds(&self, words: &[String]) -> Result<ModelPreds> {
        let w = words.len();
        let (ind, offsets) = self.model_inputs(words);
        let ind_len = ind.len();
        let ind_t = Tensor::<i64>::from_array((vec![ind_len], ind))?;
        let off_t = Tensor::<i64>::from_array((vec![w], offsets))?;
        let mut session = crate::lock_session(&self.session);
        let outputs = session.run(ort::inputs!["ind" => ind_t, "offsets" => off_t])?;
        let (stress_shape, stress_data) = outputs["stress_logits"].try_extract_tensor::<f32>()?;
        let (yo_shape, yo_data) = outputs["yo_logits"].try_extract_tensor::<f32>()?;
        let stress_dims = stress_shape.to_vec();
        let yo_dims = yo_shape.to_vec();
        if stress_dims != vec![w as i64, self.stress_dim as i64]
            || yo_dims != vec![w as i64, self.yo_dim as i64]
        {
            return Err(EngineError::Synthesis(format!(
                "accentor output shape mismatch: stress {stress_dims:?}, yo {yo_dims:?}, expected ({w}, {}) / ({w}, {})",
                self.stress_dim, self.yo_dim
            )));
        }
        let mut stress_probs = Vec::with_capacity(w);
        let mut stress_preds = Vec::with_capacity(w);
        let mut yo_probs = Vec::with_capacity(w);
        let mut yo_preds = Vec::with_capacity(w);
        for i in 0..w {
            let s = softmax(&stress_data[i * self.stress_dim..(i + 1) * self.stress_dim]);
            stress_preds.push(argmax(&s));
            stress_probs.push(s);
            let y = softmax(&yo_data[i * self.yo_dim..(i + 1) * self.yo_dim]);
            yo_preds.push(argmax(&y));
            yo_probs.push(y);
        }
        Ok((stress_probs, stress_preds, yo_probs, yo_preds))
    }

    /// Port of `AccentorNgram._get_positions`.
    fn positions(
        &self,
        word: &[char],
        stressed_vowel_ids: &[usize],
        yo_vowel_ids: &[usize],
    ) -> (Vec<usize>, Vec<usize>, usize, Option<usize>) {
        let vowel_ids: Vec<usize> = word
            .iter()
            .enumerate()
            .filter(|(_, c)| self.vowels.contains(c))
            .map(|(i, _)| i)
            .collect();
        let ye_ids: Vec<usize> = word
            .iter()
            .enumerate()
            .filter(|(_, c)| **c == 'е')
            .map(|(i, _)| i)
            .collect();
        let stress_positions: Vec<usize> = stressed_vowel_ids
            .iter()
            .filter(|idx| **idx < vowel_ids.len())
            .map(|idx| vowel_ids[*idx])
            .collect();
        let yo_positions: Vec<usize> = yo_vowel_ids
            .iter()
            .filter(|idx| **idx > 0 && *idx - 1 < ye_ids.len())
            .map(|idx| ye_ids[*idx - 1])
            .collect();
        (
            stress_positions,
            yo_positions,
            vowel_ids.len(),
            vowel_ids.first().copied(),
        )
    }

    /// Port of `AccentorNgram._accentuate_exception`.
    fn accentuate_exception(
        &self,
        clean_word: &str,
        raw_word: &str,
        have_stress: bool,
    ) -> Result<String> {
        let (exc_stress, exc_yo) = *self.exceptions.get(clean_word).ok_or_else(|| {
            EngineError::Internal(format!("exception word {clean_word:?} missing"))
        })?;
        let stress_tok = self.stress_token;
        if have_stress {
            // Positions of user '+' markers in the raw word.
            let user_positions: Vec<usize> = raw_word
                .chars()
                .enumerate()
                .filter(|(_, c)| *c == stress_tok)
                .map(|(i, _)| i)
                .collect();
            let mut word: Vec<char> = raw_word.chars().filter(|c| *c != stress_tok).collect();
            if exc_yo != -1 && user_positions.contains(&((exc_yo as usize) + 1)) {
                let yo = exc_yo as usize;
                if yo < word.len() {
                    word[yo] = if word[yo].is_lowercase() { 'ё' } else { 'Ё' };
                }
            }
            // Restore user '+' markers at their original positions.
            let mut out = String::new();
            let mut inserts: HashMap<usize, usize> = HashMap::new();
            for (n, pos) in user_positions.iter().enumerate() {
                *inserts.entry(pos - n).or_insert(0) += 1;
            }
            for (i, c) in word.iter().enumerate() {
                while let Some(n) = inserts.get_mut(&i).filter(|n| **n > 0) {
                    out.push(stress_tok);
                    *n -= 1;
                }
                out.push(*c);
            }
            while let Some(n) = inserts.get_mut(&word.len()).filter(|n| **n > 0) {
                out.push(stress_tok);
                *n -= 1;
            }
            Ok(out)
        } else {
            let mut word: Vec<char> = raw_word.chars().collect();
            if exc_yo != -1 {
                let yo = exc_yo as usize;
                if yo < word.len() {
                    word[yo] = if word[yo].is_lowercase() { 'ё' } else { 'Ё' };
                }
            }
            let mut out = String::new();
            for (i, c) in word.iter().enumerate() {
                if i == exc_stress {
                    out.push(stress_tok);
                }
                out.push(*c);
            }
            if exc_stress == word.len() {
                out.push(stress_tok);
            }
            Ok(out)
        }
    }

    /// Port of `AccentorNgram.__call__` (see upstream for the decision tree;
    /// branching order and thresholds are kept verbatim).
    #[instrument(skip_all, fields(sentence_len = sentence.len()))]
    pub fn accentuate(
        &self,
        sentence: &str,
        put_stress: bool,
        put_yo: bool,
        stress_single_vowel: bool,
        skip_stress_words: &HashSet<String>,
        skip_yo_words: &HashSet<String>,
    ) -> Result<String> {
        if !put_stress && !put_yo {
            return Ok(sentence.to_string());
        }
        let tokens = self.tokenize(sentence);
        if tokens.raw.is_empty() {
            return Ok(sentence.to_string());
        }
        let (stress_probs, stress_preds, yo_probs, yo_preds) = self.model_preds(&tokens.clean)?;

        let mut out = String::new();
        for word_idx in 0..tokens.raw.len() {
            let raw_word = &tokens.raw[word_idx];
            let clean_word = &tokens.clean[word_idx];
            if !tokens.mask[word_idx] {
                out.push_str(raw_word);
                continue;
            }
            let raw_lower: Vec<char> = raw_word.to_lowercase().chars().collect();
            let have_stress = raw_lower.contains(&self.stress_token);
            let have_yo = raw_lower.contains(&'ё');

            if have_stress && have_yo {
                out.push_str(raw_word);
                continue;
            }
            if !have_stress && have_yo && put_stress {
                let vowel_count = raw_lower.iter().filter(|c| self.vowels.contains(c)).count();
                let no_yo: String = clean_word.replace('ё', "е");
                if (vowel_count == 1 && !stress_single_vowel) || skip_stress_words.contains(&no_yo)
                {
                    out.push_str(raw_word);
                    continue;
                }
                // User-set ё: put stress on each of them.
                let yo_positions: Vec<usize> = raw_lower
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| **c == 'ё')
                    .map(|(i, _)| i)
                    .collect();
                let mut word: Vec<char> = raw_word.chars().collect();
                for (i, yo_pos) in yo_positions.iter().enumerate() {
                    word.insert(yo_pos + i, self.stress_token);
                }
                out.extend(word);
                continue;
            }

            // From here: no user-set ё.
            if self.exceptions.contains_key(clean_word.as_str()) {
                out.push_str(&self.accentuate_exception(clean_word, raw_word, have_stress)?);
                continue;
            }

            let no_yo: String = clean_word.replace('ё', "е");
            let mut stressed_vowel_ids = vec![stress_preds[word_idx]];
            let passed_stress = stress_probs[word_idx][stressed_vowel_ids[0]]
                > if put_stress {
                    self.stress_threshold
                } else {
                    1.0
                };
            let mut set_stress =
                passed_stress && !have_stress && !skip_stress_words.contains(&no_yo);

            let yo_vowel_ids = vec![yo_preds[word_idx]];
            let passed_yo =
                yo_probs[word_idx][yo_vowel_ids[0]] > if put_yo { self.yo_threshold } else { 1.0 };
            let set_yo = passed_yo && !skip_yo_words.contains(&no_yo);

            if have_stress {
                // User stress positions as cumulative vowel counts per
                // '+'-separated part (upstream splits and counts each part).
                stressed_vowel_ids = raw_word
                    .to_lowercase()
                    .split(self.stress_token)
                    .map(|part| part.chars().filter(|c| self.vowels.contains(c)).count())
                    .collect();
            }

            let (mut stress_positions, yo_positions, num_vowels, first_vowel_pos) =
                self.positions(&raw_lower, &stressed_vowel_ids, &yo_vowel_ids);
            if num_vowels == 0 {
                out.push_str(raw_word);
                continue;
            }

            let mut word: Vec<char> = raw_word.chars().collect();
            for yo_pos in &yo_positions {
                if stress_positions.contains(yo_pos) && set_yo && raw_lower[*yo_pos] == 'е' {
                    word[*yo_pos] = if word[*yo_pos].is_lowercase() {
                        'ё'
                    } else {
                        'Ё'
                    };
                }
            }

            if num_vowels == 1 {
                stress_positions = match first_vowel_pos {
                    Some(pos) => vec![pos],
                    None => vec![],
                };
                set_stress = stress_single_vowel && put_stress;
            }

            if !have_stress && set_stress {
                for (i, stress_pos) in stress_positions.iter().enumerate() {
                    word.insert(stress_pos + i, self.stress_token);
                }
            }
            out.extend(word);
        }
        debug!(output = %out, "accentor done");
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ngrams_match_upstream_shape() {
        let chars: Vec<char> = "аб".chars().collect();
        let grams = word_ngrams(&chars, 1, chars.len() + 3);
        // ext = "<аб>", lens 1..=5 (capped at ext len 4)
        let expected: Vec<&str> = vec!["<", "а", "б", ">", "<а", "аб", "б>", "<аб", "аб>", "<аб>"];
        assert_eq!(grams, expected);
    }

    #[test]
    fn ngrams_empty_word_appends_text() {
        let chars: Vec<char> = vec![];
        let grams = word_ngrams(&chars, 1, 3);
        assert_eq!(grams, vec!["<", ">", "<>", ""]);
    }

    #[test]
    fn softmax_sums_to_one() {
        let probs = softmax(&[1.0, 2.0, 3.0]);
        let sum: f32 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        assert_eq!(argmax(&probs), 2);
    }
}
