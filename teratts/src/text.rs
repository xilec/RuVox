//! Russian text frontend: language tags, RUAccent dictionary stress, and the
//! codepoint gate.
//!
//! The engine is deliberately not a normalizer. The shared pipeline has already
//! expanded numbers, units and Latin fragments according to the text profile;
//! this module only does what is specific to TeraTTSv2: wrap the text in the
//! `<ru>` tag the model requires, apply the deterministic RUAccent dictionary
//! (`ё` words and `+` stress marks), normalize to NFKD, and drop codepoints the
//! model's 65 536-entry alphabet does not contain — with one counted warning,
//! never silently.

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::LazyLock;

use flate2::read::GzDecoder;
use regex::Regex;
use tracing::warn;
use unicode_normalization::UnicodeNormalization;

use crate::error::{Error, Result};
use crate::indexer::Indexer;

static WORD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-zА-Яа-яЁё+]+").expect("word regex"));
/// Characters that may not appear in the input: the adapter owns the only tags.
const MARKUP: [char; 2] = ['<', '>'];

/// Deterministic RUAccent dictionary mode.
///
/// Loads the four shipped dictionaries but only `accents` and `yo_words` are
/// consulted; `omographs` and `yo_homographs` belong to the neural full mode
/// that this engine does not ship.
pub struct Accentizer {
    accents: HashMap<String, String>,
    yo_words: HashMap<String, String>,
    /// Loaded for format completeness; the dictionary mode never consults
    /// them (they belong to the neural full mode this engine does not ship).
    _omographs: HashMap<String, Vec<String>>,
    _yo_homographs: HashMap<String, String>,
}

impl Accentizer {
    /// Load the dictionaries from the bundle's `ruaccent/dictionary` directory.
    pub fn load(directory: &Path) -> Result<Self> {
        Ok(Self {
            accents: load_json_gz(&directory.join("accents.json.gz"))?,
            yo_words: load_json_gz(&directory.join("yo_words.json.gz"))?,
            _omographs: load_json_gz(&directory.join("omographs.json.gz"))?,
            _yo_homographs: load_json_gz(&directory.join("yo_homographs.json.gz"))?,
        })
    }

    /// Apply dictionary stress to every word of `text`, preserving casing and
    /// any explicit `+` the caller already put in a word.
    pub fn accentuate(&self, text: &str) -> String {
        WORD_RE
            .replace_all(text, |captures: &regex::Captures<'_>| {
                self.word(&captures[0])
            })
            .into_owned()
    }

    fn word(&self, word: &str) -> String {
        // An explicit mark is authoritative: never add a second one.
        if word.contains('+') {
            return word.to_string();
        }
        let lowered = word.to_lowercase();
        let normalized = match self.yo_words.get(&lowered) {
            Some(yo) => fix_capital(word, yo),
            None => word.to_string(),
        };
        let normalized_lower = normalized.to_lowercase();
        let Some(accented) = self.accents.get(&normalized_lower) else {
            return normalized;
        };
        if accented == &normalized_lower {
            return normalized;
        }
        // Transfer the dictionary marker positions to the source spelling.
        let mut target: Vec<char> = normalized.chars().collect();
        let mut inserted = 0;
        for (index, character) in accented.chars().enumerate() {
            if character == '+' {
                let position = index + inserted;
                if position <= target.len() {
                    target.insert(position, '+');
                    inserted += 1;
                }
            }
        }
        target.into_iter().collect()
    }
}

/// Copy the source word's capitalisation onto the dictionary spelling.
///
/// Returns the dictionary spelling unchanged when the lengths differ: the
/// per-character mapping is only meaningful for equal-length strings.
fn fix_capital(source: &str, target: &str) -> String {
    let source: Vec<char> = source.chars().collect();
    let target: Vec<char> = target.chars().collect();
    if source.len() != target.len() {
        return target.into_iter().collect();
    }
    source
        .iter()
        .zip(target.iter())
        .map(|(from, to)| {
            if from.is_uppercase() {
                to.to_uppercase().to_string()
            } else {
                to.to_lowercase().to_string()
            }
        })
        .collect()
}

fn load_json_gz<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let file = File::open(path)
        .map_err(|e| Error::BundleCorrupt(format!("cannot read {}: {e}", path.display())))?;
    serde_json::from_reader(GzDecoder::new(file))
        .map_err(|e| Error::BundleCorrupt(format!("{}: {e}", path.display())))
}

/// Text ready for the indexer, plus what the alphabet rejected.
#[derive(Debug, Clone)]
pub struct PreparedText {
    /// Tagged, accented, NFKD-normalized text containing only supported
    /// codepoints.
    pub text: String,
    /// Characters dropped by the gate, in order of appearance.
    pub dropped: Vec<char>,
}

/// Validate that the input carries no markup of its own.
///
/// The adapter is the only place that emits `<ru>` tags. A stray `<` or `>`
/// would either be read as a broken tag by the model or silently dropped; both
/// are worse than a clear error the user can act on.
pub fn validate_no_markup(text: &str) -> Result<()> {
    if let Some((index, character)) = text
        .char_indices()
        .find(|(_, character)| MARKUP.contains(character))
    {
        return Err(Error::InvalidText(format!(
            "unexpected markup character {character:?} at byte {index}; \
             language tags are added by the engine, not by the caller"
        )));
    }
    Ok(())
}

/// Wrap, accentuate, normalize and gate a Russian text.
pub fn prepare(text: &str, accentizer: &Accentizer, indexer: &Indexer) -> Result<PreparedText> {
    validate_no_markup(text)?;
    let text = text.trim();
    if text.is_empty() {
        return Err(Error::InvalidText("empty text".into()));
    }
    let accented = accentizer.accentuate(text);
    let wrapped = format!("<ru>{accented}</ru>");
    let normalized: String = wrapped.nfkd().collect();

    let mut kept = String::with_capacity(normalized.len());
    let mut dropped = Vec::new();
    for character in normalized.chars() {
        if indexer.supports(character) {
            kept.push(character);
        } else {
            dropped.push(character);
        }
    }

    let content = kept
        .strip_prefix("<ru>")
        .and_then(|rest| rest.strip_suffix("</ru>"))
        .unwrap_or("");
    if content.trim().is_empty() {
        return Err(Error::InvalidText(
            "text contains no characters supported by the model".into(),
        ));
    }
    if kept.is_empty() {
        return Err(Error::InvalidText("text produced no tokens".into()));
    }

    if !dropped.is_empty() {
        let total = dropped.len();
        let mut unique: Vec<char> = dropped.clone();
        unique.sort_unstable();
        unique.dedup();
        let shown: Vec<String> = unique
            .iter()
            .take(16)
            .map(|character| format!("{character:?} (U+{:04X})", *character as u32))
            .collect();
        warn!(
            count = total,
            characters = %shown.join(", "),
            "dropped characters absent from the TeraTTS alphabet"
        );
        let digits: Vec<char> = unique
            .iter()
            .copied()
            .filter(char::is_ascii_digit)
            .collect();
        if !digits.is_empty() {
            warn!(
                digits = %digits.iter().collect::<String>(),
                "normalization gap: digits reached the TeraTTS frontend and were dropped; \
                 the pipeline owns number expansion"
            );
        }
    }

    Ok(PreparedText {
        text: kept,
        dropped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    use flate2::Compression;
    use flate2::write::GzEncoder;

    fn write_dict(path: &Path, entries: &[(&str, &str)]) {
        let map: HashMap<&str, &str> = entries.iter().copied().collect();
        let file = File::create(path).unwrap();
        let mut encoder = GzEncoder::new(file, Compression::fast());
        encoder
            .write_all(serde_json::to_string(&map).unwrap().as_bytes())
            .unwrap();
        encoder.finish().unwrap();
    }

    fn accentizer(dir: &Path) -> Accentizer {
        write_dict(
            &dir.join("accents.json.gz"),
            &[("молоко", "молок+о"), ("привет", "прив+ет"), ("о", "+о")],
        );
        write_dict(
            &dir.join("yo_words.json.gz"),
            &[("все", "всё"), ("елка", "ёлка")],
        );
        write_dict(&dir.join("omographs.json.gz"), &[]);
        write_dict(&dir.join("yo_homographs.json.gz"), &[]);
        Accentizer::load(dir).unwrap()
    }

    fn indexer() -> Indexer {
        let mut table = vec![-1i32; crate::indexer::TABLE_LEN];
        for (code, id) in [
            ('<', 1),
            ('>', 2),
            ('/', 3),
            ('r', 4),
            ('u', 5),
            ('а', 10),
            ('б', 11),
            ('в', 12),
            ('г', 13),
            ('д', 14),
            ('е', 15),
            ('ж', 16),
            ('з', 17),
            ('и', 18),
            ('й', 19),
            ('к', 20),
            ('л', 21),
            ('м', 22),
            ('н', 23),
            ('о', 24),
            ('п', 25),
            ('р', 26),
            ('с', 27),
            ('т', 28),
            ('у', 29),
            ('ф', 30),
            ('х', 31),
            ('ц', 32),
            ('ч', 33),
            ('ш', 34),
            ('щ', 35),
            ('ъ', 36),
            ('ы', 37),
            ('ь', 38),
            ('э', 39),
            ('ю', 40),
            ('я', 41),
            ('ё', 42),
            (' ', 43),
            ('+', 44),
            (',', 45),
            ('.', 46),
            ('!', 47),
            ('?', 48),
            ('-', 49),
            ('\u{0306}', 50),
            ('\u{0308}', 51),
            ('М', 60),
        ] {
            table[code as usize] = id;
        }
        Indexer::from_table(table)
    }

    #[test]
    fn wraps_and_accentuates_a_dictionary_word() {
        let tmp = tempfile::tempdir().unwrap();
        let prepared = prepare("молоко", &accentizer(tmp.path()), &indexer()).unwrap();
        assert_eq!(prepared.text, "<ru>молок+о</ru>");
    }

    #[test]
    fn explicit_stress_wins() {
        let tmp = tempfile::tempdir().unwrap();
        let prepared = prepare("молок+о", &accentizer(tmp.path()), &indexer()).unwrap();
        assert_eq!(prepared.text, "<ru>молок+о</ru>");
        assert_eq!(prepared.text.matches('+').count(), 1);
    }

    #[test]
    fn unknown_words_are_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let prepared = prepare("кот", &accentizer(tmp.path()), &indexer()).unwrap();
        assert_eq!(prepared.text, "<ru>кот</ru>");
    }

    #[test]
    fn yo_words_are_applied() {
        let tmp = tempfile::tempdir().unwrap();
        let prepared = prepare("елка", &accentizer(tmp.path()), &indexer()).unwrap();
        let expected: String = "<ru>ёлка</ru>".nfkd().collect();
        assert_eq!(prepared.text, expected);
    }

    #[test]
    fn casing_is_preserved() {
        let tmp = tempfile::tempdir().unwrap();
        let prepared = prepare("Молоко", &accentizer(tmp.path()), &indexer()).unwrap();
        assert_eq!(prepared.text, "<ru>Молок+о</ru>");
    }

    #[test]
    fn unsupported_characters_are_dropped_with_a_warning() {
        let tmp = tempfile::tempdir().unwrap();
        let prepared = prepare("молоко — привет", &accentizer(tmp.path()), &indexer()).unwrap();
        assert!(!prepared.text.contains('—'));
        assert_eq!(prepared.dropped, vec!['—']);
        assert!(prepared.text.contains("молок+о"));
    }

    #[test]
    fn digits_are_reported_as_a_normalization_gap() {
        let tmp = tempfile::tempdir().unwrap();
        let prepared = prepare("молоко 5", &accentizer(tmp.path()), &indexer()).unwrap();
        assert_eq!(prepared.dropped, vec!['5']);
    }

    #[test]
    fn markup_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let accentizer = accentizer(tmp.path());
        let indexer = indexer();
        assert!(prepare("привет < мир", &accentizer, &indexer).is_err());
        assert!(prepare("<ru>привет</ru>", &accentizer, &indexer).is_err());
    }

    #[test]
    fn empty_text_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(prepare("   ", &accentizer(tmp.path()), &indexer()).is_err());
    }

    #[test]
    fn text_of_only_unsupported_characters_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(prepare("—", &accentizer(tmp.path()), &indexer()).is_err());
    }
}
