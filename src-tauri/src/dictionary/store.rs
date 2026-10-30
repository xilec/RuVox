//! TOML persistence for the user dictionary (config root, alongside
//! `config.json`). The file is the source of truth at startup; missing file
//! means an empty global dictionary, a corrupted file is backed up and
//! replaced by an empty one (the `config.json` recovery pattern).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{DictionaryBook, DictionaryEntry, DictionaryError, DictionaryScope, UserDictionary};
use crate::tts::EngineKind;

/// File-format version. Version 1 is the historical single `[entries]` map;
/// version 2 is an array of named, scoped collections. A file with a newer
/// version is treated as foreign: loaded as empty and never backed up (a
/// newer app's dictionary must not be destroyed by an older build reading it).
const FORMAT_VERSION: u32 = 2;

/// Raw file shape for parsing. Both layouts are optional so hand-minimal files
/// parse leniently. Entries are a [`toml::Table`] — `Map<String, String>` has
/// no serde impls in toml 0.8 — and each value is either a string or a table
/// with `to`/`forms`.
#[derive(Deserialize)]
struct DictionaryFileRaw {
    version: Option<u32>,
    /// Version 1 layout.
    entries: Option<toml::Table>,
    /// Version 2 layout. Kept as raw tables so a collection whose scope this
    /// build does not know survives a load/save round-trip.
    dictionaries: Option<Vec<toml::Table>>,
}

#[derive(Serialize)]
struct DictionaryFileOut {
    version: u32,
    dictionaries: Vec<toml::Value>,
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

    /// Load all collections. Infallible: missing file → one empty global
    /// collection; unreadable or unparsable file → backup as `.bak` + empty +
    /// warning; entries failing validation are skipped individually (a warning
    /// per entry). Version 1 files load as one global collection.
    pub fn load_book(&self) -> DictionaryBook {
        if !self.path.exists() {
            return DictionaryBook::with_global();
        }
        let raw = match fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(e) => {
                tracing::warn!("failed to read {}: {e}", self.path.display());
                return DictionaryBook::with_global();
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
                return DictionaryBook::with_global();
            }
        };

        if let Some(version) = file.version {
            if version > FORMAT_VERSION {
                tracing::warn!(
                    "{} has unsupported format version {version} (expected at most {FORMAT_VERSION}), loading nothing",
                    self.path.display()
                );
                return DictionaryBook::with_global();
            }
        }

        let mut book = DictionaryBook::with_global();
        if file.version == Some(2) || file.dictionaries.is_some() {
            for raw_collection in file.dictionaries.unwrap_or_default() {
                let scope_id = raw_collection
                    .get("scope")
                    .and_then(toml::Value::as_str)
                    .unwrap_or_default();
                let Some(scope) = DictionaryScope::parse(scope_id) else {
                    tracing::warn!(
                        "{}: unknown dictionary scope {:?}, preserving the collection as-is",
                        self.path.display(),
                        scope_id
                    );
                    book.push_unknown(raw_collection);
                    continue;
                };
                let name = raw_collection
                    .get("name")
                    .and_then(toml::Value::as_str)
                    .unwrap_or(scope.default_name())
                    .to_string();
                let entries = parse_entries(
                    raw_collection
                        .get("entries")
                        .and_then(toml::Value::as_table)
                        .cloned()
                        .unwrap_or_default(),
                    &self.path,
                    &format!("collection {name:?}"),
                );
                if book.collection(scope).is_some() {
                    tracing::warn!(
                        "{}: duplicate dictionary scope {:?}, last collection wins",
                        self.path.display(),
                        scope.as_str()
                    );
                }
                book.upsert(name, scope, entries);
            }
            book.ensure_global();
            return book;
        }

        // Version 1 (or a hand-written file without a version): the historical
        // single `[entries]` map becomes the global collection.
        let entries = parse_entries(
            file.entries.unwrap_or_default(),
            &self.path,
            "global entries",
        );
        book.upsert("Global", DictionaryScope::Global, entries);
        book
    }

    /// Compatibility accessor for callers that only need the global map
    /// (startup tests and the historical API). Real lookup uses
    /// [`DictionaryBook::merged_for`] with the active engine.
    pub fn load(&self) -> UserDictionary {
        self.load_book().merged_for(EngineKind::Silero)
    }

    /// Atomically write all collections to the canonical file.
    pub fn save_book(&self, book: &DictionaryBook) -> Result<(), DictionaryError> {
        write_atomic(&self.path, serialize_book(book)?.as_bytes())
    }

    /// Atomically write one historical single-dictionary file (used by export
    /// and by the compatibility path).
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

/// Parse and validate one `[entries]`-shaped table, skipping invalid entries.
fn parse_entries(table: toml::Table, path: &Path, source: &str) -> UserDictionary {
    let mut dict = UserDictionary::default();
    // `preserve_order` keeps document order, so a case-differing duplicate
    // collapses deterministically: the last one in file order wins.
    for (from, value) in table {
        let Some((to, forms)) = parse_entry_value(&value) else {
            tracing::warn!(
                "{}: {source} entry {from:?} must be a string or a table with a string `to`, skipping",
                path.display()
            );
            continue;
        };
        if let Err(e) = super::validate_entry_with_forms(&from, &to, &forms) {
            tracing::warn!("{}: skipping {source} entry {from:?}: {e}", path.display());
            continue;
        }
        if dict.insert(DictionaryEntry { from, to, forms }) {
            tracing::warn!(
                "{}: duplicate keys differing only by case collapsed, last one wins",
                path.display()
            );
        }
    }
    dict
}

/// Serialize all collections to the version 2 dictionary TOML.
pub fn serialize_book(book: &DictionaryBook) -> Result<String, DictionaryError> {
    // The global collection is the invariant a hand-built book can miss;
    // never persist a file without it.
    let mut book = book.clone();
    book.ensure_global();
    let mut dictionaries = Vec::new();
    for collection in book.iter() {
        if collection.scope != DictionaryScope::Global && collection.entries.is_empty() {
            continue;
        }
        let mut table = toml::Table::new();
        table.insert(
            "name".to_string(),
            toml::Value::String(collection.name.clone()),
        );
        table.insert(
            "scope".to_string(),
            toml::Value::String(collection.scope.as_str().to_string()),
        );
        table.insert(
            "entries".to_string(),
            toml::Value::Table(entries_table(&collection.entries)),
        );
        dictionaries.push(toml::Value::Table(table));
    }
    for unknown in book.unknown_collections() {
        dictionaries.push(toml::Value::Table(unknown.clone()));
    }
    let file = DictionaryFileOut {
        version: FORMAT_VERSION,
        dictionaries,
    };
    Ok(toml::to_string_pretty(&file)?)
}

/// Serialize one historical single-dictionary file (`version = 1` + `[entries]`).
pub fn serialize(dict: &UserDictionary) -> Result<String, DictionaryError> {
    let file = HistoricalFileOut {
        version: 1,
        entries: entries_table(dict),
    };
    Ok(toml::to_string_pretty(&file)?)
}

#[derive(Serialize)]
struct HistoricalFileOut {
    version: u32,
    entries: toml::Table,
}

fn entries_table(dict: &UserDictionary) -> toml::Table {
    let mut entries = toml::Table::new();
    for entry in dict.iter() {
        entries.insert(entry.from.clone(), entry_value(entry));
    }
    entries
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
///
/// Version 2 files are accepted by taking the global collection (or the first
/// collection when no global one is present), so exporting a selected
/// collection as version 1 remains the portable path.
pub fn parse_import(raw: &str) -> Result<Vec<DictionaryEntry>, DictionaryError> {
    let file: DictionaryFileRaw = toml::from_str(raw)?;
    let table = if let Some(entries) = file.entries {
        entries
    } else {
        let mut collections = file.dictionaries.unwrap_or_default();
        let Some(index) = collections
            .iter()
            .position(|c| c.get("scope").and_then(toml::Value::as_str) == Some("global"))
        else {
            return Err(DictionaryError::InvalidImport(
                "version 2 dictionary has no global collection".to_string(),
            ));
        };
        collections
            .remove(index)
            .get("entries")
            .and_then(toml::Value::as_table)
            .cloned()
            .unwrap_or_default()
    };
    Ok(table
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
