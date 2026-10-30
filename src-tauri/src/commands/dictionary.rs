//! User dictionary commands (change `user-dictionary`): CRUD over the
//! persisted TOML file plus immediate pipeline refresh. The file stores named
//! collections scoped to global or one engine; the pipeline receives the
//! merged map for the active engine. Import supports two modes (merge /
//! replace) chosen by the user; save is all-or-nothing.

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;
use tts_contract::TextProfile;

use super::{CmdResult, CommandError};
use crate::dictionary::{self, DictionaryEntry, DictionaryError, DictionaryScope, UserDictionary};
use crate::pipeline::TTSPipeline;
use crate::state::AppState;
use crate::tts::EngineKind;
use crate::tts::TtsEngine;
use crate::tts::profiles;

/// Entry as the UI sees it: the mapping plus the built-in override marker
/// (backend-computed so the editor can badge entries shadowing built-ins
/// without leaking the built-in tables themselves).
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntryDto {
    pub from: String,
    pub to: String,
    pub forms: Vec<String>,
    pub overrides_builtin: bool,
}

/// One named collection as the editor sees it.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryCollectionDto {
    /// Stable scope id (`global`, `piper`, `silero`, `silero_native`, `teratts`).
    pub scope: String,
    pub name: String,
    pub entries: Vec<DictionaryEntryDto>,
}

/// Import outcome counts.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub added: u32,
    pub updated: u32,
    pub skipped: u32,
}

/// How an import interacts with the selected collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportMode {
    /// Imported entries win on key collisions; invalid entries are skipped.
    Merge,
    /// Validated entries fully replace the selected collection.
    Replace,
}

fn to_command_error(e: DictionaryError) -> CommandError {
    // Detail rides in params so the frontend catalog renders it after the
    // localized prefix (ipc-commands wire format).
    CommandError::config("dictionary.error", vec![e.to_string()])
}

fn parse_scope(scope: &str) -> Result<DictionaryScope, CommandError> {
    DictionaryScope::parse(scope)
        .ok_or_else(|| to_command_error(DictionaryError::InvalidScope(scope.to_string())))
}

fn active_kind(state: &AppState) -> EngineKind {
    state.engine_switcher.kind()
}

/// The profile whose built-ins the selected collection can override: the
/// engine's own profile for an engine collection, the active engine's profile
/// for the global one.
fn scope_profile(scope: DictionaryScope, active: EngineKind) -> &'static TextProfile {
    match scope {
        DictionaryScope::Global => profiles::profile_for(active),
        DictionaryScope::Engine(kind) => profiles::profile_for(kind),
    }
}

fn dto_list(
    pipeline: &TTSPipeline,
    dict: &UserDictionary,
    profile: &TextProfile,
) -> Vec<DictionaryEntryDto> {
    dict.iter()
        .map(|entry| DictionaryEntryDto {
            from: entry.from.clone(),
            to: entry.to.clone(),
            forms: entry.forms.clone(),
            overrides_builtin: pipeline.builtin_contains_for(profile, &entry.key()),
        })
        .collect()
}

/// All collections with their entries and the built-in marker for each
/// collection's profile. The global collection is always present.
#[tauri::command]
pub async fn get_dictionaries(
    state: State<'_, AppState>,
) -> CmdResult<Vec<DictionaryCollectionDto>> {
    let book = state.dictionary_store.load_book();
    let active = active_kind(&state);
    let pipeline = state.pipeline.lock();
    Ok(book
        .iter()
        .map(|collection| DictionaryCollectionDto {
            scope: collection.scope.as_str().to_string(),
            name: collection.name.clone(),
            entries: dto_list(
                &pipeline,
                &collection.entries,
                scope_profile(collection.scope, active),
            ),
        })
        .collect())
}

/// Validate every entry, atomically replace one collection, persist, and
/// refresh the active merged map. All-or-nothing: any invalid entry rejects
/// the save and leaves the file unchanged. The pipeline lock is held across
/// compute + file write + refresh so concurrent commands can never leave the
/// file and the active map diverging.
#[tauri::command]
pub async fn save_dictionary(
    state: State<'_, AppState>,
    scope: String,
    entries: Vec<DictionaryEntry>,
) -> CmdResult<()> {
    let scope = parse_scope(&scope)?;
    for entry in &entries {
        dictionary::validate_entry_with_forms(&entry.from, &entry.to, &entry.forms)
            .map_err(to_command_error)?;
    }
    dictionary::validate_no_form_collisions(&entries).map_err(to_command_error)?;

    let mut dict = UserDictionary::default();
    for entry in entries {
        dict.insert(entry);
    }

    let store = Arc::clone(&state.dictionary_store);
    let pipeline = Arc::clone(&state.pipeline);
    let active = active_kind(&state);
    tokio::task::spawn_blocking(move || {
        let mut book = store.load_book();
        let name = book
            .collection(scope)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| scope.default_name().to_string());
        book.upsert(name, scope, dict);
        store.save_book(&book).map_err(to_command_error)?;
        pipeline.lock().set_user_dictionary(book.merged_for(active));
        Ok(())
    })
    .await
    .map_err(|e| {
        CommandError::internal("dictionary.task_panicked", vec![]).with_message(e.to_string())
    })?
}

/// Apply imported entries onto a current dictionary per the import mode.
/// Pure so the merge/replace/skip counting is unit-testable without Tauri.
fn apply_import(
    current: &UserDictionary,
    imported: Vec<DictionaryEntry>,
    mode: ImportMode,
) -> (UserDictionary, ImportReport) {
    let mut report = ImportReport {
        added: 0,
        updated: 0,
        skipped: 0,
    };
    match mode {
        ImportMode::Merge => {
            let mut dict = current.clone();
            for entry in imported {
                if dictionary::validate_entry_with_forms(&entry.from, &entry.to, &entry.forms)
                    .is_err()
                {
                    report.skipped += 1;
                    continue;
                }
                let mut candidate = dict.clone();
                candidate.insert(entry.clone());
                let snapshot: Vec<DictionaryEntry> = candidate.iter().cloned().collect();
                if dictionary::validate_no_form_collisions(&snapshot).is_err() {
                    report.skipped += 1;
                    continue;
                }
                if dict.insert(entry) {
                    report.updated += 1;
                } else {
                    report.added += 1;
                }
            }
            (dict, report)
        }
        ImportMode::Replace => {
            let mut dict = UserDictionary::default();
            for entry in imported {
                if dictionary::validate_entry_with_forms(&entry.from, &entry.to, &entry.forms)
                    .is_err()
                {
                    report.skipped += 1;
                    continue;
                }
                let mut candidate = dict.clone();
                candidate.insert(entry.clone());
                let snapshot: Vec<DictionaryEntry> = candidate.iter().cloned().collect();
                if dictionary::validate_no_form_collisions(&snapshot).is_err() {
                    report.skipped += 1;
                    continue;
                }
                dict.insert(entry);
                report.added += 1;
            }
            (dict, report)
        }
    }
}

/// Native open dialog filtered to TOML dictionaries; null = cancelled.
/// Same rfd-on-blocking-pool pattern as `pick_import_file`.
#[tauri::command]
pub async fn pick_dictionary_import_path() -> CmdResult<Option<String>> {
    tokio::task::spawn_blocking(|| {
        rfd::FileDialog::new()
            .add_filter("Словарь RuVox", &["toml"])
            .pick_file()
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| {
        CommandError::internal("dictionary.task_panicked", vec![]).with_message(e.to_string())
    })
}

/// Native save dialog suggesting `user_dictionary.toml`; null = cancelled.
#[tauri::command]
pub async fn pick_dictionary_export_path() -> CmdResult<Option<String>> {
    tokio::task::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_file_name("user_dictionary.toml")
            .add_filter("Словарь RuVox", &["toml"])
            .save_file()
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| {
        CommandError::internal("dictionary.task_panicked", vec![]).with_message(e.to_string())
    })
}

/// Read a dictionary TOML file and apply it to the selected collection in the
/// chosen mode, persist, and refresh the active merged map. An unreadable or
/// unparsable file rejects with a typed error and changes nothing; invalid
/// entries never abort a merge. Like `save_dictionary`, the pipeline lock
/// spans compute + write + refresh so concurrent commands cannot interleave.
#[tauri::command]
pub async fn import_dictionary(
    state: State<'_, AppState>,
    path: PathBuf,
    mode: ImportMode,
    scope: String,
) -> CmdResult<ImportReport> {
    let scope = parse_scope(&scope)?;
    let store = Arc::clone(&state.dictionary_store);
    let pipeline = Arc::clone(&state.pipeline);
    let active = active_kind(&state);
    tokio::task::spawn_blocking(move || {
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| CommandError::config("dictionary.import_failed", vec![e.to_string()]))?;
        let imported = dictionary::parse_import(&raw).map_err(to_command_error)?;

        let mut book = store.load_book();
        let name = book
            .collection(scope)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| scope.default_name().to_string());
        let current = book
            .collection(scope)
            .map(|c| c.entries.clone())
            .unwrap_or_default();
        let (dict, report) = apply_import(&current, imported, mode);
        book.upsert(name, scope, dict);
        store.save_book(&book).map_err(to_command_error)?;
        pipeline.lock().set_user_dictionary(book.merged_for(active));
        Ok(report)
    })
    .await
    .map_err(|e| {
        CommandError::internal("dictionary.task_panicked", vec![]).with_message(e.to_string())
    })?
}

/// Write the selected collection as valid dictionary TOML to a user-chosen
/// path.
#[tauri::command]
pub async fn export_dictionary(
    state: State<'_, AppState>,
    path: PathBuf,
    scope: String,
) -> CmdResult<()> {
    let scope = parse_scope(&scope)?;
    let dict = state
        .dictionary_store
        .load_book()
        .collection(scope)
        .map(|c| c.entries.clone())
        .unwrap_or_default();
    tokio::task::spawn_blocking(move || {
        dictionary::export_to(&dict, &path).map_err(to_command_error)
    })
    .await
    .map_err(|e| {
        CommandError::internal("dictionary.task_panicked", vec![]).with_message(e.to_string())
    })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::DictionaryBook;

    fn entry(from: &str, to: &str) -> DictionaryEntry {
        DictionaryEntry::new(from, to)
    }

    fn dict_of(entries: &[(&str, &str)]) -> UserDictionary {
        let mut dict = UserDictionary::default();
        for (from, to) in entries {
            dict.insert(entry(from, to));
        }
        dict
    }

    #[test]
    fn merge_applies_imported_wins_on_collision() {
        let current = dict_of(&[("docker", "докер"), ("nginx", "энджинкс")]);
        let imported = vec![
            entry("docker", "докка"),
            entry("kubectl", "куб контрол"),
            entry("Иванов Пётр", "иванов"), // invalid: two words
        ];

        let (dict, report) = apply_import(&current, imported, ImportMode::Merge);

        assert_eq!(
            report,
            ImportReport {
                added: 1,
                updated: 1,
                skipped: 1
            }
        );
        assert_eq!(dict.get("docker"), Some("докка"), "imported wins");
        assert_eq!(dict.get("nginx"), Some("энджинкс"), "existing kept");
        assert_eq!(dict.get("kubectl"), Some("куб контрол"));
        assert_eq!(dict.len(), 3);
    }

    #[test]
    fn replace_replaces_only_valid_entries() {
        let current = dict_of(&[("docker", "докер"), ("nginx", "энджинкс")]);
        let imported = vec![entry("kubectl", "куб контрол"), entry("123", "число")];

        let (dict, report) = apply_import(&current, imported, ImportMode::Replace);

        assert_eq!(
            report,
            ImportReport {
                added: 1,
                updated: 0,
                skipped: 1
            }
        );
        assert_eq!(dict.len(), 1);
        assert_eq!(dict.get("kubectl"), Some("куб контрол"));
        assert_eq!(dict.get("docker"), None, "old entries gone");
    }

    #[test]
    fn merge_keeps_forms_and_skips_a_colliding_form() {
        let current = dict_of(&[("дворец", "двор+ец")]);
        let imported = vec![
            DictionaryEntry::with_forms("замок", "зам+ок", ["замка", "замке"]),
            DictionaryEntry::with_forms("дворец", "двор+ец", ["замка"]),
        ];

        let (dict, report) = apply_import(&current, imported, ImportMode::Merge);

        assert_eq!(
            report,
            ImportReport {
                added: 1,
                updated: 0,
                skipped: 1
            }
        );
        assert_eq!(dict.resolve("замка", true).as_deref(), Some("зам+ок"));
        assert_eq!(
            dict.iter().find(|e| e.from == "замок").unwrap().forms,
            ["замка", "замке"]
        );
    }

    #[test]
    fn get_marks_builtin_overrides() {
        let pipeline = TTSPipeline::new();
        let mut dict = UserDictionary::default();
        dict.insert(entry("nginx", "энджинкс-кастом")); // nginx ∈ IT_TERMS
        dict.insert(entry("ios", "ай ос")); // ios ∈ abbreviation SPECIAL_CASES
        // Not in IT_TERMS / abbreviation maps / CODE_WORDS.
        dict.insert(entry("zabbix", "заббикс"));
        let mut pipeline = pipeline;
        pipeline.set_user_dictionary(dict);

        let dtos = dto_list(
            &pipeline,
            pipeline.user_dictionary(),
            &profiles::SILERO_PROFILE,
        );
        assert_eq!(dtos.len(), 3);
        let nginx = dtos.iter().find(|d| d.from == "nginx").expect("nginx");
        let ios = dtos.iter().find(|d| d.from == "ios").expect("ios");
        let zabbix = dtos.iter().find(|d| d.from == "zabbix").expect("zabbix");
        assert!(nginx.overrides_builtin);
        assert!(ios.overrides_builtin);
        assert!(!zabbix.overrides_builtin);
    }

    #[test]
    fn native_profile_does_not_mark_english_builtins() {
        let pipeline = TTSPipeline::new();
        let mut dict = UserDictionary::default();
        dict.insert(entry("nginx", "энджинкс-кастом"));
        dict.insert(entry("getUserData", "гет юзер дата"));
        let mut pipeline = pipeline;
        pipeline.set_user_dictionary(dict);

        let dtos = dto_list(
            &pipeline,
            pipeline.user_dictionary(),
            &profiles::NATIVE_EN_PROFILE,
        );
        assert!(dtos.iter().all(|d| !d.overrides_builtin));
    }

    #[test]
    fn native_profile_still_marks_mixed_alnum_builtins() {
        let pipeline = TTSPipeline::new();
        let mut dict = UserDictionary::default();
        dict.insert(entry("IPv6", "айпи ви шесть"));
        let mut pipeline = pipeline;
        pipeline.set_user_dictionary(dict);

        let dtos = dto_list(
            &pipeline,
            pipeline.user_dictionary(),
            &profiles::NATIVE_EN_PROFILE,
        );
        assert!(dtos[0].overrides_builtin);
    }

    #[test]
    fn book_merge_prefers_engine_entries() {
        let mut book = DictionaryBook::with_global();
        let mut global = UserDictionary::default();
        global.insert(entry("docker", "докер"));
        book.upsert("Global", DictionaryScope::Global, global);
        let mut piper = UserDictionary::default();
        piper.insert(entry("docker", "докка"));
        book.upsert("Piper", DictionaryScope::Engine(EngineKind::Piper), piper);

        assert_eq!(
            book.merged_for(EngineKind::Piper).get("docker"),
            Some("докка")
        );
        assert_eq!(
            book.merged_for(EngineKind::TeraTts).get("docker"),
            Some("докер")
        );
    }
}
