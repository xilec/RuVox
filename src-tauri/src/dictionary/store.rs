//! TOML persistence for the user dictionary (config root, alongside
//! `config.json`). The file is the source of truth at startup; missing file
//! means an empty dictionary, a corrupted file is backed up and replaced by
//! an empty one (the `config.json` recovery pattern).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{DictionaryEntry, DictionaryError, UserDictionary};

/// File-format version. A file with a different version is treated as
/// foreign: loaded as empty and never backed up (a newer app's dictionary
/// must not be destroyed by an older build reading it).
const FORMAT_VERSION: u32 = 1;

/// Raw file shape for parsing. Both fields optional so hand-minimal files
/// (`[entries]` only, no `version`) parse leniently. Entries are a
/// [`toml::Table`] — `Map<String, String>` has no serde impls in toml 0.8 —
/// and each value is either a string or a table with `to`/`forms`.
#[derive(Deserialize)]
struct DictionaryFileRaw {
    version: Option<u32>,
    entries: Option<toml::Table>,
}

#[derive(Serialize)]
struct DictionaryFileOut {
    version: u32,
    entries: toml::Table,
}

/// Owns the canonical dictionary file path and its load/save.
pub struct DictionaryStore {
    path: PathBuf,
}

impl DictionaryStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load the dictionary. Infallible: missing file → empty; unreadable or
    /// unparsable file → backup as `.bak` + empty + warning; entries failing
    /// validation are skipped individually (a warning per entry).
    pub fn load(&self) -> UserDictionary {
        if !self.path.exists() {
            return UserDictionary::default();
        }
        let raw = match fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(e) => {
                tracing::warn!("failed to read {}: {e}", self.path.display());
                return UserDictionary::default();
            }
        };
        let file: DictionaryFileRaw = match toml::from_str(&raw) {
            Ok(file) => file,
            Err(e) => {
                tracing::warn!(
                    "{} is corrupted ({e}), backing up and starting empty",
                    self.path.display()
                );
                self.backup_corrupted();
                return UserDictionary::default();
            }
        };
        if let Some(version) = file.version {
            if version != FORMAT_VERSION {
                tracing::warn!(
                    "{} has unsupported format version {version} (expected {FORMAT_VERSION}), loading nothing",
                    self.path.display()
                );
                return UserDictionary::default();
            }
        }
        let mut dict = UserDictionary::default();
        // `preserve_order` keeps document order, so a case-differing duplicate
        // collapses deterministically: the last one in file order wins.
        for (from, value) in file.entries.unwrap_or_default() {
            let Some((to, forms)) = parse_entry_value(&value) else {
                tracing::warn!(
                    "{}: entry {from:?} must be a string or a table with a string `to`, skipping",
                    self.path.display()
                );
                continue;
            };
            if let Err(e) = super::validate_entry_with_forms(&from, &to, &forms) {
                tracing::warn!("{}: skipping entry {from:?}: {e}", self.path.display());
                continue;
            }
            if dict.insert(DictionaryEntry { from, to, forms }) {
                tracing::warn!(
                    "{}: duplicate keys differing only by case collapsed, last one wins",
                    self.path.display()
                );
            }
        }
        dict
    }

    /// Atomically write the dictionary to the canonical file.
    pub fn save(&self, dict: &UserDictionary) -> Result<(), DictionaryError> {
        write_atomic(&self.path, serialize(dict)?.as_bytes())
    }

    /// Back up a corrupted file to `<name>.toml.bak` (best effort).
    fn backup_corrupted(&self) {
        let bak = self.path.with_extension("toml.bak");
        if let Err(e) = fs::rename(&self.path, &bak) {
            tracing::error!("failed to back up corrupted {}: {e}", self.path.display());
        }
    }
}

/// Serialize entries to the dictionary TOML (`version = 1` + `[entries]`
/// map, keys in display case, ordered by the lowercased key). An entry with
/// forms becomes a table value; one without stays a plain string.
pub fn serialize(dict: &UserDictionary) -> Result<String, DictionaryError> {
    let mut entries = toml::Table::new();
    for entry in dict.iter() {
        entries.insert(entry.from.clone(), entry_value(entry));
    }
    let file = DictionaryFileOut {
        version: FORMAT_VERSION,
        entries,
    };
    Ok(toml::to_string_pretty(&file)?)
}

/// TOML value for one entry: a bare string keeps the historical file shape,
/// while a form list needs a table value.
fn entry_value(entry: &DictionaryEntry) -> toml::Value {
    if entry.forms.is_empty() {
        return toml::Value::String(entry.to.clone());
    }
    let mut table = toml::Table::new();
    table.insert("to".to_string(), toml::Value::String(entry.to.clone()));
    table.insert(
        "forms".to_string(),
        toml::Value::Array(
            entry
                .forms
                .iter()
                .map(|form| toml::Value::String(form.clone()))
                .collect(),
        ),
    );
    toml::Value::Table(table)
}

/// Read one `[entries]` value: `"to"` or `{ to = "...", forms = [...] }`.
fn parse_entry_value(value: &toml::Value) -> Option<(String, Vec<String>)> {
    if let Some(to) = value.as_str() {
        return Some((to.to_string(), Vec::new()));
    }
    let table = value.as_table()?;
    let to = table.get("to")?.as_str()?.to_string();
    let forms = match table.get("forms") {
        None => Vec::new(),
        Some(value) => value
            .as_array()?
            .iter()
            .map(|form| form.as_str().map(str::to_string))
            .collect::<Option<Vec<_>>>()?,
    };
    Some((to, forms))
}

/// Parse an imported dictionary file: raw string → entries in document order,
/// **unvalidated** (the caller validates and counts skipped ones). A parse
/// error is surfaced as a typed error — unlike `load`, an import must not
/// silently degrade to empty. A non-string value is represented with an empty
/// `to`, which fails validation downstream and lands in the skipped count.
pub fn parse_import(raw: &str) -> Result<Vec<DictionaryEntry>, DictionaryError> {
    let file: DictionaryFileRaw = toml::from_str(raw)?;
    Ok(file
        .entries
        .unwrap_or_default()
        .into_iter()
        .map(|(from, value)| {
            let (to, forms) = parse_entry_value(&value).unwrap_or_default();
            DictionaryEntry { from, to, forms }
        })
        .collect())
}

/// Write exported TOML to an arbitrary user-chosen path (atomic).
pub fn export_to(dict: &UserDictionary, path: &Path) -> Result<(), DictionaryError> {
    write_atomic(path, serialize(dict)?.as_bytes())
}

/// Write to `<path>.tmp` then atomically rename onto `<path>` (full path
/// preserved, so the temp name never clobbers an extension).
fn write_atomic(path: &Path, data: &[u8]) -> Result<(), DictionaryError> {
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
}
