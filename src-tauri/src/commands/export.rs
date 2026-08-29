//! Export command cluster (audio-export spec, #225): save-dialog picker and
//! a byte-for-byte copy of an entry's stored audio file. Split out of
//! `mod.rs` along the domain seam like `import.rs`; errors map onto the
//! `export.*` wire codes here. Follows the #224 rfd-backend pattern — no
//! dialog/fs plugin, no capability surface.

use std::path::Path;

use super::*;

/// Pure default-name/filter derivation for the save dialog (unit-testable
/// without rfd): the name follows the stored audio file's extension, and so
/// does the filter — `Ogg Opus`/`opus` normally, `WAV`/`wav` for the
/// synthesis-transcode fallback (a no-extension name falls back to `opus`).
fn save_dialog_defaults(
    entry_id: &EntryId,
    audio_name: &str,
) -> (String, &'static str, &'static str) {
    let ext = Path::new(audio_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("opus")
        .to_string();
    let (filter_name, filter_ext) = if ext == "wav" {
        ("WAV", "wav")
    } else {
        ("Ogg Opus", "opus")
    };
    (
        format!("ruvox-{}.{}", entry_id, ext),
        filter_name,
        filter_ext,
    )
}

/// Dialog-free export core (unit-testable without rfd): resolve the entry's
/// cached audio file under the storage lock, then copy it to the target on
/// the caller's thread. The command wrapper puts this on the blocking pool.
/// A vanished cached file maps to `export.no_audio` (cache eviction), any
/// other I/O failure to `export.copy_failed`.
pub(crate) fn export_audio_to(storage: &StorageService, id: &str, target: &Path) -> CmdResult<()> {
    let uuid = parse_entry_id(id)?;
    // require_entry keeps `entry.not_found` distinct from `export.no_audio`.
    require_entry(storage, id)?;
    let source = storage
        .get_audio_path(&uuid)
        .ok_or_else(|| CommandError::not_found("export.no_audio", vec![id.to_string()]))?;
    std::fs::copy(&source, target)
        .map(|_| ())
        .map_err(|e| CommandError::internal("export.copy_failed", vec![e.to_string()]))
}

/// Open the native save dialog for an entry's audio (#225): the default
/// name and the file filter follow the stored audio format. Returns the
/// chosen path or `None` on cancel. The dialog is modal-blocking, so it
/// runs on the blocking pool (the #224 `pick_import_file` pattern).
#[tauri::command]
pub async fn pick_export_audio_path(
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<Option<String>> {
    let entry = require_entry(&state.storage, &id)?;
    let audio_name = entry
        .audio_path
        .ok_or_else(|| CommandError::not_found("export.no_audio", vec![id.clone()]))?;
    let (default_name, filter_name, filter_ext) = save_dialog_defaults(&entry.id, &audio_name);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_dialog_defaults_follow_the_opus_format() {
        let id: EntryId = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
        let (name, filter_name, filter_ext) =
            save_dialog_defaults(&id, "550e8400-e29b-41d4-a716-446655440000.opus");
        assert_eq!(name, format!("ruvox-{id}.opus"));
        assert_eq!((filter_name, filter_ext), ("Ogg Opus", "opus"));
    }

    #[test]
    fn save_dialog_defaults_follow_the_wav_fallback() {
        let id: EntryId = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
        let (name, filter_name, filter_ext) =
            save_dialog_defaults(&id, "550e8400-e29b-41d4-a716-446655440000.wav");
        assert_eq!(name, format!("ruvox-{id}.wav"));
        assert_eq!((filter_name, filter_ext), ("WAV", "wav"));
    }

    #[test]
    fn save_dialog_defaults_fall_back_to_opus_without_extension() {
        let id: EntryId = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
        let (name, filter_name, filter_ext) = save_dialog_defaults(&id, "no-extension-name");
        assert_eq!(name, format!("ruvox-{id}.opus"));
        assert_eq!((filter_name, filter_ext), ("Ogg Opus", "opus"));
    }
}
