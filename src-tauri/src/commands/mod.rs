//! Tauri command handlers (IPC Layer 1: Frontend → Backend).
//!
//! All commands use `Result<T, CommandError>` so that errors are serialized as
//! typed JSON objects (`{ "type": "...", "message": "..." }`) which the frontend
//! can pattern-match on.

use std::sync::Arc;

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Runtime, State, Wry};
use tracing::{info, warn};

use crate::pipeline::tracked_text::CharMapping;
use crate::state::AppState;
use crate::storage::schema::{EntryId, EntryStatus, TextEntry, UIConfig, UIConfigPatch, WordTimestamp};
use crate::storage::service::{StorageError, StorageService};
use crate::tts::{CharMappingEntry, TtsError};

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
pub fn spawn_synthesis_pub<R: Runtime + 'static>(
    app: AppHandle<R>,
    storage: Arc<StorageService>,
    tts: Arc<crate::tts::TtsSubprocess>,
    player: Arc<crate::player::Player<R>>,
    pipeline: Arc<parking_lot::Mutex<crate::pipeline::TTSPipeline>>,
    entry_id: EntryId,
    play_when_ready: bool,
) {
    spawn_synthesis(app, storage, tts, player, pipeline, entry_id, play_when_ready);
}

/// Run the full synthesis pipeline for `entry_id` in a background task.
///
/// Steps:
/// 1. Run Rust pipeline → normalized text + char mapping.
/// 2. Update entry status → processing.
/// 3. Call ttsd synthesize.
/// 4. Save WAV + timestamps.
/// 5. Update entry status → ready.
/// 6. Optionally start playback.
fn spawn_synthesis<R: Runtime + 'static>(
    app: AppHandle<R>,
    storage: Arc<StorageService>,
    tts: Arc<crate::tts::TtsSubprocess>,
    player: Arc<crate::player::Player<R>>,
    pipeline: Arc<parking_lot::Mutex<crate::pipeline::TTSPipeline>>,
    entry_id: EntryId,
    play_when_ready: bool,
) {
    tokio::spawn(async move {
        // Load entry — bail silently if already deleted.
        let entry = match storage.get_entry(&entry_id) {
            Some(e) => e,
            None => {
                warn!("synthesis task: entry {entry_id} vanished before synthesis started");
                return;
            }
        };

        // Use edited_text as source if the user has saved an override; fall back
        // to original_text so the pipeline always receives something meaningful.
        let source_text = entry
            .edited_text
            .clone()
            .unwrap_or_else(|| entry.original_text.clone());

        // Phase 1: run Rust text pipeline (CPU-bound, run in blocking thread).
        let pipeline_clone = Arc::clone(&pipeline);
        let original_for_pipeline = source_text.clone();
        let pipeline_result = tokio::task::spawn_blocking(move || {
            let mut p = pipeline_clone.lock();
            p.process_with_char_mapping(&original_for_pipeline)
        })
        .await;

        let (normalized_text, char_mapping) = match pipeline_result {
            Ok(result) => result,
            Err(e) => {
                let msg = format!("pipeline task panicked: {e}");
                tracing::error!("{msg}");
                set_entry_error(&storage, &app, &entry_id, &msg);
                return;
            }
        };

        if normalized_text.is_empty() {
            let msg = "нормализация вернула пустой текст".to_string();
            set_entry_error(&storage, &app, &entry_id, &msg);
            return;
        }

        // Phase 2: update entry status → processing.
        let mut processing_entry = match storage.get_entry(&entry_id) {
            Some(e) => e,
            None => return,
        };
        processing_entry.status = EntryStatus::Processing;
        processing_entry.normalized_text = Some(normalized_text.clone());
        if let Err(e) = storage.update_entry(processing_entry.clone()) {
            warn!("failed to update entry to processing: {e}");
        }
        emit_entry_updated(&app, &processing_entry);

        // Phase 3: determine output WAV path.
        let wav_filename = format!("{entry_id}.wav");
        let out_wav = storage
            .cache_dir()
            .join("audio")
            .join(&wav_filename)
            .to_string_lossy()
            .into_owned();

        // Load config for speaker / sample_rate.
        let config = storage.load_config().unwrap_or_default();

        // Build char_mapping entries for ttsd.
        let tts_char_mapping = if char_mapping.char_map.is_empty() {
            None
        } else {
            Some(char_mapping_to_entries(&char_mapping))
        };

        // Phase 4: call ttsd.
        let synth_result = tts
            .synthesize(
                normalized_text.clone(),
                config.speaker.clone(),
                config.sample_rate,
                out_wav.clone(),
                tts_char_mapping,
            )
            .await;

        match synth_result {
            Ok(output) => {
                // Phase 5: save timestamps.
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

                let ts_filename = match storage.save_timestamps(&entry_id, &tts_words) {
                    Ok(f) => f,
                    Err(e) => {
                        warn!("failed to save timestamps: {e}");
                        String::new()
                    }
                };

                // Phase 6: update entry → ready.
                let mut ready_entry = match storage.get_entry(&entry_id) {
                    Some(e) => e,
                    None => return,
                };
                ready_entry.status = EntryStatus::Ready;
                ready_entry.audio_path = Some(wav_filename.clone());
                ready_entry.timestamps_path = if ts_filename.is_empty() {
                    None
                } else {
                    Some(ts_filename)
                };
                ready_entry.duration_sec = Some(output.duration_sec);
                ready_entry.audio_generated_at = Some(chrono::Local::now().naive_local());

                if let Err(e) = storage.update_entry(ready_entry.clone()) {
                    warn!("failed to update entry to ready: {e}");
                }
                emit_entry_updated(&app, &ready_entry);

                info!("synthesis complete: entry_id={entry_id}");

                // Phase 7: auto-play if requested.
                if play_when_ready {
                    let path = storage
                        .cache_dir()
                        .join("audio")
                        .join(&wav_filename);
                    if let Err(e) = player.load(&path, entry_id.to_string()) {
                        warn!("auto-play load failed: {e}");
                    } else if let Err(e) = player.play() {
                        warn!("auto-play play failed: {e}");
                    }
                }
            }
            Err(tts_err) => {
                let msg = tts_err.to_string();
                tracing::error!("synthesis failed for {entry_id}: {msg}");
                set_entry_error(&storage, &app, &entry_id, &msg);
                let _ = app.emit(
                    "tts_error",
                    json!({ "entry_id": entry_id.to_string(), "message": msg }),
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

/// Read text from the system clipboard and add a new entry to the queue.
/// Triggers TTS synthesis immediately in the background.
#[tauri::command]
pub async fn add_clipboard_entry(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    play_when_ready: bool,
) -> CmdResult<String> {
    // Read clipboard (must be on a blocking thread on Linux to avoid deadlock).
    let text = tokio::task::spawn_blocking(|| {
        let mut board = arboard::Clipboard::new()
            .map_err(|e| CommandError::Internal { message: format!("clipboard init: {e}") })?;
        board
            .get_text()
            .map_err(|e| CommandError::Internal { message: format!("clipboard read: {e}") })
    })
    .await
    .map_err(|e| CommandError::Internal { message: format!("clipboard task panicked: {e}") })??;

    if text.trim().is_empty() {
        return Err(CommandError::Internal {
            message: "буфер обмена пуст".to_string(),
        });
    }

    // Save entry (status: pending).
    let entry = state.storage.add_entry(text).map_err(CommandError::from)?;
    let entry_id = entry.id;

    emit_entry_updated(&app, &entry);

    // Spawn background synthesis.
    spawn_synthesis(
        app,
        Arc::clone(&state.storage),
        Arc::clone(&state.tts),
        Arc::clone(&state.player),
        Arc::clone(&state.pipeline),
        entry_id,
        play_when_ready,
    );

    Ok(entry_id.to_string())
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
pub async fn get_entries(
    state: State<'_, AppState>,
) -> CmdResult<Vec<TextEntry>> {
    Ok(state.storage.get_all_entries())
}

/// Return a single entry by ID, or null if not found.
#[tauri::command]
pub async fn get_entry(
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<Option<TextEntry>> {
    let uuid = parse_entry_id(&id)?;
    Ok(state.storage.get_entry(&uuid))
}

/// Delete an entry and its audio + timestamps files.
#[tauri::command]
pub async fn delete_entry(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<()> {
    let uuid = parse_entry_id(&id)?;

    // Stop playback if this entry is playing.
    if state.player.current_entry_id().as_deref() == Some(&id) {
        let _ = state.player.stop();
    }

    state.storage.delete_entry(&uuid).map_err(CommandError::from)?;

    let _ = app.emit("playback_stopped", json!({}));

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
    state.storage.delete_audio(&uuid).map_err(CommandError::from)?;

    if let Some(entry) = state.storage.get_entry(&uuid) {
        emit_entry_updated(&app, &entry);
    }
    Ok(())
}

/// Cancel an in-progress or queued synthesis job.
/// Currently marks entry as pending (mid-request abort is not supported since
/// TtsSubprocess serializes all requests into a single channel).
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
        .ok_or_else(|| CommandError::NotFound { message: format!("entry not found: {id}") })?;

    entry.status = EntryStatus::Pending;
    state.storage.update_entry(entry.clone()).map_err(CommandError::from)?;
    emit_entry_updated(&app, &entry);
    Ok(())
}

/// Start playback of a ready entry.
#[tauri::command]
pub async fn play_entry(
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<()> {
    let uuid = parse_entry_id(&id)?;
    let entry = state
        .storage
        .get_entry(&uuid)
        .ok_or_else(|| CommandError::NotFound { message: format!("entry not found: {id}") })?;

    if entry.status != EntryStatus::Ready {
        return Err(CommandError::PlaybackError {
            message: format!("entry {id} is not ready (status: {:?})", entry.status),
        });
    }

    let path = state.storage.get_audio_path(&uuid).ok_or_else(|| {
        CommandError::PlaybackError { message: format!("audio file missing for entry {id}") }
    })?;

    state
        .player
        .load(&path, id.clone())
        .map_err(|e| CommandError::PlaybackError { message: e.to_string() })?;

    state
        .player
        .play()
        .map_err(|e| CommandError::PlaybackError { message: e.to_string() })?;

    Ok(())
}

/// Pause the currently playing entry.
#[tauri::command]
pub async fn pause_playback(state: State<'_, AppState>) -> CmdResult<()> {
    state
        .player
        .pause()
        .map_err(|e| CommandError::PlaybackError { message: e.to_string() })
}

/// Resume playback from the paused position.
#[tauri::command]
pub async fn resume_playback(state: State<'_, AppState>) -> CmdResult<()> {
    state
        .player
        .resume()
        .map_err(|e| CommandError::PlaybackError { message: e.to_string() })
}

/// Stop playback entirely.
#[tauri::command]
pub async fn stop_playback(state: State<'_, AppState>) -> CmdResult<()> {
    state
        .player
        .stop()
        .map_err(|e| CommandError::PlaybackError { message: e.to_string() })
}

/// Seek to an absolute position in the current audio.
#[tauri::command]
pub async fn seek_to(
    state: State<'_, AppState>,
    position_sec: f64,
) -> CmdResult<()> {
    state
        .player
        .seek(position_sec)
        .map_err(|e| CommandError::PlaybackError { message: e.to_string() })
}

/// Set playback speed (0.5–2.0). Persisted to UIConfig.speech_rate.
#[tauri::command]
pub async fn set_speed(
    state: State<'_, AppState>,
    speed: f32,
) -> CmdResult<()> {
    if !(0.5..=2.0).contains(&speed) {
        return Err(CommandError::ConfigError {
            message: format!("speed {speed} is out of range [0.5, 2.0]"),
        });
    }

    state
        .player
        .set_speed(speed)
        .map_err(|e| CommandError::PlaybackError { message: e.to_string() })?;

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
pub async fn set_volume(
    state: State<'_, AppState>,
    volume: f32,
) -> CmdResult<()> {
    if !(0.0..=1.0).contains(&volume) {
        return Err(CommandError::ConfigError {
            message: format!("volume {volume} is out of range [0.0, 1.0]"),
        });
    }

    state
        .player
        .set_volume(volume)
        .map_err(|e| CommandError::PlaybackError { message: e.to_string() })
}

/// Return the current application configuration.
#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> CmdResult<UIConfig> {
    state.storage.load_config().map_err(CommandError::from)
}

/// Merge a partial config patch into the current configuration and persist.
#[tauri::command]
pub async fn update_config(
    state: State<'_, AppState>,
    patch: UIConfigPatch,
) -> CmdResult<()> {
    let mut config = state.storage.load_config().unwrap_or_default();
    apply_config_patch(&mut config, patch);
    state.storage.save_config(&config).map_err(CommandError::from)
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
        return Err(CommandError::NotFound { message: format!("entry not found: {id}") });
    }

    let timestamps = state
        .storage
        .load_timestamps(&uuid)
        .map_err(CommandError::from)?
        .unwrap_or_default();

    Ok(timestamps)
}

/// Delete audio files older than auto_cleanup_days, or all if force=true.
#[tauri::command]
pub async fn clear_cache(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    force: bool,
) -> CmdResult<ClearCacheResult> {
    let config = state.storage.load_config().unwrap_or_default();
    let mut deleted_files: u32 = 0;
    let mut freed_bytes: u64 = 0;

    let entries = state.storage.get_all_entries();
    for entry in entries {
        let should_delete = if force {
            entry.audio_path.is_some()
        } else if config.auto_cleanup_days > 0 {
            let cutoff = chrono::Local::now().naive_local()
                - chrono::Duration::days(config.auto_cleanup_days as i64);
            entry.created_at < cutoff && entry.audio_path.is_some()
        } else {
            false
        };

        if should_delete {
            // Measure file size before deletion.
            if let Some(ref filename) = entry.audio_path {
                let path = state.storage.cache_dir().join("audio").join(filename);
                if let Ok(meta) = std::fs::metadata(&path) {
                    freed_bytes += meta.len();
                }
                deleted_files += 1;
            }

            if let Err(e) = state.storage.delete_audio(&entry.id) {
                warn!("clear_cache: failed to delete audio for {}: {e}", entry.id);
                continue;
            }

            if let Some(updated) = state.storage.get_entry(&entry.id) {
                emit_entry_updated(&app, &updated);
            }
        }
    }

    Ok(ClearCacheResult { deleted_files, freed_bytes })
}

#[derive(Serialize)]
pub struct ClearCacheResult {
    pub deleted_files: u32,
    pub freed_bytes: u64,
}

/// Return current cache size information.
#[tauri::command]
pub async fn get_cache_stats(
    state: State<'_, AppState>,
) -> CmdResult<CacheSizeInfo> {
    let total_bytes = state.storage.get_cache_size().map_err(CommandError::from)?;
    let audio_file_count = state.storage.get_audio_count().map_err(CommandError::from)?;
    Ok(CacheSizeInfo { total_bytes, audio_file_count })
}

#[derive(Serialize)]
pub struct CacheSizeInfo {
    pub total_bytes: u64,
    pub audio_file_count: u32,
}

/// Update (or clear) the user-edited text override for an entry.
///
/// When `edited` is `Some(text)` the entry's `edited_text` is updated and
/// persisted.  When `edited` is `None` the override is cleared, causing the
/// next synthesis to use `normalized_text` / `original_text` again.
///
/// The command emits `entry_updated` so that the frontend immediately sees the
/// change without polling.
#[tauri::command]
pub async fn update_entry_edited_text(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    id: String,
    edited: Option<String>,
) -> CmdResult<()> {
    let uuid = parse_entry_id(&id)?;
    let mut entry = state
        .storage
        .get_entry(&uuid)
        .ok_or_else(|| CommandError::NotFound { message: format!("entry not found: {id}") })?;

    entry.edited_text = edited;
    state.storage.update_entry(entry.clone()).map_err(CommandError::from)?;

    info!("updated edited_text: entry_id={id}, has_override={}", entry.edited_text.is_some());
    emit_entry_updated(&app, &entry);
    Ok(())
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn parse_entry_id(s: &str) -> CmdResult<EntryId> {
    s.parse::<uuid::Uuid>().map_err(|e| CommandError::NotFound {
        message: format!("invalid entry id '{s}': {e}"),
    })
}

fn apply_config_patch(config: &mut UIConfig, patch: UIConfigPatch) {
    if let Some(v) = patch.speaker { config.speaker = v; }
    if let Some(v) = patch.sample_rate { config.sample_rate = v; }
    if let Some(v) = patch.speech_rate { config.speech_rate = v; }
    if let Some(v) = patch.hotkey_read_now { config.hotkey_read_now = v; }
    if let Some(v) = patch.hotkey_read_later { config.hotkey_read_later = v; }
    if let Some(v) = patch.notify_on_ready { config.notify_on_ready = v; }
    if let Some(v) = patch.notify_on_error { config.notify_on_error = v; }
    if let Some(v) = patch.text_format { config.text_format = v; }
    if let Some(v) = patch.history_days { config.history_days = v; }
    if let Some(v) = patch.audio_max_files { config.audio_max_files = v; }
    if let Some(v) = patch.audio_regenerated_hours { config.audio_regenerated_hours = v; }
    if let Some(v) = patch.max_cache_size_mb { config.max_cache_size_mb = v; }
    if let Some(v) = patch.auto_cleanup_days { config.auto_cleanup_days = v; }
    if let Some(v) = patch.code_block_mode { config.code_block_mode = v; }
    if let Some(v) = patch.read_operators { config.read_operators = v; }
    if let Some(v) = patch.theme { config.theme = v; }
    if let Some(v) = patch.player_hotkeys { config.player_hotkeys = v; }
    if let Some(v) = patch.window_geometry { config.window_geometry = v; }
    if let Some(v) = patch.preview_dialog_enabled { config.preview_dialog_enabled = v; }
    if let Some(v) = patch.preview_threshold { config.preview_threshold = v; }
}
