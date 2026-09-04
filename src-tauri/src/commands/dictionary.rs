//! User dictionary commands (change `user-dictionary`): CRUD over the
//! persisted TOML file plus immediate pipeline refresh. Import supports two
//! modes (merge / replace) chosen by the user; save is all-or-nothing.

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::{CmdResult, CommandError};
use crate::dictionary::{self, DictionaryEntry, DictionaryError, UserDictionary};
use crate::pipeline::TTSPipeline;
use crate::state::AppState;

/// Entry as the UI sees it: the mapping plus the built-in override marker
/// (backend-computed so the editor can badge entries shadowing built-ins
/// without leaking the built-in tables themselves).
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntryDto {
    pub from: String,
    pub to: String,
    pub overrides_builtin: bool,
}

/// Import outcome counts.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub added: u32,
    pub updated: u32,
    pub skipped: u32,
}

/// How an import interacts with the current dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportMode {
    /// Imported entries win on key collisions; invalid entries are skipped.
    Merge,
    /// Validated entries fully replace the current dictionary.
    Replace,
}

fn to_command_error(e: DictionaryError) -> CommandError {
    // Detail rides in params so the frontend catalog renders it after the
    // localized prefix (ipc-commands wire format).
    CommandError::config("dictionary.error", vec![e.to_string()])
}

fn dto_list(pipeline: &TTSPipeline) -> Vec<DictionaryEntryDto> {
    pipeline
        .user_dictionary()
        .iter()
        .map(|entry| DictionaryEntryDto {
            from: entry.from.clone(),
            to: entry.to.clone(),
            overrides_builtin: pipeline.builtin_contains(&entry.key()),
        })
        .collect()
}

/// Sorted entry list with the built-in override marker for the editor.
#[tauri::command]
pub async fn get_user_dictionary(state: State<'_, AppState>) -> CmdResult<Vec<DictionaryEntryDto>> {
    let pipeline = state.pipeline.lock();
    Ok(dto_list(&pipeline))
}

/// Validate every entry, atomically replace the whole dictionary, persist,
/// and refresh the active pipeline. All-or-nothing: any invalid entry rejects
/// the save and leaves the file unchanged. The pipeline lock is held across
/// compute + file write + refresh so concurrent commands can never leave the
/// file and the active map diverging (the lock is sync and never crosses an
/// await; the file is tiny).
#[tauri::command]
pub async fn save_user_dictionary(
    state: State<'_, AppState>,
    entries: Vec<DictionaryEntry>,
) -> CmdResult<()> {
    for entry in &entries {
        dictionary::validate_entry(&entry.from, &entry.to).map_err(to_command_error)?;
    }

    let mut dict = UserDictionary::default();
    for entry in entries {
        dict.insert(entry);
    }

    let store = Arc::clone(&state.dictionary_store);
    let pipeline = Arc::clone(&state.pipeline);
    tokio::task::spawn_blocking(move || {
        // File I/O is blocking by nature — keep the async runtime free
        // (same pattern as read_text_file in commands/import.rs).
        store.save(&dict).map_err(to_command_error)?;
        pipeline.lock().set_user_dictionary(dict);
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
                if dictionary::validate_entry(&entry.from, &entry.to).is_err() {
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
                if dictionary::validate_entry(&entry.from, &entry.to).is_err() {
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

/// Read a dictionary TOML file and apply it in the chosen mode, persist, and
/// refresh the active pipeline. An unreadable or unparsable file rejects with
/// a typed error and changes nothing; invalid entries never abort a merge.
/// Like `save_user_dictionary`, the pipeline lock spans compute + write +
/// refresh so concurrent commands cannot interleave.
#[tauri::command]
pub async fn import_user_dictionary(
    state: State<'_, AppState>,
    path: PathBuf,
    mode: ImportMode,
) -> CmdResult<ImportReport> {
    let store = Arc::clone(&state.dictionary_store);
    let pipeline = Arc::clone(&state.pipeline);
    tokio::task::spawn_blocking(move || {
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| CommandError::config("dictionary.import_failed", vec![e.to_string()]))?;
        let imported = dictionary::parse_import(&raw).map_err(to_command_error)?;

        let mut guard = pipeline.lock();
        let (dict, report) = apply_import(guard.user_dictionary(), imported, mode);
        store.save(&dict).map_err(to_command_error)?;
        guard.set_user_dictionary(dict);
        Ok(report)
    })
    .await
    .map_err(|e| {
        CommandError::internal("dictionary.task_panicked", vec![]).with_message(e.to_string())
    })?
}

/// Write the current entries as valid dictionary TOML to a user-chosen path.
#[tauri::command]
pub async fn export_user_dictionary(state: State<'_, AppState>, path: PathBuf) -> CmdResult<()> {
    let dict = state.pipeline.lock().user_dictionary().clone();
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

    fn entry(from: &str, to: &str) -> DictionaryEntry {
        DictionaryEntry {
            from: from.to_string(),
            to: to.to_string(),
        }
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
            entry("Иванов", "иванов"), // invalid: Cyrillic from
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
    fn get_marks_builtin_overrides() {
        let pipeline = TTSPipeline::new();
        let mut dict = UserDictionary::default();
        dict.insert(entry("docker", "докка")); // docker ∈ IT_TERMS
        dict.insert(entry("ios", "ай ос")); // ios ∈ abbreviation SPECIAL_CASES
        // Not in IT_TERMS / abbreviation maps / CODE_WORDS.
        dict.insert(entry("zabbix", "заббикс"));
        let mut pipeline = pipeline;
        pipeline.set_user_dictionary(dict);

        let dtos = dto_list(&pipeline);
        assert_eq!(dtos.len(), 3);
        let docker = dtos.iter().find(|d| d.from == "docker").expect("docker");
        let ios = dtos.iter().find(|d| d.from == "ios").expect("ios");
        let zabbix = dtos.iter().find(|d| d.from == "zabbix").expect("zabbix");
        assert!(docker.overrides_builtin);
        assert!(ios.overrides_builtin);
        assert!(!zabbix.overrides_builtin);
    }
}
