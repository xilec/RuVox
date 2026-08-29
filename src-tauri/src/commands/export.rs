//! Export command cluster (audio-export spec, #225): save-dialog picker and
//! export of an entry's stored audio file. Split out of
//! `mod.rs` along the domain seam like `import.rs`; errors map onto the
//! `export.*` wire codes here. Follows the #224 rfd-backend pattern — no
//! dialog/fs plugin, no capability surface.

use std::path::Path;

use super::*;

/// Pure default-name/filter derivation for the save dialog (unit-testable
/// without rfd): the pre-filled name carries NO extension — the native
/// dialog appends the active filter's extension on save, so the filter
/// choice (not a baked-in suffix) decides the format. For an `.opus` source
/// (the normal case) the dialog offers both export formats — `Ogg Opus`
/// first as the default, `WAV` second (#252); the synthesis-transcode
/// fallback (`.wav` source) stays WAV-only. A no-extension stored name
/// falls back to the Opus case.
fn save_dialog_defaults(
    entry_id: &EntryId,
    audio_name: &str,
) -> (String, Vec<(&'static str, &'static str)>) {
    let ext = Path::new(audio_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("opus")
        .to_string();
    let filters: Vec<(&'static str, &'static str)> = if ext == "wav" {
        vec![("WAV", "wav")]
    } else {
        vec![("Ogg Opus", "opus"), ("WAV", "wav")]
    };
    (format!("ruvox-{entry_id}"), filters)
}

/// Normalize the dialog result against the stored audio format. The native
/// dialog appends the active filter's extension to an extensionless name,
/// but a manually typed name can carry a foreign extension (`foo.mp3`) or
/// none at all — replace it (or append) with the stored format's extension
/// so the file name always matches the exported bytes (#252 manual-pass
/// feedback). A recognized `opus`/`wav` extension (any case) is kept as
/// typed and decides the export format.
fn normalize_export_target(chosen: &str, stored_ext: &str) -> String {
    let path = std::path::PathBuf::from(chosen);
    let recognized = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("opus") || e.eq_ignore_ascii_case("wav"));
    if recognized {
        return chosen.to_string();
    }
    path.with_extension(stored_ext)
        .to_string_lossy()
        .into_owned()
}

/// Dialog-free export core (unit-testable without rfd): resolve the entry's
/// cached audio file under the storage lock, then write it to the target on
/// the caller's thread. A `.wav` target for an `.opus` source is produced by
/// decoding the stream to PCM WAV (#252); every other combination is a
/// byte-for-byte copy. The command wrapper puts this on the blocking pool.
/// A vanished cached file maps to `export.no_audio` (cache eviction), a
/// failed conversion to `export.convert_failed`, any other I/O failure to
/// `export.copy_failed`. The cached original is never modified.
pub(crate) fn export_audio_to(storage: &StorageService, id: &str, target: &Path) -> CmdResult<()> {
    let uuid = parse_entry_id(id)?;
    // require_entry keeps `entry.not_found` distinct from `export.no_audio`.
    require_entry(storage, id)?;
    let source = storage
        .get_audio_path(&uuid)
        .ok_or_else(|| CommandError::not_found("export.no_audio", vec![id.to_string()]))?;

    let is_wav_target = target
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("wav"));
    let is_opus_source = source
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("opus"));
    if is_wav_target && is_opus_source {
        return crate::audio::decode_opus_to_wav(&source, target)
            .map_err(|e| CommandError::internal("export.convert_failed", vec![e.to_string()]));
    }

    std::fs::copy(&source, target)
        .map(|_| ())
        .map_err(|e| CommandError::internal("export.copy_failed", vec![e.to_string()]))
}

/// Open the native save dialog for an entry's audio (#225): the pre-filled
/// name has no extension (the active filter appends it on save); an
/// `.opus` source offers both the `Ogg Opus` and `WAV` filters (#252).
/// Returns the chosen path with its extension normalized against the
/// stored format, or `None` on cancel. The dialog is modal-blocking, so it
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
    let (default_name, filters) = save_dialog_defaults(&entry.id, &audio_name);
    let stored_ext = Path::new(&audio_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("opus")
        .to_string();

    tokio::task::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new().set_file_name(&default_name);
        for (filter_name, filter_ext) in filters {
            dialog = dialog.add_filter(filter_name, &[filter_ext]);
        }
        dialog
            .save_file()
            .map(|p| normalize_export_target(&p.to_string_lossy(), &stored_ext))
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
    fn save_dialog_defaults_offer_both_formats_for_opus_source() {
        let id: EntryId = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
        let (name, filters) =
            save_dialog_defaults(&id, "550e8400-e29b-41d4-a716-446655440000.opus");
        // No extension in the pre-filled name: the active filter appends it
        // on save, so switching the filter never leaves a stale suffix.
        assert_eq!(name, format!("ruvox-{id}"));
        assert_eq!(
            filters,
            vec![("Ogg Opus", "opus"), ("WAV", "wav")],
            "Ogg Opus must stay the first (default) filter"
        );
    }

    #[test]
    fn save_dialog_defaults_follow_the_wav_fallback() {
        let id: EntryId = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
        let (name, filters) = save_dialog_defaults(&id, "550e8400-e29b-41d4-a716-446655440000.wav");
        assert_eq!(name, format!("ruvox-{id}"));
        assert_eq!(filters, vec![("WAV", "wav")]);
    }

    #[test]
    fn save_dialog_defaults_fall_back_to_opus_without_extension() {
        let id: EntryId = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
        let (name, filters) = save_dialog_defaults(&id, "no-extension-name");
        assert_eq!(name, format!("ruvox-{id}"));
        assert_eq!(filters, vec![("Ogg Opus", "opus"), ("WAV", "wav")]);
    }

    #[test]
    fn normalize_keeps_recognized_extensions_as_typed() {
        assert_eq!(
            normalize_export_target("/tmp/ruvox-a.opus", "opus"),
            "/tmp/ruvox-a.opus"
        );
        assert_eq!(
            normalize_export_target("/tmp/ruvox-a.WAV", "opus"),
            "/tmp/ruvox-a.WAV",
            "a recognized extension decides the format regardless of case"
        );
    }

    #[test]
    fn normalize_replaces_foreign_extension_with_stored_format() {
        assert_eq!(
            normalize_export_target("/tmp/audio.mp3", "opus"),
            "/tmp/audio.opus",
            "a foreign extension must not name a file whose bytes are Opus"
        );
        assert_eq!(
            normalize_export_target("/tmp/audio.mp3", "wav"),
            "/tmp/audio.wav"
        );
    }

    #[test]
    fn normalize_appends_extension_when_missing() {
        assert_eq!(
            normalize_export_target("/tmp/ruvox-a", "opus"),
            "/tmp/ruvox-a.opus"
        );
        assert_eq!(
            normalize_export_target("/tmp/ruvox-a", "wav"),
            "/tmp/ruvox-a.wav"
        );
    }
}
