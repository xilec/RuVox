//! Character indexer: the model's 65 536-entry codepoint table.
//!
//! The model is trained on a fixed alphabet; every character maps to an id or
//! is unsupported. The table is indexed by codepoint (BMP only — the table is
//! 65 536 entries), so non-BMP characters are unsupported by construction.
//! The text frontend applies NFKD before encoding and logs every dropped
//! character, because a silent drop changes what the model says.

use std::fs;
use std::path::Path;

use crate::error::{Error, Result};

/// Number of codepoints the table covers.
pub const TABLE_LEN: usize = 65_536;

/// Parsed `unicode_indexer.json`.
#[derive(Debug, Clone)]
pub struct Indexer {
    table: Vec<i32>,
}

/// Encoded text plus what the alphabet rejected.
#[derive(Debug, Clone, Default)]
pub struct Encoded {
    /// Token ids, one per kept character.
    pub ids: Vec<i64>,
    /// Attention mask, `1.0` per kept character.
    pub mask: Vec<f32>,
    /// Dropped characters in order of appearance (repeats included).
    pub dropped: Vec<char>,
}

impl Indexer {
    /// Build an indexer from an explicit table (tests only).
    #[cfg(test)]
    pub(crate) fn from_table(table: Vec<i32>) -> Self {
        assert_eq!(table.len(), TABLE_LEN);
        Self { table }
    }

    /// Load and validate the table.
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)
            .map_err(|e| Error::BundleCorrupt(format!("cannot read {}: {e}", path.display())))?;
        let table: Vec<i32> = serde_json::from_slice(&bytes)
            .map_err(|e| Error::BundleCorrupt(format!("{}: {e}", path.display())))?;
        if table.len() != TABLE_LEN {
            return Err(Error::BundleCorrupt(format!(
                "{}: expected {TABLE_LEN} entries, got {}",
                path.display(),
                table.len()
            )));
        }
        Ok(Self { table })
    }

    /// Whether the model has an id for this codepoint.
    pub fn supports(&self, c: char) -> bool {
        let code = c as usize;
        code < self.table.len() && self.table[code] >= 0
    }

    /// Encode text that is already NFKD-normalized.
    pub fn encode(&self, text: &str) -> Encoded {
        let mut encoded = Encoded::default();
        for c in text.chars() {
            let code = c as usize;
            if code < self.table.len() && self.table[code] >= 0 {
                encoded.ids.push(self.table[code] as i64);
                encoded.mask.push(1.0);
            } else {
                encoded.dropped.push(c);
            }
        }
        encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indexer() -> Indexer {
        let mut table = vec![-1i32; TABLE_LEN];
        for (code, id) in [('а', 10), ('б', 11), (' ', 1), ('-', 9), ('+', 7)] {
            table[code as usize] = id;
        }
        Indexer { table }
    }

    #[test]
    fn encodes_supported_characters_in_order() {
        let encoded = indexer().encode("аб а");
        assert_eq!(encoded.ids, vec![10, 11, 1, 10]);
        assert_eq!(encoded.mask, vec![1.0, 1.0, 1.0, 1.0]);
        assert!(encoded.dropped.is_empty());
    }

    #[test]
    fn reports_unsupported_characters() {
        let encoded = indexer().encode("а—б");
        assert_eq!(encoded.ids, vec![10, 11]);
        assert_eq!(encoded.dropped, vec!['—']);
    }

    #[test]
    fn non_bmp_and_digits_are_unsupported() {
        let encoded = indexer().encode("а🙂5");
        assert_eq!(encoded.ids, vec![10]);
        assert_eq!(encoded.dropped, vec!['🙂', '5']);
    }

    #[test]
    fn empty_input_encodes_to_nothing() {
        let encoded = indexer().encode("");
        assert!(encoded.ids.is_empty());
        assert!(encoded.dropped.is_empty());
    }
}
