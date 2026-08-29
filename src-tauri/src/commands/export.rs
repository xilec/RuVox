//! Export command cluster (audio-export spec, #225): save-dialog picker and
//! a byte-for-byte copy of an entry's stored audio file. Split out of
//! `mod.rs` along the domain seam like `import.rs`; errors map onto the
//! `export.*` wire codes here. Follows the #224 rfd-backend pattern — no
//! dialog/fs plugin, no capability surface.

use std::path::Path;

use super::*;

/// Dialog-free export core (unit-testable without rfd): resolve the entry's
/// cached audio file under the storage lock, then copy it to the target on
/// the caller's thread. The command wrapper puts this on the blocking pool.
/// A vanished source file maps to `export.no_audio` (cache eviction), any
/// other I/O failure to `export.copy_failed`.
pub(crate) fn export_audio_to(storage: &StorageService, id: &str, target: &Path) -> CmdResult<()> {
    let entry = require_entry(storage, id)?;
    let audio_name = entry
        .audio_path
        .ok_or_else(|| CommandError::not_found("export.no_audio", vec![id.to_string()]))?;
    let source = storage.data_dir().join("audio").join(&audio_name);
    if !source.is_file() {
        return Err(CommandError::not_found("export.no_audio", vec![audio_name]));
    }
    std::fs::copy(&source, target)
        .map(|_| ())
        .map_err(|e| CommandError::internal("export.copy_failed", vec![e.to_string()]))
}

/// Open the native save dialog for an entry's audio (#225): the default
/// name and the file filter follow the stored audio format (`Ogg Opus` for
/// `.opus`, `WAV` for `.wav`). Returns the chosen path or `None` on cancel.
/// The dialog is modal-blocking, so it runs on the blocking pool (the
/// #224 `pick_import_file` pattern).
#[tauri::command]
pub async fn pick_export_audio_path(
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<Option<String>> {
    let entry = require_entry(&state.storage, &id)?;
    let audio_name = entry
        .audio_path
        .ok_or_else(|| CommandError::not_found("export.no_audio", vec![id.clone()]))?;
    let ext = Path::new(&audio_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("opus")
        .to_string();
    let default_name = format!("ruvox-{}.{}", entry.id, ext);
    let (filter_name, filter_ext) = if ext == "wav" {
        ("WAV", "wav")
    } else {
        ("Ogg Opus", "opus")
    };

    tokio::task::spawn_blocking(move || {
        rfd::FileDialog::new()
            .set_file_name(&default_name)
            .add_filter(filter_name, &[filter_ext])
            .save_file()
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| {
        CommandError::internal("export.dialog_panicked", vec![]).with_message(e.to_string())
    })
}

/// Copy an entry's stored audio file to a user-chosen path (#225).
/// Byte-for-byte: the cache original is untouched and no `entry_updated`
/// is emitted — exporting is not a queue event. The copy is I/O-bound, so
/// it runs on the blocking pool.
#[tauri::command]
pub async fn export_audio(state: State<'_, AppState>, id: String, path: String) -> CmdResult<()> {
    let storage = Arc::clone(&state.storage);
    tokio::task::spawn_blocking(move || export_audio_to(&storage, &id, Path::new(&path)))
        .await
        .map_err(|e| {
            CommandError::internal("export.task_panicked", vec![]).with_message(e.to_string())
        })?
}
