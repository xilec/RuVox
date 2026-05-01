//! Tauri command handlers (IPC Layer 1: Frontend → Backend).
//!
//! All commands use `Result<T, CommandError>` so that errors are serialized as
//! typed JSON objects (`{ "type": "...", "message": "..." }`) which the frontend
//! can pattern-match on.

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Runtime, State, Wry};
use tracing::{info, warn};

use crate::pipeline::tracked_text::CharMapping;
use crate::pipeline::TTSPipeline;
use crate::state::AppState;
use crate::storage::schema::{
    EntryId, EntryStatus, TextEntry, UIConfig, UIConfigPatch, WordTimestamp,
};
use crate::storage::service::{StorageError, StorageService};
use crate::tts::piper::download::download_voice;
use crate::tts::{
    availability, AvailableEngines, CharMappingEntry, SynthesizeOutput, TtsEngine, TtsError,
};

// ── Error type ─────────────────────────────────────────────────────────────────

/// Typed error returned by all Tauri commands.
/// `#[serde(tag = "type")]` produces `{ "type": "not_found", "message": "..." }`.
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandError {
    #[error("not found: {message}")]
    NotFound { message: String },

    #[error("storage error: {message}")]
    StorageError { message: String },

    #[error("synthesis error: {message}")]
    SynthesisError { message: String },

    #[error("playback error: {message}")]
    PlaybackError { message: String },

    #[error("config error: {message}")]
    ConfigError { message: String },

    #[error("internal error: {message}")]
    Internal { message: String },
}

impl From<StorageError> for CommandError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::NotFound(id) => CommandError::NotFound {
                message: format!("entry not found: {id}"),
            },
            other => CommandError::StorageError {
                message: other.to_string(),
            },
        }
    }
}

impl From<TtsError> for CommandError {
    fn from(e: TtsError) -> Self {
        CommandError::SynthesisError {
            message: e.to_string(),
        }
    }
}

type CmdResult<T> = Result<T, CommandError>;

// ── Helper: emit entry_updated ─────────────────────────────────────────────────

fn emit_entry_updated<R: Runtime>(app: &AppHandle<R>, entry: &TextEntry) {
    let _ = app.emit("entry_updated", json!({ "entry": entry }));
}

// ── Helper: convert CharMapping to Vec<CharMappingEntry> ────────────────────────

fn char_mapping_to_entries(mapping: &CharMapping) -> Vec<CharMappingEntry> {
    mapping
        .char_map
        .iter()
        .enumerate()
        .map(|(norm_idx, &(orig_start, orig_end))| CharMappingEntry {
            norm_start: norm_idx,
            norm_end: norm_idx + 1,
            orig_start,
            orig_end,
        })
        .collect()
}

// ── Background synthesis ───────────────────────────────────────────────────────

/// Public alias used by the tray module to avoid duplicating the synthesis logic.
#[allow(clippy::too_many_arguments)]
pub fn spawn_synthesis_pub<R: Runtime + 'static>(
    app: AppHandle<R>,
    storage: Arc<StorageService>,
    tts: Arc<dyn TtsEngine>,
    piper_voices_dir: PathBuf,
    emitter: crate::tts::supervisor::Emitter,
    player: Arc<crate::player::Player<R>>,
    pipeline: Arc<parking_lot::Mutex<crate::pipeline::TTSPipeline>>,
    entry_id: EntryId,
    play_when_ready: bool,
) {
    spawn_synthesis(
        app,
        storage,
        tts,
        piper_voices_dir,
        emitter,
        player,
        pipeline,
        entry_id,
        play_when_ready,
    );
}

/// Distinct failure points for a synthesis task. Each variant maps to the
/// user-visible string written into `TextEntry.error_message`; `TtsFailed`
/// additionally triggers a `tts_error` event for the frontend toast.
#[derive(Debug)]
enum SynthesisError {
    PipelinePanic(String),
    EmptyText,
    TtsFailed(String),
}

impl SynthesisError {
    fn user_message(&self) -> String {
        match self {
            Self::PipelinePanic(msg) => format!("pipeline task panicked: {msg}"),
            Self::EmptyText => "нормализация вернула пустой текст".to_string(),
            Self::TtsFailed(msg) => msg.clone(),
        }
    }
}

/// Phase 1: run Rust text pipeline (CPU-bound, runs in blocking thread).
async fn run_normalization(
    pipeline: Arc<Mutex<TTSPipeline>>,
    original_text: String,
) -> Result<(String, CharMapping), SynthesisError> {
    let (normalized, mapping) = tokio::task::spawn_blocking(move || {
        let mut p = pipeline.lock();
        p.process_with_char_mapping(&original_text)
    })
    .await
    .map_err(|e| SynthesisError::PipelinePanic(e.to_string()))?;

    if normalized.is_empty() {
        return Err(SynthesisError::EmptyText);
    }
    Ok((normalized, mapping))
}

/// Phase 2: mark entry as `Processing` and emit `entry_updated`.
///
/// Best-effort: a failed `update_entry` is logged and ignored, so a temporary
/// `history.json` write hiccup does not abort synthesis.
fn mark_processing<R: Runtime>(
    storage: &StorageService,
    app: &AppHandle<R>,
    entry_id: &EntryId,
    normalized: &str,
) {
    let Some(mut entry) = storage.get_entry(entry_id) else {
        return;
    };
    entry.status = EntryStatus::Processing;
    entry.normalized_text = Some(normalized.to_string());
    if let Err(e) = storage.update_entry(entry.clone()) {
        warn!("failed to update entry to processing: {e}");
    }
    emit_entry_updated(app, &entry);
}

/// Phases 3–4: determine the WAV path / config / char-mapping inputs and
/// call `tts.synthesize`. Returns the synthesize output along with the
/// resolved WAV path and filename so [`finalize_audio_files`] can transcode
/// to Opus without rebuilding them.
///
/// When the engine returns `voice_not_installed` and the active engine is
/// Piper, the function auto-fetches the voice files via
/// [`crate::tts::piper::download::download_voice`] and retries once. The
/// retry runs only on Piper because Silero is bundled — its Python venv
/// already includes the model.
async fn synthesize_audio(
    tts: &dyn TtsEngine,
    storage: &StorageService,
    piper_voices_dir: &std::path::Path,
    emitter: &crate::tts::supervisor::Emitter,
    entry_id: &EntryId,
    normalized: String,
    mapping: &CharMapping,
) -> Result<(SynthesizeOutput, PathBuf, String), SynthesisError> {
    // ttsd writes WAV; finalize_audio_files transcodes it to Opus right after.
    let wav_filename = format!("{entry_id}.wav");
    let out_wav_path = storage.cache_dir().join("audio").join(&wav_filename);
    let out_wav = out_wav_path.to_string_lossy().into_owned();

    let config = storage.load_config().unwrap_or_default();
    let tts_char_mapping = if mapping.char_map.is_empty() {
        None
    } else {
        Some(char_mapping_to_entries(mapping))
    };

    // Voice id is engine-specific: Piper uses `piper_voice` (e.g. "ruslan"),
    // Silero uses `speaker` (e.g. "xenia"). Keeping them in two distinct
    // config fields means flipping engines preserves each side's choice.
    let voice = if config.engine == "silero" {
        config.speaker.clone()
    } else {
        config.piper_voice.clone()
    };

    let attempt = tts
        .synthesize(
            normalized.clone(),
            voice.clone(),
            config.sample_rate,
            out_wav.clone(),
            tts_char_mapping.clone(),
        )
        .await;

    let output = match attempt {
        Ok(o) => o,
        Err(TtsError::Ttsd { code, message })
            if code == "voice_not_installed" && config.engine == "piper" =>
        {
            info!("voice \"{voice}\" not installed; auto-downloading then retrying ({message})");
            crate::tts::piper::download::download_voice(piper_voices_dir, &voice, emitter)
                .await
                .map_err(|e| SynthesisError::TtsFailed(e.to_string()))?;
            tts.synthesize(
                normalized,
                voice,
                config.sample_rate,
                out_wav,
                tts_char_mapping,
            )
            .await
            .map_err(|e| SynthesisError::TtsFailed(e.to_string()))?
        }
        Err(e) => return Err(SynthesisError::TtsFailed(e.to_string())),
    };

    Ok((output, out_wav_path, wav_filename))
}

/// Phases 5 + 5b: persist word timestamps and transcode WAV → Opus.
///
/// Both steps are best-effort: timestamp save failure yields `None`; opus
/// encode failure (or panic) keeps the original WAV filename so playback
/// still works.
async fn finalize_audio_files(
    storage: &StorageService,
    entry_id: &EntryId,
    output: &SynthesizeOutput,
    out_wav_path: PathBuf,
    wav_filename: &str,
) -> (Option<String>, String) {
    let tts_words: Vec<WordTimestamp> = output
        .timestamps
        .iter()
        .map(|w| WordTimestamp {
            word: w.word.clone(),
            start: w.start,
            end: w.end,
            original_pos: w.original_pos,
        })
        .collect();

    let ts_filename = match storage.save_timestamps(entry_id, &tts_words) {
        Ok(f) => Some(f),
        Err(e) => {
            warn!("failed to save timestamps: {e}");
            None
        }
    };

    let wav_path_for_encode = out_wav_path;
    let encode_result = tokio::task::spawn_blocking(move || {
        crate::audio::replace_wav_with_opus(&wav_path_for_encode)
    })
    .await;
    let audio_filename = match encode_result {
        Ok(Ok(opus_path)) => opus_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| wav_filename.to_string()),
        Ok(Err(e)) => {
            warn!("opus encode failed for {entry_id}, keeping wav: {e}");
            wav_filename.to_string()
        }
        Err(e) => {
            warn!("opus encode task panicked for {entry_id}, keeping wav: {e}");
            wav_filename.to_string()
        }
    };

    (ts_filename, audio_filename)
}

/// Phase 6: mark entry `Ready` with audio + timestamp paths and emit
/// `entry_updated`. Vanishing entries (deleted mid-synthesis) silently abort.
fn mark_ready_and_emit<R: Runtime>(
    storage: &StorageService,
    app: &AppHandle<R>,
    entry_id: &EntryId,
    output: &SynthesizeOutput,
    ts_filename: Option<String>,
    audio_filename: &str,
) {
    let Some(mut entry) = storage.get_entry(entry_id) else {
        return;
    };
    entry.status = EntryStatus::Ready;
    entry.audio_path = Some(audio_filename.to_string());
    entry.timestamps_path = ts_filename;
    entry.duration_sec = Some(output.duration_sec);
    entry.audio_generated_at = Some(chrono::Local::now().naive_local());

    if let Err(e) = storage.update_entry(entry.clone()) {
        warn!("failed to update entry to ready: {e}");
    }
    emit_entry_updated(app, &entry);
    info!("synthesis complete: entry_id={entry_id}");
}

/// Phase 7: kick off auto-play. Errors are logged and swallowed — failed
/// auto-play must not flip the entry into `Error`.
fn autoplay<R: Runtime>(
    player: &crate::player::Player<R>,
    audio_path: PathBuf,
    entry_id: &EntryId,
) {
    if let Err(e) = player.load(&audio_path, entry_id.to_string()) {
        warn!("auto-play load failed: {e}");
    } else if let Err(e) = player.play() {
        warn!("auto-play play failed: {e}");
    }
}

/// Run the full synthesis pipeline for `entry_id` in a background task.
#[allow(clippy::too_many_arguments)]
fn spawn_synthesis<R: Runtime + 'static>(
    app: AppHandle<R>,
    storage: Arc<StorageService>,
    tts: Arc<dyn TtsEngine>,
    piper_voices_dir: PathBuf,
    emitter: crate::tts::supervisor::Emitter,
    player: Arc<crate::player::Player<R>>,
    pipeline: Arc<Mutex<TTSPipeline>>,
    entry_id: EntryId,
    play_when_ready: bool,
) {
    tokio::spawn(async move {
        let Some(entry) = storage.get_entry(&entry_id) else {
            warn!("synthesis task: entry {entry_id} vanished before synthesis started");
            return;
        };

        let result: Result<(), SynthesisError> = async {
            let (normalized, mapping) =
                run_normalization(Arc::clone(&pipeline), entry.original_text.clone()).await?;
            mark_processing(&storage, &app, &entry_id, &normalized);
            let (output, out_wav_path, wav_filename) = synthesize_audio(
                tts.as_ref(),
                &storage,
                &piper_voices_dir,
                &emitter,
                &entry_id,
                normalized,
                &mapping,
            )
            .await?;
            let (ts_filename, audio_filename) =
                finalize_audio_files(&storage, &entry_id, &output, out_wav_path, &wav_filename)
                    .await;
            mark_ready_and_emit(
                &storage,
                &app,
                &entry_id,
                &output,
                ts_filename,
                &audio_filename,
            );
            if play_when_ready {
                let path = storage.cache_dir().join("audio").join(&audio_filename);
                autoplay(&player, path, &entry_id);
            }
            Ok(())
        }
        .await;

        if let Err(err) = result {
            let msg = err.user_message();
            tracing::error!("synthesis failed for {entry_id}: {msg}");
            set_entry_error(&storage, &app, &entry_id, &msg);
            if let SynthesisError::TtsFailed(tts_msg) = err {
                let _ = app.emit(
                    "tts_error",
                    json!({ "entry_id": entry_id.to_string(), "message": tts_msg }),
                );
            }
        }
    });
}

fn set_entry_error<R: Runtime>(
    storage: &StorageService,
    app: &AppHandle<R>,
    entry_id: &EntryId,
    message: &str,
) {
    if let Some(mut entry) = storage.get_entry(entry_id) {
        entry.status = EntryStatus::Error;
        entry.error_message = Some(message.to_string());
        let _ = storage.update_entry(entry.clone());
        emit_entry_updated(app, &entry);
    }
}

// ── Commands ───────────────────────────────────────────────────────────────────

/// Shared implementation for the two "add text to queue" commands below.
/// Rejects blank input, persists the entry, emits `entry_updated`, and
/// spawns background synthesis.
fn ingest_text(
    app: AppHandle<Wry>,
    state: &AppState,
    text: String,
    play_when_ready: bool,
) -> CmdResult<String> {
    if text.trim().is_empty() {
        return Err(CommandError::Internal {
            message: "в буфере обмена нет текста".to_string(),
        });
    }

    let entry = state.storage.add_entry(text).map_err(CommandError::from)?;
    let entry_id = entry.id;

    emit_entry_updated(&app, &entry);

    spawn_synthesis(
        app,
        Arc::clone(&state.storage),
        Arc::clone(&state.tts),
        state.piper_voices_dir.clone(),
        Arc::clone(&state.emitter),
        Arc::clone(&state.player),
        Arc::clone(&state.pipeline),
        entry_id,
        play_when_ready,
    );

    Ok(entry_id.to_string())
}

/// Add an entry to the queue from text already held by the frontend.
/// Preferred over `add_clipboard_entry` for UI paths, because WebKit's
/// Clipboard API is more robust on Wayland than the Rust-side `arboard`
/// crate (which silently fails with `ContentNotAvailable` for
/// WebKit-sourced clipboard data on KDE Plasma 6).
#[tauri::command]
pub async fn add_text_entry(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    text: String,
    play_when_ready: bool,
) -> CmdResult<String> {
    ingest_text(app, &state, text, play_when_ready)
}

/// Read text from the system clipboard and add a new entry to the queue.
/// Used by the tray menu, where no webview context is available.
/// Frontend code should prefer `add_text_entry` (see above).
#[tauri::command]
pub async fn add_clipboard_entry(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    play_when_ready: bool,
) -> CmdResult<String> {
    // Read clipboard on a blocking thread (required on Linux to avoid deadlock).
    let text = tokio::task::spawn_blocking(|| {
        let mut board = arboard::Clipboard::new().map_err(|e| CommandError::Internal {
            message: format!("clipboard init: {e}"),
        })?;
        board.get_text().map_err(|_| CommandError::Internal {
            message: "в буфере обмена нет текста".to_string(),
        })
    })
    .await
    .map_err(|e| CommandError::Internal {
        message: format!("clipboard task panicked: {e}"),
    })??;

    ingest_text(app, &state, text, play_when_ready)
}

/// Run the text normalization pipeline on `text` and return the normalized result.
///
/// Used by the preview dialog (FF 1.1) to show original ↔ normalized side-by-side
/// before the user confirms synthesis.
#[tauri::command]
pub async fn preview_normalize(
    state: State<'_, AppState>,
    text: String,
) -> CmdResult<PreviewNormalizeResult> {
    let pipeline = Arc::clone(&state.pipeline);
    let result = tokio::task::spawn_blocking(move || {
        let mut p = pipeline.lock();
        p.process_with_char_mapping(&text)
    })
    .await
    .map_err(|e| CommandError::Internal {
        message: format!("pipeline task panicked: {e}"),
    })?;

    let (normalized, _char_mapping) = result;
    Ok(PreviewNormalizeResult { normalized })
}

#[derive(Debug, Serialize)]
pub struct PreviewNormalizeResult {
    pub normalized: String,
}

/// Return all entries sorted by created_at descending.
#[tauri::command]
pub async fn get_entries(state: State<'_, AppState>) -> CmdResult<Vec<TextEntry>> {
    Ok(state.storage.get_all_entries())
}

/// Return a single entry by ID, or null if not found.
#[tauri::command]
pub async fn get_entry(state: State<'_, AppState>, id: String) -> CmdResult<Option<TextEntry>> {
    let uuid = parse_entry_id(&id)?;
    Ok(state.storage.get_entry(&uuid))
}

/// Delete an entry and its audio + timestamps files.
#[tauri::command]
pub async fn delete_entry(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    let uuid = parse_entry_id(&id)?;

    // Stop playback if this entry is playing.  Player::stop emits
    // playback_stopped on its own, so we do not re-emit here.
    if state.player.current_entry_id().as_deref() == Some(&id) {
        let _ = state.player.stop();
    }

    state
        .storage
        .delete_entry(&uuid)
        .map_err(CommandError::from)?;

    Ok(())
}

/// Delete only the audio files for an entry, resetting its status to pending.
#[tauri::command]
pub async fn delete_audio(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<()> {
    let uuid = parse_entry_id(&id)?;
    state
        .storage
        .delete_audio(&uuid)
        .map_err(CommandError::from)?;

    if let Some(entry) = state.storage.get_entry(&uuid) {
        emit_entry_updated(&app, &entry);
    }
    Ok(())
}

/// Regenerate audio for an existing entry: drop its current audio + timestamps,
/// reset status to `Pending`, and re-run the synthesis pipeline. Useful when
/// the user has changed `speaker`, `speech_rate`, or other normalization
/// settings and wants the cached audio to reflect them.
///
/// Rejects the call if the entry is currently being synthesized — re-entering
/// `spawn_synthesis` for the same id would race with the in-flight task.
#[tauri::command]
pub async fn regenerate_entry(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<()> {
    let uuid = parse_entry_id(&id)?;

    let entry = state
        .storage
        .get_entry(&uuid)
        .ok_or_else(|| CommandError::NotFound {
            message: format!("entry not found: {id}"),
        })?;

    if entry.status == EntryStatus::Processing {
        return Err(CommandError::SynthesisError {
            message: "запись уже синтезируется".to_string(),
        });
    }

    // If this entry is currently playing, stop playback so the about-to-be-
    // deleted audio file is not held open by the player.
    if state.player.current_entry_id().as_deref() == Some(&id) {
        let _ = state.player.stop();
    }

    state
        .storage
        .delete_audio(&uuid)
        .map_err(CommandError::from)?;

    let mut entry = state
        .storage
        .get_entry(&uuid)
        .ok_or_else(|| CommandError::NotFound {
            message: format!("entry vanished after delete_audio: {id}"),
        })?;
    entry.was_regenerated = true;
    entry.error_message = None;
    state
        .storage
        .update_entry(entry.clone())
        .map_err(CommandError::from)?;
    emit_entry_updated(&app, &entry);

    spawn_synthesis(
        app,
        Arc::clone(&state.storage),
        Arc::clone(&state.tts),
        state.piper_voices_dir.clone(),
        Arc::clone(&state.emitter),
        Arc::clone(&state.player),
        Arc::clone(&state.pipeline),
        uuid,
        false,
    );

    Ok(())
}

/// Cancel an in-progress or queued synthesis job.
/// Currently marks entry as pending (mid-request abort is not supported since
/// the TTS supervisor serialises all requests into a single channel).
#[tauri::command]
pub async fn cancel_synthesis(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<()> {
    let uuid = parse_entry_id(&id)?;
    let mut entry = state
        .storage
        .get_entry(&uuid)
        .ok_or_else(|| CommandError::NotFound {
            message: format!("entry not found: {id}"),
        })?;

    entry.status = EntryStatus::Pending;
    state
        .storage
        .update_entry(entry.clone())
        .map_err(CommandError::from)?;
    emit_entry_updated(&app, &entry);
    Ok(())
}

/// Start playback of a ready entry.
#[tauri::command]
pub async fn play_entry(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    let uuid = parse_entry_id(&id)?;
    let entry = state
        .storage
        .get_entry(&uuid)
        .ok_or_else(|| CommandError::NotFound {
            message: format!("entry not found: {id}"),
        })?;

    if entry.status != EntryStatus::Ready {
        return Err(CommandError::PlaybackError {
            message: format!("entry {id} is not ready (status: {:?})", entry.status),
        });
    }

    let path = state
        .storage
        .get_audio_path(&uuid)
        .ok_or_else(|| CommandError::PlaybackError {
            message: format!("audio file missing for entry {id}"),
        })?;

    state
        .player
        .load(&path, id.clone())
        .map_err(|e| CommandError::PlaybackError {
            message: e.to_string(),
        })?;

    state
        .player
        .play()
        .map_err(|e| CommandError::PlaybackError {
            message: e.to_string(),
        })?;

    Ok(())
}

/// Pause the currently playing entry.
#[tauri::command]
pub async fn pause_playback(state: State<'_, AppState>) -> CmdResult<()> {
    state
        .player
        .pause()
        .map_err(|e| CommandError::PlaybackError {
            message: e.to_string(),
        })
}

/// Resume playback from the paused position.
#[tauri::command]
pub async fn resume_playback(state: State<'_, AppState>) -> CmdResult<()> {
    state
        .player
        .resume()
        .map_err(|e| CommandError::PlaybackError {
            message: e.to_string(),
        })
}

/// Stop playback entirely.
#[tauri::command]
pub async fn stop_playback(state: State<'_, AppState>) -> CmdResult<()> {
    state
        .player
        .stop()
        .map_err(|e| CommandError::PlaybackError {
            message: e.to_string(),
        })
}

/// Seek to an absolute position in the current audio.
#[tauri::command]
pub async fn seek_to(state: State<'_, AppState>, position_sec: f64) -> CmdResult<()> {
    state
        .player
        .seek(position_sec)
        .map_err(|e| CommandError::PlaybackError {
            message: e.to_string(),
        })
}

/// Set playback speed (0.5–2.0). Persisted to UIConfig.speech_rate.
#[tauri::command]
pub async fn set_speed(state: State<'_, AppState>, speed: f32) -> CmdResult<()> {
    if !(0.5..=2.0).contains(&speed) {
        return Err(CommandError::ConfigError {
            message: format!("speed {speed} is out of range [0.5, 2.0]"),
        });
    }

    state
        .player
        .set_speed(speed)
        .map_err(|e| CommandError::PlaybackError {
            message: e.to_string(),
        })?;

    // Persist to config.
    let mut config = state.storage.load_config().unwrap_or_default();
    config.speech_rate = speed as f64;
    if let Err(e) = state.storage.save_config(&config) {
        warn!("failed to persist speech_rate: {e}");
    }

    Ok(())
}

/// Set playback volume (0.0–1.0). Not persisted.
#[tauri::command]
pub async fn set_volume(state: State<'_, AppState>, volume: f32) -> CmdResult<()> {
    if !(0.0..=1.0).contains(&volume) {
        return Err(CommandError::ConfigError {
            message: format!("volume {volume} is out of range [0.0, 1.0]"),
        });
    }

    state
        .player
        .set_volume(volume)
        .map_err(|e| CommandError::PlaybackError {
            message: e.to_string(),
        })
}

/// Return the current application configuration.
#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> CmdResult<UIConfig> {
    state.storage.load_config().map_err(CommandError::from)
}

/// Download a Piper voice on user demand. Idempotent — already-present
/// files are skipped. Progress is delivered via the
/// `voice_download_started` / `voice_download_progress` /
/// `voice_download_finished` events; the `Result` here only reports the
/// final outcome so the frontend can show one final notification.
#[tauri::command]
pub async fn download_piper_voice(state: State<'_, AppState>, voice_id: String) -> CmdResult<()> {
    let voices_dir = state.piper_voices_dir.clone();
    let emitter = Arc::clone(&state.emitter);
    download_voice(&voices_dir, &voice_id, &emitter)
        .await
        .map_err(CommandError::from)
}

/// Probe which TTS engines can be selected on the running system.
///
/// Piper is in-process and always available. Silero requires the `ttsd/`
/// Python package and the `uv` toolchain — see [`tts::availability`].
/// Cheap (filesystem stat + one `uv --version` exec); safe to call on
/// every Settings dialog open.
#[tauri::command]
pub async fn get_available_engines(state: State<'_, AppState>) -> CmdResult<AvailableEngines> {
    let ttsd_dir = state.ttsd_dir.clone();
    tokio::task::spawn_blocking(move || availability::probe(&ttsd_dir))
        .await
        .map_err(|e| CommandError::Internal {
            message: format!("availability probe panicked: {e}"),
        })
}

/// Merge a partial config patch into the current configuration, swap the
/// active TTS engine if needed, and persist. The engine swap runs *before*
/// the config is saved — if the user picked a Silero stack we cannot spawn,
/// the call returns an error and the previous config stays on disk.
#[tauri::command]
pub async fn update_config(state: State<'_, AppState>, patch: UIConfigPatch) -> CmdResult<()> {
    let mut config = state.storage.load_config().unwrap_or_default();
    apply_config_patch(&mut config, patch);

    state
        .engine_switcher
        .apply_config(&config.engine, &config.piper_voice)
        .await
        .map_err(|e| CommandError::ConfigError {
            message: format!("не удалось переключить движок: {e}"),
        })?;

    state
        .storage
        .save_config(&config)
        .map_err(CommandError::from)
}

/// Load and return word timestamps for an entry.
#[tauri::command]
pub async fn get_timestamps(
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<Vec<WordTimestamp>> {
    let uuid = parse_entry_id(&id)?;

    // Verify entry exists.
    if state.storage.get_entry(&uuid).is_none() {
        return Err(CommandError::NotFound {
            message: format!("entry not found: {id}"),
        });
    }

    let timestamps = state
        .storage
        .load_timestamps(&uuid)
        .map_err(CommandError::from)?
        .unwrap_or_default();

    Ok(timestamps)
}

/// What "fits in the cache" means for this clear_cache invocation.
#[derive(Debug, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CleanupMode {
    /// Trim the oldest entries until the cache fits in `target_mb`.
    SizeLimit { target_mb: u32 },
    /// Drop everything: every entry's audio (and texts when `delete_texts` is true).
    All,
}

#[derive(Debug, Deserialize)]
pub struct ClearCacheArgs {
    pub mode: CleanupMode,
    /// `false` → keep entries in history with `audio_path: null`.
    /// `true`  → remove entries from history entirely.
    #[serde(default)]
    pub delete_texts: bool,
}

#[derive(Serialize)]
pub struct ClearCacheResult {
    pub deleted_files: u32,
    pub deleted_entries: u32,
    pub freed_bytes: u64,
}

/// Sweep orphan files in `audio/`, then evict entries (size-based or wholesale)
/// according to `args.mode`. With `delete_texts = true`, evicted entries are
/// removed from `history.json`; otherwise only their audio is dropped.
/// Always sweeps orphans regardless of `mode` / `delete_texts`.
#[tauri::command]
pub async fn clear_cache(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    args: ClearCacheArgs,
) -> CmdResult<ClearCacheResult> {
    let storage = Arc::clone(&state.storage);
    let mode = args.mode;
    let delete_texts = args.delete_texts;

    // File I/O is blocking by nature — keep the async runtime free.
    let (sweep, evict) = tokio::task::spawn_blocking(move || -> Result<_, StorageError> {
        let sweep = storage.sweep_orphans()?;
        let evict = match mode {
            CleanupMode::SizeLimit { target_mb } => {
                storage.evict_to_size((target_mb as u64) * 1024 * 1024, delete_texts)?
            }
            CleanupMode::All => storage.evict_all(delete_texts)?,
        };
        Ok((sweep, evict))
    })
    .await
    .map_err(|e| CommandError::Internal {
        message: format!("clear_cache task panicked: {e}"),
    })??;

    for id in &evict.updated_ids {
        if let Some(entry) = state.storage.get_entry(id) {
            emit_entry_updated(&app, &entry);
        }
    }
    for id in &evict.removed_ids {
        let _ = app.emit("entry_removed", json!({ "id": id }));
    }

    info!(
        "clear_cache: sweep_files={}, evict_files={}, evict_entries={}, freed={} bytes",
        sweep.deleted_files,
        evict.deleted_files,
        evict.deleted_entries,
        sweep.freed_bytes + evict.freed_bytes,
    );

    Ok(ClearCacheResult {
        deleted_files: sweep.deleted_files + evict.deleted_files,
        deleted_entries: evict.deleted_entries,
        freed_bytes: sweep.freed_bytes + evict.freed_bytes,
    })
}

/// Return current cache size information.
#[tauri::command]
pub async fn get_cache_stats(state: State<'_, AppState>) -> CmdResult<CacheSizeInfo> {
    let total_bytes = state.storage.get_cache_size().map_err(CommandError::from)?;
    let audio_file_count = state
        .storage
        .get_audio_count()
        .map_err(CommandError::from)?;
    Ok(CacheSizeInfo {
        total_bytes,
        audio_file_count,
    })
}

#[derive(Serialize)]
pub struct CacheSizeInfo {
    pub total_bytes: u64,
    pub audio_file_count: u32,
}

/// Absolute path to the on-disk cache directory (`~/.cache/ruvox/` by default,
/// or wherever `XDG_CACHE_HOME`/`dirs::cache_dir()` resolved to at startup).
/// The frontend uses this to display the path in Settings and to pass it to
/// `revealItemInDir` for opening the folder in the OS file manager.
#[tauri::command]
pub async fn get_cache_dir(state: State<'_, AppState>) -> CmdResult<String> {
    Ok(state.storage.cache_dir().to_string_lossy().into_owned())
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn parse_entry_id(s: &str) -> CmdResult<EntryId> {
    s.parse::<uuid::Uuid>().map_err(|e| CommandError::NotFound {
        message: format!("invalid entry id '{s}': {e}"),
    })
}

fn apply_config_patch(config: &mut UIConfig, patch: UIConfigPatch) {
    if let Some(v) = patch.speaker {
        config.speaker = v;
    }
    if let Some(v) = patch.sample_rate {
        config.sample_rate = v;
    }
    if let Some(v) = patch.speech_rate {
        config.speech_rate = v;
    }
    if let Some(v) = patch.notify_on_ready {
        config.notify_on_ready = v;
    }
    if let Some(v) = patch.notify_on_error {
        config.notify_on_error = v;
    }
    if let Some(v) = patch.text_format {
        config.text_format = v;
    }
    if let Some(v) = patch.max_cache_size_mb {
        config.max_cache_size_mb = v;
    }
    if let Some(v) = patch.code_block_mode {
        config.code_block_mode = v;
    }
    if let Some(v) = patch.read_operators {
        config.read_operators = v;
    }
    if let Some(v) = patch.theme {
        config.theme = v;
    }
    if let Some(v) = patch.player_hotkeys {
        config.player_hotkeys = v;
    }
    if let Some(v) = patch.window_geometry {
        config.window_geometry = v;
    }
    if let Some(v) = patch.preview_dialog_enabled {
        config.preview_dialog_enabled = v;
    }
    if let Some(v) = patch.engine {
        config.engine = v;
    }
    if let Some(v) = patch.piper_voice {
        config.piper_voice = v;
    }
}

#[cfg(test)]
mod synthesis_tests {
    use super::*;
    use tempfile::TempDir;

    fn make_storage() -> (StorageService, TempDir) {
        let dir = TempDir::new().unwrap();
        let svc = StorageService::with_cache_dir(dir.path().to_path_buf()).unwrap();
        (svc, dir)
    }

    #[tokio::test]
    async fn run_normalization_returns_normalized_text_and_mapping() {
        let pipeline = Arc::new(Mutex::new(TTSPipeline::new()));
        let (normalized, _mapping) = run_normalization(pipeline, "Привет мир".to_string())
            .await
            .unwrap();
        assert!(!normalized.is_empty());
    }

    #[tokio::test]
    async fn run_normalization_flags_empty_input_as_empty_text() {
        let pipeline = Arc::new(Mutex::new(TTSPipeline::new()));
        let err = run_normalization(pipeline, String::new())
            .await
            .unwrap_err();
        assert!(matches!(err, SynthesisError::EmptyText));
    }

    #[tokio::test]
    async fn finalize_audio_files_falls_back_to_wav_when_opus_encode_fails() {
        let (storage, _dir) = make_storage();
        let entry = storage.add_entry("text".to_string()).unwrap();
        let id = entry.id;

        // The encoder requires a valid RIFF header; bogus bytes force the
        // best-effort path that keeps the .wav file as audio_filename.
        let wav_filename = format!("{id}.wav");
        let wav_path = storage.cache_dir().join("audio").join(&wav_filename);
        std::fs::write(&wav_path, b"not a wav file").unwrap();

        let output = SynthesizeOutput {
            timestamps: Vec::new(),
            duration_sec: 1.0,
        };

        let (ts_filename, audio_filename) =
            finalize_audio_files(&storage, &id, &output, wav_path.clone(), &wav_filename).await;
        assert!(ts_filename.is_some());
        assert_eq!(audio_filename, wav_filename);
        // .wav file is left untouched on encode failure (replace_wav_with_opus contract).
        assert!(wav_path.exists());
    }

    #[test]
    fn synthesis_error_user_messages_match_legacy_strings() {
        assert_eq!(
            SynthesisError::EmptyText.user_message(),
            "нормализация вернула пустой текст",
        );
        assert_eq!(
            SynthesisError::PipelinePanic("boom".into()).user_message(),
            "pipeline task panicked: boom",
        );
        assert_eq!(
            SynthesisError::TtsFailed("ttsd died".into()).user_message(),
            "ttsd died",
        );
    }
}
