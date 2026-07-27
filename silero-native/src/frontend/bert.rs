//! BERT tokenizer for the HomoSolver: a port of
//! `custom_tokenizers/bert_tokenizer.py` (`BasicTokenizer` +
//! `WordpieceTokenizer` + `SimpleBertTokenizer.encode`) from the upstream
//! package. No lowercasing and no accent stripping — the upstream
//! `SimpleBertTokenizer` does neither.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use unicode_categories::UnicodeCategories;

use crate::error::{EngineError, Result};

/// CJK ranges from `_is_chinese_char`.
fn is_chinese_char(cp: u32) -> bool {
    (0x4E00..=0x9FFF).contains(&cp)
        || (0x3400..=0x4DBF).contains(&cp)
        || (0x20000..=0x2A6DF).contains(&cp)
        || (0x2A700..=0x2B73F).contains(&cp)
        || (0x2B740..=0x2B81F).contains(&cp)
        || (0x2B820..=0x2CEAF).contains(&cp)
        || (0xF900..=0xFAFF).contains(&cp)
        || (0x2F800..=0x2FA1F).contains(&cp)
}

/// `_is_whitespace`: space, \t, \n, \r or Unicode category Zs.
fn is_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r') || c.is_separator_space()
}

/// `_is_control`: Unicode category C* except \t \n \r.
fn is_control(c: char) -> bool {
    if matches!(c, '\t' | '\n' | '\r') {
        return false;
    }
    c.is_other()
}

/// `_is_punctuation`: ASCII punctuation ranges or Unicode category P*.
fn is_punctuation(c: char) -> bool {
    let cp = c as u32;
    if (33..=47).contains(&cp)
        || (58..=64).contains(&cp)
        || (91..=96).contains(&cp)
        || (123..=126).contains(&cp)
    {
        return true;
    }
    c.is_punctuation()
}

/// Port of `BasicTokenizer`.
pub struct BasicTokenizer {
    never_split: HashSet<String>,
    tokenize_chinese_chars: bool,
}

impl BasicTokenizer {
    pub fn new(never_split: HashSet<String>, tokenize_chinese_chars: bool) -> Self {
        Self {
            never_split,
            tokenize_chinese_chars,
        }
    }

    pub fn tokenize(&self, text: &str) -> Vec<String> {
        let text = self.clean_text(text);
        let text = if self.tokenize_chinese_chars {
            tokenize_chinese_chars(&text)
        } else {
            text
        };
        let mut split_tokens = Vec::new();
        for token in text.split_whitespace() {
            if self.never_split.contains(token) {
                split_tokens.push(token.to_string());
            } else {
                split_tokens.extend(self.split_on_punc(token));
            }
        }
        split_tokens
    }

    fn clean_text(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        for c in text.chars() {
            let cp = c as u32;
            if cp == 0 || cp == 0xfffd || is_control(c) {
                continue;
            }
            if is_whitespace(c) {
                out.push(' ');
            } else {
                out.push(c);
            }
        }
        out
    }

    fn split_on_punc(&self, token: &str) -> Vec<String> {
        if self.never_split.contains(token) {
            return vec![token.to_string()];
        }
        let mut output: Vec<String> = Vec::new();
        let mut start_new_word = true;
        for c in token.chars() {
            if is_punctuation(c) {
                output.push(c.to_string());
                start_new_word = true;
            } else {
                if start_new_word {
                    output.push(String::new());
                }
                start_new_word = false;
                if let Some(last) = output.last_mut() {
                    last.push(c);
                }
            }
        }
        output
    }
}

fn tokenize_chinese_chars(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if is_chinese_char(c as u32) {
            out.push(' ');
            out.push(c);
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

/// Port of `WordpieceTokenizer` (greedy longest-match-first).
pub struct WordpieceTokenizer<'a> {
    vocab: &'a HashMap<String, i64>,
    unk_token: &'a str,
    max_input_chars_per_word: usize,
}

impl<'a> WordpieceTokenizer<'a> {
    pub fn new(vocab: &'a HashMap<String, i64>, unk_token: &'a str) -> Self {
        Self {
            vocab,
            unk_token,
            max_input_chars_per_word: 100,
        }
    }

    pub fn tokenize(&self, text: &str) -> Vec<String> {
        let mut output_tokens = Vec::new();
        for token in text.split_whitespace() {
            let chars: Vec<char> = token.chars().collect();
            if chars.len() > self.max_input_chars_per_word {
                output_tokens.push(self.unk_token.to_string());
                continue;
            }
            let mut is_bad = false;
            let mut start = 0usize;
            let mut sub_tokens = Vec::new();
            while start < chars.len() {
                let mut end = chars.len();
                let mut cur_substr: Option<String> = None;
                while start < end {
                    let mut substr: String = chars[start..end].iter().collect();
                    if start > 0 {
                        substr = format!("##{substr}");
                    }
                    if self.vocab.contains_key(&substr) {
                        cur_substr = Some(substr);
                        break;
                    }
                    end -= 1;
                }
                match cur_substr {
                    Some(substr) => {
                        sub_tokens.push(substr);
                        start = end;
                    }
                    None => {
                        is_bad = true;
                        break;
                    }
                }
            }
            if is_bad {
                output_tokens.push(self.unk_token.to_string());
            } else {
                output_tokens.extend(sub_tokens);
            }
        }
        output_tokens
    }
}

/// Port of `SimpleBertTokenizer.encode` (Basic → WordPiece → [CLS] … [SEP]).
pub struct BertTokenizer {
    vocab: HashMap<String, i64>,
    basic: BasicTokenizer,
    pub unk_token_id: i64,
    pub sep_token_id: i64,
    pub pad_token_id: i64,
    pub cls_token_id: i64,
    pub homo_start_id: i64,
    pub homo_end_id: i64,
}

impl BertTokenizer {
    /// Load `vocab.txt` (id = line number); special token ids come from
    /// `frontend.json.homosolver` rather than `special_tokens_map.json`
    /// (the bundle does not ship the tokenizer config files).
    pub fn load(
        vocab_path: &Path,
        never_split: HashSet<String>,
        ids: &super::HomosolverConfig,
    ) -> Result<Self> {
        let text = std::fs::read_to_string(vocab_path).map_err(|e| {
            EngineError::Bundle(format!("cannot read {}: {e}", vocab_path.display()))
        })?;
        let vocab: HashMap<String, i64> = text
            .lines()
            .enumerate()
            .map(|(i, token)| (token.to_string(), i as i64))
            .collect();
        Ok(Self {
            vocab,
            basic: BasicTokenizer::new(never_split, true),
            unk_token_id: ids.unk_token_id,
            sep_token_id: ids.sep_token_id,
            pad_token_id: ids.pad_token_id,
            cls_token_id: ids.cls_token_id,
            homo_start_id: ids.homo_start_id,
            homo_end_id: ids.homo_end_id,
        })
    }

    /// `tokenize` = BasicTokenizer → WordpieceTokenizer.
    pub fn tokenize(&self, text: &str) -> Vec<String> {
        let basic_tokens = self.basic.tokenize(text);
        let wordpiece = WordpieceTokenizer::new(&self.vocab, "[UNK]");
        wordpiece.tokenize(&basic_tokens.join(" "))
    }

    /// `encode` = tokenize + convert to ids, wrapped in [CLS]/[SEP].
    pub fn encode(&self, text: &str) -> Vec<i64> {
        let tokens = self.tokenize(text);
        let mut ids = Vec::with_capacity(tokens.len() + 2);
        ids.push(self.cls_token_id);
        for token in &tokens {
            ids.push(self.vocab.get(token).copied().unwrap_or(self.unk_token_id));
        }
        ids.push(self.sep_token_id);
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vocab(pairs: &[(&str, i64)]) -> HashMap<String, i64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn basic_splits_punctuation_and_keeps_never_split() {
        let tok = BasicTokenizer::new(["[HOMO]".to_string()].into_iter().collect(), true);
        let out = tok.tokenize("открой [HOMO] замок, пожалуйста!");
        assert_eq!(
            out,
            vec!["открой", "[HOMO]", "замок", ",", "пожалуйста", "!"]
        );
    }

    #[test]
    fn basic_cleans_control_and_whitespace() {
        let tok = BasicTokenizer::new(HashSet::new(), true);
        let out = tok.tokenize("а\u{0000}\u{0007}б\u{00A0}в");
        assert_eq!(out, vec!["аб", "в"]);
    }

    #[test]
    fn wordpiece_greedy_longest_match() {
        let v = vocab(&[
            ("за", 1),
            ("мок", 2),
            ("##мок", 3),
            ("замок", 4),
            ("[UNK]", 0),
        ]);
        let wp = WordpieceTokenizer::new(&v, "[UNK]");
        assert_eq!(wp.tokenize("замок"), vec!["замок"]);
        assert_eq!(wp.tokenize("замоки"), vec!["[UNK]"]);
        let v2 = vocab(&[("за", 1), ("##мок", 3), ("[UNK]", 0)]);
        let wp2 = WordpieceTokenizer::new(&v2, "[UNK]");
        assert_eq!(wp2.tokenize("замок"), vec!["за", "##мок"]);
    }

    #[test]
    fn basic_keeps_mixed_cyrillic_latin_digits_and_case() {
        let tok = BasicTokenizer::new(HashSet::new(), true);
        let out = tok.tokenize("запусти getUserData123 версии v2.1!");
        assert_eq!(
            out,
            vec!["запусти", "getUserData123", "версии", "v2", ".", "1", "!"]
        );
    }

    #[test]
    fn basic_never_split_keeps_punctuated_token_whole() {
        let tok = BasicTokenizer::new(["[/HOMO]".to_string()].into_iter().collect(), true);
        let out = tok.tokenize("текст [/HOMO] дальше");
        assert_eq!(out, vec!["текст", "[/HOMO]", "дальше"]);
    }

    #[test]
    fn wordpiece_overlong_word_becomes_unk() {
        let v = vocab(&[("а", 1), ("[UNK]", 0)]);
        let wp = WordpieceTokenizer::new(&v, "[UNK]");
        let long_word = "а".repeat(101);
        assert_eq!(wp.tokenize(&long_word), vec!["[UNK]"]);
    }
}
