//! Export command cluster (audio-export spec, #225): save-dialog picker and
//! export of an entry's stored audio file. Split out of
//! `mod.rs` along the domain seam like `import.rs`; errors map onto the
//! `export.*` wire codes here. Follows the #224 rfd-backend pattern — no
//! dialog/fs plugin, no capability surface.

use std::path::Path;

use super::*;

/// The stored audio file's extension, defaulting to `opus` (the normal
/// case; the synthesis-transcode fallback stores `.wav`).
fn stored_ext_of(audio_name: &str) -> String {
    Path::new(audio_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("opus")
        .to_string()
}

/// Normalize the dialog result so the file name always matches the exported
/// bytes. `requested` is the format the dialog's «Формат» choice reported
/// (`Some`, Linux portal): a matching extension is kept as typed (any
/// case), a mismatched or foreign one is replaced, a missing one appended.
/// Without a reported choice (`None`, platforms whose dialog cannot report
/// it): a recognized `opus`/`wav` extension is kept as typed, anything else
/// falls back to the stored format's extension.
fn normalize_export_target(chosen: &str, requested: Option<&str>, stored_ext: &str) -> String {
    let path = std::path::PathBuf::from(chosen);
    let typed = path.extension().and_then(|e| e.to_str());
    let target = match requested {
        Some(ext) => {
            if typed.is_some_and(|e| e.eq_ignore_ascii_case(ext)) {
                return chosen.to_string();
            }
            ext
        }
        None => {
            if typed
                .is_some_and(|e| e.eq_ignore_ascii_case("opus") || e.eq_ignore_ascii_case("wav"))
            {
                return chosen.to_string();
            }
            stored_ext
        }
    };
    path.with_extension(target).to_string_lossy().into_owned()
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

/// Open the native save dialog for an entry's audio and report the chosen
/// path plus the format the dialog itself selected (`None` when the dialog
/// cannot report one, or the user cancelled).
#[tauri::command]
pub async fn pick_export_audio_path(
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<Option<String>> {
    let entry = require_entry(&state.storage, &id)?;
    let audio_name = entry
        .audio_path
        .ok_or_else(|| CommandError::not_found("export.no_audio", vec![id.clone()]))?;
    let stored_ext = stored_ext_of(&audio_name);

    let chosen = run_save_dialog(&entry.id.to_string()).await?;
    Ok(chosen.map(|(path, format)| normalize_export_target(&path, format.as_deref(), &stored_ext)))
}

/// Linux: the xdg-desktop-portal save dialog with a «Формат» choice combo —
/// WAV first (the widely editable format a user exports for), Ogg Opus
/// second. The portal response reports the combo's selected value, which
/// rfd's flat `Option<PathBuf>` discards — hence the direct ashpd call.
/// No file-type filters: the combo, not a filter switch, decides the
/// format. Awaiting the portal future needs no blocking pool (it is IPC
/// plus the user's think time, not CPU work).
#[cfg(target_os = "linux")]
async fn run_save_dialog(entry_id: &str) -> CmdResult<Option<(String, Option<String>)>> {
    use ashpd::desktop::file_chooser::{Choice, SaveFileRequest};

    let default_name = format!("ruvox-{entry_id}.wav");
    let request = SaveFileRequest::default()
        .title("Сохранить аудио как…")
        .current_name(default_name.as_str())
        .choice(
            Choice::new("format", "Формат", "wav")
                .insert("wav", "WAV")
                .insert("opus", "Ogg Opus"),
        );
    let request = request
        .send()
        .await
        .map_err(|e| CommandError::internal("export.dialog_failed", vec![e.to_string()]))?;
    let selected = match request.response() {
        Ok(selected) => selected,
        Err(ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)) => {
            // The user cancelled the dialog — a silent no-op, like rfd.
            return Ok(None);
        }
        Err(e) => {
            return Err(CommandError::internal(
                "export.dialog_failed",
                vec![e.to_string()],
            ));
        }
    };

    let path = selected
        .uris()
        .first()
        .and_then(|uri| uri.to_file_path().ok())
        .map(|p| p.to_string_lossy().into_owned());
    // The choice's selected key ("wav"/"opus"); an implementation that
    // dropped the combo falls back to the stored format via `None`.
    let format = selected
        .choices()
        .iter()
        .find(|(key, _)| key == "format")
        .map(|(_, value)| value.to_string())
        .filter(|value| value == "wav" || value == "opus");
    Ok(path.map(|p| (p, format)))
}

/// Other platforms (Windows): rfd's native dialog with both filters —
/// Win32 rewrites the typed extension to the selected filter's on save, so
/// the resulting extension carries the format decision back to
/// [`normalize_export_target`] (`None` = no reported choice).
#[cfg(not(target_os = "linux"))]
async fn run_save_dialog(entry_id: &str) -> CmdResult<Option<(String, Option<String>)>> {
    let default_name = format!("ruvox-{entry_id}.opus");
    tokio::task::spawn_blocking(move || {
        let path = rfd::FileDialog::new()
            .set_file_name(&default_name)
            .add_filter("Ogg Opus", &["opus"])
            .add_filter("WAV", &["wav"])
            .save_file()
            .map(|p| p.to_string_lossy().into_owned());
        path.map(|p| (p, None))
    })
    .await
    .map_err(|e| {
        CommandError::internal("export.dialog_panicked", vec![]).with_message(e.to_string())
    })?
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
    fn stored_ext_defaults_to_opus_without_extension() {
        assert_eq!(
            stored_ext_of("550e8400-e29b-41d4-a716-446655440000.opus"),
            "opus"
        );
        assert_eq!(
            stored_ext_of("550e8400-e29b-41d4-a716-446655440000.wav"),
            "wav"
        );
        assert_eq!(stored_ext_of("no-extension-name"), "opus");
    }

    #[test]
    fn normalize_with_reported_format_keeps_a_matching_extension() {
        assert_eq!(
            normalize_export_target("/tmp/ruvox-a.opus", Some("opus"), "opus"),
            "/tmp/ruvox-a.opus"
        );
        assert_eq!(
            normalize_export_target("/tmp/ruvox-a.WAV", Some("wav"), "opus"),
            "/tmp/ruvox-a.WAV",
            "case-insensitive match is kept, not rewritten"
        );
    }

    #[test]
    fn normalize_with_reported_format_wins_over_the_typed_name() {
        // The dialog's «Формат» combo is the explicit choice: even a typed
        // .opus must not rename a WAV export behind its back.
        assert_eq!(
            normalize_export_target("/tmp/audio.opus", Some("wav"), "opus"),
            "/tmp/audio.wav"
        );
        assert_eq!(
            normalize_export_target("/tmp/audio.mp3", Some("opus"), "opus"),
            "/tmp/audio.opus",
            "a foreign extension must not name a file whose bytes are Opus"
        );
    }

    #[test]
    fn normalize_without_reported_format_keeps_recognized_extensions() {
        // Platforms whose dialog cannot report the choice: the typed
        // extension decides between the two supported formats.
        assert_eq!(
            normalize_export_target("/tmp/audio.wav", None, "opus"),
            "/tmp/audio.wav"
        );
        assert_eq!(
            normalize_export_target("/tmp/audio.mp3", None, "opus"),
            "/tmp/audio.opus"
        );
        assert_eq!(
            normalize_export_target("/tmp/audio", None, "wav"),
            "/tmp/audio.wav"
        );
    }
}
