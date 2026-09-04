//! User dictionary: user-authored pronunciation overrides that win over every
//! built-in pipeline table (change `user-dictionary`, issue #10).
//!
//! An entry maps one source token (`from`, Latin letters and digits with at
//! least one letter) to its spoken form (`to`). Matching is case-insensitive;
//! the in-memory map is keyed by the lowercased `from` while the typed case is
//! preserved for display and TOML serialization.

mod store;

pub use store::{DictionaryStore, export_to, parse_import};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Longest accepted `from` (ASCII, so chars == bytes).
pub const MAX_FROM_LEN: usize = 64;
/// Longest accepted `to`, in characters.
pub const MAX_TO_LEN: usize = 256;

/// A single user-dictionary mapping: source token as typed and its spoken
/// form. The unit of CRUD, import, and export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub from: String,
    pub to: String,
}

impl DictionaryEntry {
    /// The map key for this entry: the lowercased `from`.
    pub fn key(&self) -> String {
        self.from.to_lowercase()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DictionaryError {
    #[error(
        "source word must be a single Latin token (letters and digits, at least one letter, max {MAX_FROM_LEN} chars)"
    )]
    InvalidFrom,
    #[error("spoken form must be non-empty and at most {MAX_TO_LEN} characters")]
    InvalidTo,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML serialization error: {0}")]
    Toml(#[from] toml::ser::Error),
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),
}

/// Validate `from`/`to` per the user-dictionary spec: `from` matches
/// `^[A-Za-z0-9]+$` with at least one letter (alnum tokens like "IPv6" are
/// the point; pure numbers, Cyrillic, and punctuation are not), `to` is a
/// non-empty free-form string of at most [`MAX_TO_LEN`] characters.
pub fn validate_entry(from: &str, to: &str) -> Result<(), DictionaryError> {
    let from_ok = !from.is_empty()
        && from.len() <= MAX_FROM_LEN
        && from.bytes().all(|b| b.is_ascii_alphanumeric())
        && from.bytes().any(|b| b.is_ascii_alphabetic());
    if !from_ok {
        return Err(DictionaryError::InvalidFrom);
    }
    if to.is_empty() || to.chars().count() > MAX_TO_LEN {
        return Err(DictionaryError::InvalidTo);
    }
    Ok(())
}

/// The in-memory dictionary: entries keyed by the lowercased `from`, so
/// iteration is sorted by key and one-word-one-entry holds by construction.
#[derive(Debug, Default, Clone)]
pub struct UserDictionary {
    entries: BTreeMap<String, DictionaryEntry>,
}

impl UserDictionary {
    /// Insert (or replace) an already-validated entry. Returns `true` when an
    /// entry with the same key existed (updated) and `false` when added.
    pub fn insert(&mut self, entry: DictionaryEntry) -> bool {
        self.entries.insert(entry.key(), entry).is_some()
    }

    /// Spoken form for a lowercased word, if the dictionary has an entry.
    pub fn get(&self, word_lower: &str) -> Option<&str> {
        self.entries.get(word_lower).map(|e| e.to.as_str())
    }

    pub fn contains(&self, word_lower: &str) -> bool {
        self.entries.contains_key(word_lower)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Entries sorted by the lowercased `from`.
    pub fn iter(&self) -> impl Iterator<Item = &DictionaryEntry> {
        self.entries.values()
    }

    /// Drop everything and insert the given already-validated entries.
    pub fn replace_all(&mut self, entries: impl IntoIterator<Item = DictionaryEntry>) {
        self.entries.clear();
        for entry in entries {
            self.entries.insert(entry.key(), entry);
        }
    }
}

#[cfg(test)]
mod tests;
