//! User dictionary: user-authored pronunciation overrides that win over every
//! built-in pipeline table (change `user-dictionary`, issue #10).
//!
//! An entry maps one source token (`from`, one word of Latin or Russian
//! Cyrillic letters and digits, one script) to its spoken form (`to`).
//! Matching is case-insensitive and exact;
//! the in-memory map is keyed by the lowercased `from` while the typed case is
//! preserved for display and TOML serialization. `to` may carry one `+` before
//! a Russian vowel to fix the stress; whether that mark reaches the text is the
//! active engine's decision (change `dictionary-stress-marks`).

mod store;

pub use store::{DictionaryStore, export_to, parse_import};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Longest accepted `from`, in characters.
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
        "source word must be a single token of Latin or Russian letters and digits, one script, at least one letter, max {MAX_FROM_LEN} chars"
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

/// The spoken form of `to` with the stress mark removed, or a validation error
/// when the mark is malformed. At most one mark is allowed and it must sit
/// immediately before a Russian vowel; the mark itself never becomes part of
/// the returned text — it is a pronunciation hint, not something the engine
/// reads aloud.
pub fn spoken_form(to: &str) -> Result<String, DictionaryError> {
    let mut spoken = String::with_capacity(to.len());
    let mut stressed = false;
    let mut chars = to.chars().peekable();
    while let Some(c) = chars.next() {
        if c != STRESS_MARK {
            spoken.push(c);
            continue;
        }
        if stressed {
            return Err(DictionaryError::InvalidStress);
        }
        match chars.peek() {
            Some(&next) if is_stressed_vowel(next) => stressed = true,
            _ => return Err(DictionaryError::InvalidStress),
        }
    }
    Ok(spoken)
}

/// The spoken form with every stress mark removed. Entries that reach the
/// dictionary are validated, so this only ever strips a well-formed mark.
fn strip_stress(to: &str) -> String {
    to.chars().filter(|c| *c != STRESS_MARK).collect()
}

/// True for a Russian Cyrillic letter in either case. The rest of the Cyrillic
/// block (Ukrainian, Serbian, …) is rejected on purpose: the pipeline narrates
/// Russian, so an entry in another script would match text it cannot handle.
fn is_russian_letter(c: char) -> bool {
    matches!(c, 'а'..='я' | 'А'..='Я' | 'ё' | 'Ё')
}

/// The script a dictionary key is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyScript {
    Latin,
    Cyrillic,
}

/// The script `from` uses, or `None` when it is not a single valid token: one
/// word of letters and digits with at least one letter, all letters from one
/// script, at most [`MAX_FROM_LEN`] characters.
pub fn key_script(from: &str) -> Option<KeyScript> {
    if from.is_empty() || from.chars().count() > MAX_FROM_LEN {
        return None;
    }
    let mut script = None;
    for c in from.chars() {
        if c.is_ascii_digit() {
            continue;
        }
        let this = if c.is_ascii_alphabetic() {
            KeyScript::Latin
        } else if is_russian_letter(c) {
            KeyScript::Cyrillic
        } else {
            return None;
        };
        match script {
            None => script = Some(this),
            Some(seen) if seen == this => {}
            Some(_) => return None,
        }
    }
    script
}

/// Validate `from`/`to` per the user-dictionary spec: `from` is one token of
/// Latin or Russian Cyrillic letters and digits with at least one letter (alnum
/// tokens like "IPv6" and Russian words like "замок" are the point; pure
/// numbers, punctuation, and mixed-script tokens are not), `to` is a non-empty
/// free-form string whose spoken form is at most [`MAX_TO_LEN`] characters,
/// optionally carrying one well-formed stress mark.
pub fn validate_entry(from: &str, to: &str) -> Result<(), DictionaryError> {
    if key_script(from).is_none() {
        return Err(DictionaryError::InvalidFrom);
    }
    let spoken = spoken_form(to)?;
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
    /// entry. `mark_stress` is the active engine's decision: when it is true the
    /// stored `to` is returned as typed, mark included; when it is false the mark
    /// is removed first. Every caller turns the result into the replacement text
    /// of a match, so the value is owned rather than borrowed.
    pub fn resolve(&self, word_lower: &str, mark_stress: bool) -> Option<String> {
        self.entries.get(word_lower).map(|entry| {
            if mark_stress || !entry.to.contains(STRESS_MARK) {
                entry.to.clone()
            } else {
                strip_stress(&entry.to)
            }
        })
    }

    pub fn contains(&self, word_lower: &str) -> bool {
        self.entries.contains_key(word_lower)
    }

    /// True when at least one key is written in `script`. The pre-pass uses it
    /// to skip a candidate scan for a script the dictionary cannot match;
    /// dictionaries are small, so a pass over the keys per normalization is
    /// cheaper than keeping script counters in sync with every mutation.
    pub fn has_keys_in(&self, script: KeyScript) -> bool {
        self.entries
            .keys()
            .any(|key| key_script(key) == Some(script))
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
