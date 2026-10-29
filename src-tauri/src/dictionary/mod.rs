//! User dictionary: user-authored pronunciation overrides that win over every
//! built-in pipeline table (change `user-dictionary`, issue #10).
//!
//! An entry maps one source token (`from`, Latin letters and digits with at
//! least one letter) to its spoken form (`to`). Matching is case-insensitive;
//! the in-memory map is keyed by the lowercased `from` while the typed case is
//! preserved for display and TOML serialization. `to` may carry one `+` before
//! a Russian vowel to fix the stress; whether that mark reaches the text is the
//! active engine's decision (change `dictionary-stress-marks`).

mod store;

pub use store::{DictionaryStore, export_to, parse_import};

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
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
    #[error("stress mark must be a single '+' immediately before a Russian vowel")]
    InvalidStress,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML serialization error: {0}")]
    Toml(#[from] toml::ser::Error),
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),
}

/// The explicit stress mark an entry's spoken form may carry: `+` immediately
/// before the stressed vowel (`М+осква`).
pub const STRESS_MARK: char = '+';

/// Russian vowels a stress mark may precede, both cases.
const STRESSED_VOWELS: [char; 20] = [
    'а', 'А', 'е', 'Е', 'ё', 'Ё', 'и', 'И', 'о', 'О', 'у', 'У', 'ы', 'Ы', 'э', 'Э', 'ю', 'Ю', 'я',
    'Я',
];

fn is_stressed_vowel(c: char) -> bool {
    STRESSED_VOWELS.contains(&c)
}

/// Split a spoken form into its text and the position (in characters) of the
/// stressed vowel. At most one mark is allowed, it must sit immediately before
/// a Russian vowel, and it never becomes part of the returned text: the mark is
/// a pronunciation hint, not something the engine reads aloud.
pub fn split_stress(to: &str) -> Result<(String, Option<usize>), DictionaryError> {
    let mut spoken = String::with_capacity(to.len());
    let mut stress = None;
    let mut chars = to.chars().peekable();
    while let Some(c) = chars.next() {
        if c != STRESS_MARK {
            spoken.push(c);
            continue;
        }
        if stress.is_some() {
            return Err(DictionaryError::InvalidStress);
        }
        match chars.peek() {
            Some(&next) if is_stressed_vowel(next) => stress = Some(spoken.chars().count()),
            _ => return Err(DictionaryError::InvalidStress),
        }
    }
    Ok((spoken, stress))
}

/// The spoken form with every stress mark removed. Entries that reach the
/// dictionary are validated, so this only ever strips a well-formed mark.
pub fn strip_stress(to: &str) -> String {
    to.chars().filter(|c| *c != STRESS_MARK).collect()
}

/// Validate `from`/`to` per the user-dictionary spec: `from` matches
/// `^[A-Za-z0-9]+$` with at least one letter (alnum tokens like "IPv6" are
/// the point; pure numbers, Cyrillic, and punctuation are not), `to` is a
/// non-empty free-form string whose spoken form is at most [`MAX_TO_LEN`]
/// characters, optionally carrying one well-formed stress mark.
pub fn validate_entry(from: &str, to: &str) -> Result<(), DictionaryError> {
    let from_ok = !from.is_empty()
        && from.len() <= MAX_FROM_LEN
        && from.bytes().all(|b| b.is_ascii_alphanumeric())
        && from.bytes().any(|b| b.is_ascii_alphabetic());
    if !from_ok {
        return Err(DictionaryError::InvalidFrom);
    }
    let (spoken, _) = split_stress(to)?;
    if spoken.is_empty() || spoken.chars().count() > MAX_TO_LEN {
        return Err(DictionaryError::InvalidTo);
    }
    Ok(())
}

/// The in-memory dictionary: entries keyed by the lowercased `from`, so
/// iteration is sorted by key and one-word-one-entry holds by construction.
#[derive(Debug, Default, Clone)]
pub struct UserDictionary {
    entries: BTreeMap<String, DictionaryEntry>,
    /// Whether an explicit stress mark survives into the text: the active
    /// profile's `Stress` decides, and the pipeline re-applies it every run.
    mark_stress: bool,
}

impl UserDictionary {
    /// Insert (or replace) an already-validated entry. Returns `true` when an
    /// entry with the same key existed (updated) and `false` when added.
    pub fn insert(&mut self, entry: DictionaryEntry) -> bool {
        self.entries.insert(entry.key(), entry).is_some()
    }

    /// The stored replacement exactly as typed, stress mark included. Read it
    /// for display and persistence; a site that inserts text into the pipeline
    /// must use [`Self::resolve`], which applies the profile's stress decision.
    pub fn get(&self, word_lower: &str) -> Option<&str> {
        self.entries.get(word_lower).map(|e| e.to.as_str())
    }

    /// The text to insert for a lowercased word, if the dictionary has an
    /// entry: the stored `to` verbatim when the active engine renders stress
    /// marks, its spoken form (marks removed) otherwise.
    pub fn resolve(&self, word_lower: &str) -> Option<Cow<'_, str>> {
        self.entries.get(word_lower).map(|entry| {
            if self.mark_stress {
                Cow::Borrowed(entry.to.as_str())
            } else {
                Cow::Owned(strip_stress(&entry.to))
            }
        })
    }

    /// Keep explicit stress marks in replacements (the active profile declares
    /// stress support). Write-side of the `resolve` decision; the pipeline sets
    /// it from the profile at the start of every normalization.
    pub fn set_mark_stress(&mut self, mark: bool) {
        self.mark_stress = mark;
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
