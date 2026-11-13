//! Tauri command handlers (IPC Layer 1: Frontend → Backend).
//!
//! All commands use `Result<T, CommandError>` so that errors are serialized as
//! typed JSON objects (`{ "type": "...", "code": "...", "params": [...],
//! "message": "..." }`) which the frontend localizes by `code` and falls back
//! to `message` for unknown codes.

use base64::Engine as _;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tauri_plugin_mpv::MpvExt;
use tokio::task::AbortHandle;
use tracing::{debug, info, warn};

use crate::pipeline::tracked_text::CharMapping;
use crate::pipeline::{Span, TTSPipeline};
use crate::state::AppState;
use crate::storage::schema::{
    DisplayMode, EntryId, EntrySource, EntryStatus, GenerationParams, ModelParams, TextEntry,
    TextFormat, UIConfig, UIConfigPatch, WordTimestamp,
};
use crate::storage::service::{StorageError, StorageService};
use crate::storage::title::TITLE_MAX_CHARS;
use crate::tts::engine::EngineKind;
use crate::tts::model_lifecycle::{self, ModelAssetStatus, ModelState, ModelStatus};
use crate::tts::piper::catalog;
use crate::tts::piper::download::download_voice;
use crate::tts::{
    AvailableEngines, CharMappingEntry, ModelInfo, SynthesizeOutput, TtsEngine, TtsError,
    availability,
};
use tts_contract::{Stress, TextProfile};

// ── Error type ─────────────────────────────────────────────────────────────────

/// Typed error returned by all Tauri commands.
///
/// The wire format carries no user-facing prose: `code` is a stable
/// machine-readable site id the frontend translates via its localization
/// catalogs, `params` holds positional interpolation values in the order the
/// localized sentence needs them, and `message` (optional) is raw diagnostic
/// detail used as a fallback for codes unknown to the frontend.
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandError {
    #[error("not found: {code}")]
    NotFound {
        code: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        params: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },

    #[error("storage error: {code}")]
    StorageError {
        code: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        params: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },

    #[error("synthesis error: {code}")]
    SynthesisError {
        code: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        params: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },

    #[error("playback error: {code}")]
    PlaybackError {
        code: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        params: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },

    #[error("config error: {code}")]
    ConfigError {
        code: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        params: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },

    #[error("internal error: {code}")]
    Internal {
        code: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        params: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
}

impl CommandError {
    fn not_found(code: impl Into<String>, params: Vec<String>) -> Self {
        Self::NotFound {
            code: code.into(),
            params,
            message: None,
        }
    }

    fn storage(code: impl Into<String>, params: Vec<String>) -> Self {
        Self::StorageError {
            code: code.into(),
            params,
            message: None,
        }
    }

    fn synthesis(code: impl Into<String>, params: Vec<String>) -> Self {
        Self::SynthesisError {
            code: code.into(),
            params,
            message: None,
        }
    }

    fn playback(code: impl Into<String>, params: Vec<String>) -> Self {
        Self::PlaybackError {
            code: code.into(),
            params,
            message: None,
        }
    }

    fn config(code: impl Into<String>, params: Vec<String>) -> Self {
        Self::ConfigError {
            code: code.into(),
            params,
            message: None,
        }
    }

    fn internal(code: impl Into<String>, params: Vec<String>) -> Self {
        Self::Internal {
            code: code.into(),
            params,
            message: None,
        }
    }

    /// Attach raw diagnostic detail (engine/HTTP/storage strings). Used as a
    /// display fallback when the frontend does not know the `code`.
    fn with_message(mut self, message: impl Into<String>) -> Self {
        let slot = match &mut self {
            Self::NotFound { message, .. }
            | Self::StorageError { message, .. }
            | Self::SynthesisError { message, .. }
            | Self::PlaybackError { message, .. }
            | Self::ConfigError { message, .. }
            | Self::Internal { message, .. } => message,
        };
        *slot = Some(message.into());
        self
    }
}

impl From<StorageError> for CommandError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::NotFound(id) => {
                CommandError::not_found("entry.not_found", vec![id.to_string()])
            }
            other => {
                CommandError::storage("storage.failure", vec![]).with_message(other.to_string())
            }
        }
    }
}

impl From<TtsError> for CommandError {
    fn from(e: TtsError) -> Self {
        CommandError::synthesis("synthesis.failed", vec![]).with_message(e.to_string())
    }
}

type CmdResult<T> = Result<T, CommandError>;

/// Wire string of an [`EntryStatus`] (matches its lowercase serde names) so
/// status values can travel in `CommandError::params`.
fn entry_status_str(status: EntryStatus) -> &'static str {
    match status {
        EntryStatus::Pending => "pending",
        EntryStatus::Processing => "processing",
        EntryStatus::Ready => "ready",
        EntryStatus::Playing => "playing",
        EntryStatus::Error => "error",
        EntryStatus::Cancelled => "cancelled",
    }
}

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

mod synthesis;

pub(crate) use synthesis::{SynthesisDeps, spawn_synthesis};
use synthesis::{discard_transient_files, run_pipeline_normalization};

// ── Commands ───────────────────────────────────────────────────────────────────

/// Shared implementation for the two "add text to queue" commands below.
/// Rejects blank input, persists the entry, emits `entry_updated`, and
/// spawns background synthesis.
#[allow(clippy::too_many_arguments)]
fn ingest_text<R: Runtime>(
    app: AppHandle<R>,
    state: &AppState,
    text: String,
    play_when_ready: bool,
    format: Option<TextFormat>,
    html_source: Option<String>,
    source: Option<EntrySource>,
) -> CmdResult<String> {
    if text.trim().is_empty() {
        return Err(CommandError::internal("input.empty", vec![]));
    }

    let entry = state
        .storage
        .add_entry_with_source(text, format, html_source, source)
        .map_err(CommandError::from)?;
    let entry_id = entry.id;

    emit_entry_updated(&app, &entry);

    spawn_synthesis(
        SynthesisDeps::from_state(&app, state),
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
///
/// For HTML-ingested entries the frontend passes `format: "html"`, the
/// extracted plain text as `text`, and the sanitized markup as
/// `html_source` (rendering only — synthesis always normalizes `text`).
/// `source` records where the text came from (clipboard / file / URL) for
/// the generation-params dialog.
#[tauri::command]
pub async fn add_text_entry<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    text: String,
    play_when_ready: bool,
    format: Option<TextFormat>,
    html_source: Option<String>,
    source: Option<EntrySource>,
) -> CmdResult<String> {
    ingest_text(
        app,
        &state,
        text,
        play_when_ready,
        format,
        html_source,
        source,
    )
}

/// Read text from the system clipboard and add a new entry to the queue.
/// Used by the tray menu, where no webview context is available.
/// Frontend code should prefer `add_text_entry` (see above).
#[tauri::command]
pub async fn add_clipboard_entry<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    play_when_ready: bool,
) -> CmdResult<String> {
    // Read clipboard on a blocking thread (required on Linux to avoid deadlock).
    let text = tokio::task::spawn_blocking(|| {
        let mut board = arboard::Clipboard::new().map_err(|e| {
            CommandError::internal("clipboard.unavailable", vec![]).with_message(e.to_string())
        })?;
        board
            .get_text()
            .map_err(|_| CommandError::internal("clipboard.empty", vec![]))
    })
    .await
    .map_err(|e| {
        CommandError::internal("clipboard.task_panicked", vec![]).with_message(e.to_string())
    })??;

    ingest_text(
        app,
        &state,
        text,
        play_when_ready,
        None,
        None,
        Some(EntrySource::Clipboard),
    )
}

/// Pure normalization step behind [`preview_normalize`]: runs the pipeline on
/// a blocking thread and returns the normalized text together with its char
/// mapping. Unlike [`synthesis::run_normalization`], empty input is not an error — the
/// preview dialog must show even an empty normalization result.
///
/// Takes no storage/TTS handles, so a preview can never create history
/// entries or kick off synthesis.
async fn preview_normalization(
    pipeline: Arc<Mutex<TTSPipeline>>,
    text: String,
    profile: &'static TextProfile,
) -> CmdResult<(String, CharMapping)> {
    let (normalized, mapping, _spans, _code_block_mode) =
        run_pipeline_normalization(pipeline, text, profile)
            .await
            .map_err(|e| {
                CommandError::internal("pipeline.panicked", vec![]).with_message(e.to_string())
            })?;
    Ok((normalized, mapping))
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
    let (normalized, _char_mapping) =
        preview_normalization(Arc::clone(&state.pipeline), text, state.tts.text_profile()).await?;
    Ok(PreviewNormalizeResult { normalized })
}

#[derive(Debug, Serialize)]
pub struct PreviewNormalizeResult {
    pub normalized: String,
}

/// Response of [`preview_pronunciation`]: the text that was actually spoken
/// (the normalized reading, or the stress-adjusted spoken form) plus the WAV
/// bytes as base64 and the rendered duration.
#[derive(Debug, Serialize)]
pub struct PreviewPronunciationResult {
    pub spoken_text: String,
    pub wav_base64: String,
    pub duration_sec: f64,
}

/// Longest preview input, in Unicode code points (`+` is ASCII, so the
/// stress mark cannot inflate the count). The editor's controls only ever
/// send one word or one spoken form; the command is reachable directly, and
/// a pasted paragraph would keep the engine busy for seconds.
const PREVIEW_MAX_CHARS: usize = 200;

/// Cap on the WAV that `preview_play` accepts, after decoding. A 200-code-point
/// preview renders to about 1-2 MB; the command is reachable directly, so a
/// modified client must not be able to hand mpv an arbitrarily large file.
const PREVIEW_MAX_WAV_BYTES: usize = 8 * 1024 * 1024;

/// How old a file in the preview directory must be to count as a leftover
/// of a crashed or killed run. The command deletes its WAV on every exit
/// path (see [`PreviewWav`]), so nothing legitimate lives this long.
pub(crate) const PREVIEW_STALE_AFTER: std::time::Duration = std::time::Duration::from_secs(10 * 60);

/// Owns one preview WAV and deletes it when it goes out of scope, i.e. on
/// every exit path of [`preview_pronunciation`]: a successful read, an engine
/// error, a read failure, an aborted task. `tokio::fs` is unusable from
/// `Drop`, so the removal is a best-effort blocking call on a small
/// cache-disk file.
struct PreviewWav(PathBuf);

impl Drop for PreviewWav {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.0) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => warn!("failed to remove preview wav {}: {e}", self.0.display()),
        }
    }
}

/// Map an engine failure onto the wire error, keeping the engine's site id
/// (`bundle_not_installed`, `voice_not_installed`, …) so the form can say
/// "model not installed" instead of a generic sentence.
fn preview_synthesis_error(e: TtsError) -> CommandError {
    match e {
        TtsError::Ttsd { code, message } => {
            CommandError::synthesis(&code, vec![]).with_message(message)
        }
        other => CommandError::synthesis("preview.synthesis_failed", vec![])
            .with_message(other.to_string()),
    }
}

/// Delete every regular file in `dir` modified at or before `stale_before`,
/// returning how many were removed. The startup backstop for [`PreviewWav`]:
/// a crash or a killed process bypasses the guard, and the leftovers must not
/// accumulate in the app cache. A missing directory is not an error (no
/// preview has ever run).
pub(crate) fn sweep_stale_previews(
    dir: &Path,
    stale_before: std::time::SystemTime,
) -> std::io::Result<usize> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    let mut removed = 0;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file() {
            continue;
        }
        let modified = match entry.metadata().and_then(|m| m.modified()) {
            Ok(modified) => modified,
            Err(e) => {
                warn!(
                    "preview sweep: cannot read mtime of {}: {e}",
                    path.display()
                );
                continue;
            }
        };
        if modified > stale_before {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => removed += 1,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => warn!("preview sweep: failed to remove {}: {e}", path.display()),
        }
    }
    Ok(removed)
}

/// Speak one dictionary word without touching the entry lifecycle (issue
/// #349): the editor's play controls call this to hear «Слово» as the
/// pipeline reads it today (`mode = "normalize"`) or «Как произносить» as the
/// typed spoken form will sound (`mode = "spoken"`).
///
/// The audio is synthesized by the active engine only, into a unique file
/// under the app cache directory, read into memory and deleted before the
/// response leaves the command — no entry, timestamp, queue state or
/// synthesis event is created (this deliberately bypasses
/// [`synthesis::synthesize_audio`]).
#[tauri::command]
pub async fn preview_pronunciation(
    state: State<'_, AppState>,
    text: String,
    mode: String,
) -> CmdResult<PreviewPronunciationResult> {
    let char_count = text.chars().count();
    if char_count > PREVIEW_MAX_CHARS {
        return Err(CommandError::internal(
            "preview.text_too_long",
            vec![char_count.to_string(), PREVIEW_MAX_CHARS.to_string()],
        ));
    }

    // One preview at a time: a double click racing the disabled state (or a
    // second editor) must not synthesize over the first one. `try_lock`, not
    // `lock`: the queue holds this lock around its engine call (see
    // `synthesis::synthesize_audio`), and a preview must never wait behind a
    // narration that can run for minutes — it is refused instead.
    //
    // Lock order (mirrored there): `preview_lock` first, `synthesize_entered`
    // second. The preview only *checks* the marker while holding the lock and
    // never waits on it, so the two cannot deadlock.
    let _preview = state
        .preview_lock
        .try_lock()
        .map_err(|_| CommandError::internal("preview.engine_busy", vec![]))?;

    // A queue entry inside `tts.synthesize` owns the engine. Every engine
    // tolerates a second caller structurally, but the preview would wait
    // behind a narration that can run for minutes, and TeraTTS shares one
    // cancellation flag between runs — so refuse instead of queueing.
    if !state.synthesize_entered.lock().is_empty() {
        return Err(CommandError::internal("preview.engine_busy", vec![]));
    }

    let spoken_text = match mode.as_str() {
        "normalize" => {
            // Same pass as `preview_normalize`: the saved dictionary is
            // applied and the entry being added does not exist yet (in edit
            // mode the saved entry is part of the dictionary).
            let (normalized, _mapping) =
                preview_normalization(Arc::clone(&state.pipeline), text, state.tts.text_profile())
                    .await?;
            normalized.trim().to_string()
        }
        "spoken" => {
            // `spoken_form` validates the mark and returns the text without
            // it; the mark reaches the engine only when its profile renders
            // stress — the same rule the pipeline applies to an entry's `to`.
            let stripped = tts_normalize::dictionary::spoken_form(&text)
                .map_err(|e| {
                    CommandError::internal("preview.invalid_stress", vec![])
                        .with_message(e.to_string())
                })?
                .trim()
                .to_string();
            if state.tts.text_profile().stress == Stress::None {
                stripped
            } else {
                text.trim().to_string()
            }
        }
        other => {
            return Err(CommandError::internal(
                "preview.invalid_mode",
                vec![other.to_string()],
            ));
        }
    };

    if spoken_text.is_empty() {
        return Err(CommandError::internal("preview.empty_text", vec![]));
    }

    let config = state.storage.load_config().unwrap_or_default();
    let voice = synthesis::voice_for_engine(state.tts.kind(), &config);
    let preview_dir = state.preview_dir.join("preview");
    tokio::fs::create_dir_all(&preview_dir).await.map_err(|e| {
        CommandError::internal("preview.dir_create_failed", vec![]).with_message(e.to_string())
    })?;
    let out_path = preview_dir.join(format!("{}.wav", uuid::Uuid::new_v4()));
    // Deletes the WAV on every exit path below: an engine error, a read
    // failure, an early return.
    let _wav = PreviewWav(out_path.clone());

    // Final check right before the engine call. Holding `preview_lock`
    // already makes the first check final (the queue sets the marker only
    // once it owns the lock); this one pins the invariant at the call site,
    // with the normalization pass being the long await in between.
    if !state.synthesize_entered.lock().is_empty() {
        return Err(CommandError::internal("preview.engine_busy", vec![]));
    }

    let synthesis = state
        .tts
        .synthesize(
            spoken_text.clone(),
            voice,
            config.sample_rate,
            out_path.to_string_lossy().into_owned(),
            None,
        )
        .await;

    let output = synthesis.map_err(preview_synthesis_error)?;
    // Keep the real cause: a permissions or IO failure must not read as
    // "the file disappeared".
    let bytes = tokio::fs::read(&out_path).await.map_err(|e| {
        CommandError::internal("preview.audio_read_failed", vec![]).with_message(format!(
            "failed to read preview wav {}: {e}",
            out_path.display()
        ))
    })?;

    Ok(PreviewPronunciationResult {
        spoken_text,
        wav_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        duration_sec: output.duration_sec,
    })
}

/// Play a synthesized preview WAV through the dedicated preview player
/// (issue #349).
///
/// The dialog keeps the WAV bytes in its in-memory cache and hands them back
/// here, so the synthesis contract and the cache stay untouched; only the
/// playback channel moves off the webview's media stack. The player writes a
/// temporary WAV under the app cache, starts mpv, and deletes the file on
/// [`preview_stop`], editor close or playback end.
#[tauri::command]
pub async fn preview_play(state: State<'_, AppState>, wav_base64: String) -> CmdResult<()> {
    // Reject obviously oversized payloads before allocating the decoded
    // buffer: base64 is 4/3 of the binary length, so this lower bound is
    // exact enough to avoid a multi-megabyte decode of garbage.
    if wav_base64.len() / 4 * 3 > PREVIEW_MAX_WAV_BYTES {
        return Err(CommandError::internal("preview.audio_too_large", vec![]));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(wav_base64.as_bytes())
        .map_err(|e| {
            CommandError::internal("preview.invalid_audio", vec![]).with_message(e.to_string())
        })?;
    if bytes.len() > PREVIEW_MAX_WAV_BYTES {
        return Err(CommandError::internal("preview.audio_too_large", vec![]));
    }

    state.preview_audio.play(&bytes).map_err(|e| {
        CommandError::internal("preview.play_failed", vec![]).with_message(e.to_string())
    })
}

/// Stop and release the preview player. Idempotent: the editor calls it on
/// every close, and the playback-end timer may race a manual stop.
#[tauri::command]
pub async fn preview_stop(state: State<'_, AppState>) -> CmdResult<()> {
    state.preview_audio.stop().map_err(|e| {
        CommandError::internal("preview.stop_failed", vec![]).with_message(e.to_string())
    })
}

/// Return all entries sorted by created_at descending.
#[tauri::command]
pub async fn get_entries(state: State<'_, AppState>) -> CmdResult<Vec<TextEntry>> {
    Ok(state.storage.get_all_entries())
}

/// Shared implementation for [`get_entry`]: parse the wire id and look it up.
/// An unknown but well-formed id yields `Ok(None)` (serialized as `null`),
/// a malformed one a `not_found` error (see [`parse_entry_id`]).
fn lookup_entry(storage: &StorageService, id: &str) -> CmdResult<Option<TextEntry>> {
    let uuid = parse_entry_id(id)?;
    Ok(storage.get_entry(&uuid))
}

/// Shared implementation for commands that act on an existing entry: parse
/// the wire id and return the entry, or a `not_found` error naming the id.
/// Unlike [`lookup_entry`], a well-formed but unknown id is an error here.
fn require_entry(storage: &StorageService, id: &str) -> CmdResult<TextEntry> {
    let uuid = parse_entry_id(id)?;
    storage
        .get_entry(&uuid)
        .ok_or_else(|| CommandError::not_found("entry.not_found", vec![id.to_string()]))
}

/// Return a single entry by ID, or null if not found.
#[tauri::command]
pub async fn get_entry(state: State<'_, AppState>, id: String) -> CmdResult<Option<TextEntry>> {
    lookup_entry(&state.storage, &id)
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
pub async fn delete_audio<R: Runtime>(
    app: AppHandle<R>,
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

/// Apply a metadata-only mutation (display mode, title, note) to one entry
/// and broadcast the result (issue #353).
///
/// The write is applied in place under the storage lock instead of persisting
/// a read-decide-write clone of the whole entry: a synthesis completion that
/// lands between a stale read and the write would otherwise be rolled back
/// (status, audio path, timestamps). Unknown ids are rejected with the typed
/// `not_found` error and emit nothing.
fn patch_entry<R: Runtime>(
    app: &AppHandle<R>,
    storage: &StorageService,
    id: &str,
    mutate: impl FnOnce(&mut TextEntry),
) -> CmdResult<()> {
    let uuid = parse_entry_id(id)?;
    if !storage.update_entry_if(&uuid, |_| true, mutate) {
        return Err(CommandError::not_found(
            "entry.not_found",
            vec![id.to_string()],
        ));
    }
    if let Some(entry) = storage.get_entry(&uuid) {
        emit_entry_updated(app, &entry);
    }
    Ok(())
}

/// Persist the viewer display mode of an entry and notify the UI.
///
/// Display-only: the immutable source `format`, `normalized_text`, audio, and
/// timestamps are untouched, so no `Processing` guard is needed. The write goes
/// through [`patch_entry`], which applies it in place under the storage lock.
#[tauri::command]
pub async fn set_entry_display_mode<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
    display_mode: DisplayMode,
) -> CmdResult<()> {
    patch_entry(&app, &state.storage, &id, |entry| {
        entry.display_mode = Some(display_mode);
    })
}

/// Maximum stored note length in Unicode scalar values.
const NOTE_MAX_CHARS: usize = 2000;

/// Normalize a user-supplied title or note value: trim, treat empty as
/// absent, and cap at `max_chars` (re-trimming the end, since the cap can
/// cut mid-whitespace).
fn normalize_metadata_value(value: Option<String>, max_chars: usize) -> Option<String> {
    let value = value?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let capped: String = trimmed.chars().take(max_chars).collect();
    let capped = capped.trim_end();
    (!capped.is_empty()).then(|| capped.to_string())
}

/// Persist the editable title of an entry and notify the UI (issue #353).
///
/// A `None` (or empty / whitespace-only) title clears the stored value, so the
/// UI falls back to the title derived from `original_text`. The cap is
/// [`TITLE_MAX_CHARS`] — the same constant that seeds new entries — and the
/// write goes through [`patch_entry`].
#[tauri::command]
pub async fn set_entry_title<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
    title: Option<String>,
) -> CmdResult<()> {
    let title = normalize_metadata_value(title, TITLE_MAX_CHARS);
    patch_entry(&app, &state.storage, &id, |entry| entry.title = title)
}

/// Persist the editable note of an entry and notify the UI (issue #353).
///
/// Notes are plain text limited to 2000 characters; they are never part of
/// the TTS input. Same atomic-write and clear-on-empty rules as
/// [`set_entry_title`].
#[tauri::command]
pub async fn set_entry_note<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
    note: Option<String>,
) -> CmdResult<()> {
    let note = normalize_metadata_value(note, NOTE_MAX_CHARS);
    patch_entry(&app, &state.storage, &id, |entry| entry.note = note)
}

/// Regenerate audio for an existing entry: re-run the synthesis pipeline
/// against the stored `original_text` with the current settings. Useful when
/// the user has changed `speaker`, `speech_rate`, or other normalization
/// settings and wants the cached audio to reflect them. With `play_when_ready`,
/// the fresh audio autoplays on success (same rule as the initial synthesis).
///
/// Non-destructive: the entry's current audio and timestamps stay in place
/// until the fresh synthesis succeeds (the Opus transcode replaces the file
/// with a single rename). A run that fails or is cancelled therefore leaves the
/// previous recording playable — see [`synthesis::apply_error_if_current`] and
/// [`cancel_entry`], which restore `ready` instead of parking the entry in
/// `error` / `cancelled` for exactly that reason.
///
/// Rejects the call if the entry is currently being synthesized — re-entering
/// `spawn_synthesis` for the same id would race with the in-flight task.
#[tauri::command]
pub async fn regenerate_entry<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
    play_when_ready: bool,
) -> CmdResult<()> {
    let entry = require_entry(&state.storage, &id)?;
    let uuid = entry.id;

    if entry.status == EntryStatus::Processing {
        return Err(CommandError::synthesis(
            "synthesis.in_progress",
            vec![id.to_string()],
        ));
    }

    // If this entry is currently playing, stop playback so the player does not
    // hold the file the fresh transcode is about to rename over.
    if state.player.current_entry_id().as_deref() == Some(&id) {
        let _ = state.player.stop();
    }

    let mut entry = state
        .storage
        .get_entry(&uuid)
        .ok_or_else(|| CommandError::not_found("entry.not_found", vec![id.to_string()]))?;
    // `delete_audio` used to move the entry to `pending` as a side effect; the
    // lifecycle still starts there, only without touching the stored artifacts.
    entry.status = EntryStatus::Pending;
    entry.was_regenerated = true;
    entry.error_message = None;
    state
        .storage
        .update_entry(entry.clone())
        .map_err(CommandError::from)?;
    emit_entry_updated(&app, &entry);

    spawn_synthesis(
        SynthesisDeps::from_state(&app, &state),
        uuid,
        play_when_ready,
    );

    Ok(())
}

/// Core of [`cancel_synthesis`] without Tauri handles: abort the entry's
/// synthesis task (if registered) and flip the entry out of the in-flight
/// states. Returns the updated entry plus whether the task had entered the TTS
/// stage — the caller kills ttsd only in that case.
///
/// Only a `processing` or `pending` entry may be cancelled: the spec
/// sanctions just the `processing → cancelled` transition, and silently
/// regressing a `ready` / `error` entry to `cancelled` would orphan its
/// audio from the state machine (playback requires `ready`). `pending`
/// is allowed: a queued/idle entry can be cancelled, and a just-added
/// entry briefly sits in `pending` with its synthesis task already
/// registered — cancelling must still abort it. `ready`, `playing`,
/// `error` and `cancelled` fail with `synthesis_error` and change nothing.
///
/// An entry that still owns a recording returns to `ready` instead of
/// `cancelled`: cancelling a regeneration keeps the audio the run was going to
/// replace, and only `ready` makes it playable again.
fn cancel_entry(
    storage: &StorageService,
    synthesis_tasks: &Mutex<HashMap<EntryId, AbortHandle>>,
    synthesize_entered: &Mutex<HashSet<EntryId>>,
    id: &str,
) -> CmdResult<(TextEntry, bool)> {
    let uuid = parse_entry_id(id)?;

    // Snapshot the live status to reject terminal states up front. The
    // authoritative check-then-apply runs atomically inside `update_entry_if`
    // below (issue #179); this snapshot only drives the error path so a
    // `ready`/`playing`/`error` entry is rejected without touching the
    // synthesis registries.
    let live_status = storage
        .get_entry(&uuid)
        .map(|e| e.status)
        .ok_or_else(|| CommandError::not_found("entry.not_found", vec![id.to_string()]))?;
    if matches!(
        live_status,
        EntryStatus::Ready | EntryStatus::Error | EntryStatus::Playing | EntryStatus::Cancelled
    ) {
        return Err(CommandError::synthesis(
            "entry.cannot_cancel",
            vec![id.to_string(), entry_status_str(live_status).to_string()],
        ));
    }

    // Atomic transition: leave the in-flight states only if the entry is still
    // `processing`/`pending` at apply time. A concurrent synthesis completion
    // that already moved it out of these states wins, and the stale
    // `cancelled` clone is NOT persisted. The target status follows the same
    // rule as a failed regeneration: a stored recording means the entry is
    // still playable, so it goes back to `ready` rather than the terminal
    // `cancelled`.
    let applied = storage.update_entry_if(
        &uuid,
        |e| matches!(e.status, EntryStatus::Processing | EntryStatus::Pending),
        |e| {
            e.status = if e.audio_path.is_some() {
                EntryStatus::Ready
            } else {
                EntryStatus::Cancelled
            };
        },
    );

    if !applied {
        // Lost the race to a simultaneous completion — the entry is now
        // `ready`/`error`. Nothing to abort; report the live state.
        let entry = storage
            .get_entry(&uuid)
            .ok_or_else(|| CommandError::not_found("entry.not_found", vec![id.to_string()]))?;
        return Ok((entry, false));
    }

    // Cancellation actually applied: abort the entry's synthesis task (a
    // no-op if already finished) and clear the entered-TTS marker.
    if let Some(handle) = synthesis_tasks.lock().remove(&uuid) {
        handle.abort();
    }
    let entered_tts = synthesize_entered.lock().remove(&uuid);
    // The abort kills the async task, not a blocking encoder that already
    // started: drop its staging files, but only while no newer run owns them.
    discard_transient_files(storage, &uuid);
    let entry = storage
        .get_entry(&uuid)
        .ok_or_else(|| CommandError::not_found("entry.not_found", vec![id.to_string()]))?;
    Ok((entry, entered_tts))
}

/// Cancel an in-progress or queued synthesis job: abort the entry's
/// synthesis task and move the entry out of the in-flight states: `cancelled`
/// for an entry without a stored recording, back to `ready` for one that still
/// owns its previous audio (cancelling a regeneration must not hide a playable
/// recording — see [`cancel_entry`]). If the task had
/// already entered the TTS stage, the active engine's in-flight work is
/// terminated too — the ttsd subprocess for Silero, the chunk-loop cancel
/// flag for Piper (the supervisor transparently respawns ttsd on the next
/// request). A late completion belonging to the cancelled entry is
/// discarded by the stale-completion guard in `apply_ready_if_current` /
/// `apply_error_if_current`.
#[tauri::command]
pub async fn cancel_synthesis<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<()> {
    let (entry, entered_tts) = cancel_entry(
        &state.storage,
        &state.synthesis_tasks,
        &state.synthesize_entered,
        &id,
    )?;

    info!(
        "cancel_synthesis: id={id}, status={}, entered_tts={entered_tts}",
        entry_status_str(entry.status)
    );

    if entered_tts {
        state.engine_switcher.kill_current_ttsd().await;
    }

    emit_entry_updated(&app, &entry);
    Ok(())
}

/// Start playback of a ready entry.
#[tauri::command]
pub async fn play_entry(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    let entry = require_entry(&state.storage, &id)?;
    let uuid = entry.id;

    if entry.status != EntryStatus::Ready {
        return Err(CommandError::playback(
            "entry.not_ready",
            vec![id.to_string(), entry_status_str(entry.status).to_string()],
        ));
    }

    let path = state
        .storage
        .get_audio_path(&uuid)
        .ok_or_else(|| CommandError::playback("audio.missing", vec![id.to_string()]))?;

    state.player.load(&path, id.clone()).map_err(|e| {
        CommandError::playback("playback.load_failed", vec![]).with_message(e.to_string())
    })?;

    state.player.play().map_err(|e| {
        CommandError::playback("playback.play_failed", vec![]).with_message(e.to_string())
    })?;

    Ok(())
}

/// Pause the currently playing entry.
#[tauri::command]
pub async fn pause_playback(state: State<'_, AppState>) -> CmdResult<()> {
    state.player.pause().map_err(|e| {
        CommandError::playback("playback.pause_failed", vec![]).with_message(e.to_string())
    })
}

/// Resume playback from the paused position.
#[tauri::command]
pub async fn resume_playback(state: State<'_, AppState>) -> CmdResult<()> {
    state.player.resume().map_err(|e| {
        CommandError::playback("playback.resume_failed", vec![]).with_message(e.to_string())
    })
}

/// Stop playback entirely.
#[tauri::command]
pub async fn stop_playback(state: State<'_, AppState>) -> CmdResult<()> {
    state.player.stop().map_err(|e| {
        CommandError::playback("playback.stop_failed", vec![]).with_message(e.to_string())
    })
}

/// Whether the updater can serve this install (#226). Windows installs always
/// (NSIS flow); on Linux only an AppImage exposes the `APPIMAGE` env var
/// tauri-plugin-updater needs to replace the running image — .deb/nix installs
/// get no update UI instead of a check that would always error. The env lookup
/// is injectable for tests.
fn updater_supported_with(appimage_env: Option<OsString>) -> bool {
    #[cfg(target_os = "windows")]
    {
        let _ = appimage_env;
        true
    }
    #[cfg(target_os = "linux")]
    {
        appimage_env.is_some()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = appimage_env;
        false
    }
}

/// Report to the frontend whether the update check/UI should be offered.
#[tauri::command]
pub fn updater_supported() -> bool {
    updater_supported_with(std::env::var_os("APPIMAGE"))
}

/// Destroy the mpv subprocess before the updater launches the installer
/// (#211). The updater-launched NSIS installer force-kills this process,
/// so `RunEvent::Exit` (which normally destroys mpv) never fires and the
/// orphaned mpv.exe keeps `$INSTDIR\mpv\mpv.exe` locked, failing the
/// install. Called by the frontend right before `downloadAndInstall()`.
/// Mirrors the `RunEvent::Exit` cleanup in `lib.rs`: mark first so
/// in-flight player commands short-circuit, then destroy.
#[tauri::command]
pub async fn shutdown_player_for_update<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> CmdResult<()> {
    state.player.mark_destroyed();
    // Best-effort: a missing mpv instance (already destroyed when the main
    // window closed) must not abort the update.
    if let Err(e) = app.mpv().destroy(crate::player::WINDOW_LABEL) {
        warn!("mpv destroy before update failed (continuing): {e}");
    }
    Ok(())
}

/// Seek to an absolute position in the current audio.
#[tauri::command]
pub async fn seek_to(state: State<'_, AppState>, position_sec: f64) -> CmdResult<()> {
    state.player.seek(position_sec).map_err(|e| {
        CommandError::playback("playback.seek_failed", vec![]).with_message(e.to_string())
    })
}

/// Set playback speed (0.5–3.0). Persisted to UIConfig.speech_rate.
#[tauri::command]
pub async fn set_speed(state: State<'_, AppState>, speed: f32) -> CmdResult<()> {
    if !(0.5..=3.0).contains(&speed) {
        return Err(CommandError::config(
            "speed.out_of_range",
            vec![speed.to_string()],
        ));
    }

    state.player.set_speed(speed).map_err(|e| {
        CommandError::playback("playback.set_speed_failed", vec![]).with_message(e.to_string())
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
        return Err(CommandError::config(
            "volume.out_of_range",
            vec![volume.to_string()],
        ));
    }

    state.player.set_volume(volume).map_err(|e| {
        CommandError::playback("playback.set_volume_failed", vec![]).with_message(e.to_string())
    })
}

/// Return the current application configuration.
#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> CmdResult<UIConfig> {
    state.storage.load_config().map_err(CommandError::from)
}

/// Path of the small, non-config model-state file (skip-version choices).
fn model_state_path() -> CmdResult<PathBuf> {
    crate::paths::config_root()
        .map(|root| root.join("model_state.json"))
        .ok_or_else(|| CommandError::internal("models.config_root_missing", vec![]))
}

fn bundle_dir(state: &AppState, asset: &str) -> Option<PathBuf> {
    match asset {
        model_lifecycle::SILERO_NATIVE => Some(state.silero_native_bundle_dir.clone()),
        model_lifecycle::TERATTS => Some(state.teratts_bundle_dir.clone()),
        _ => None,
    }
}

fn pinned_manifest_url(asset: &str) -> Option<String> {
    match asset {
        model_lifecycle::SILERO_NATIVE => Some(format!(
            "{}/manifest.json",
            crate::tts::silero_native::download::BUNDLE_RELEASE_BASE_URL
        )),
        model_lifecycle::TERATTS => Some(format!(
            "{}/manifest.json",
            crate::tts::teratts::download::BUNDLE_RELEASE_BASE_URL
        )),
        _ => None,
    }
}

fn default_bundle_version(asset: &str) -> &'static str {
    match asset {
        model_lifecycle::SILERO_NATIVE => "v5_ru",
        model_lifecycle::TERATTS => "1",
        _ => "unknown",
    }
}

fn bundle_row(asset: &str, bundle_dir: &Path, active_kind: EngineKind) -> ModelAssetStatus {
    let info = model_lifecycle::inspect_bundle(asset, bundle_dir);
    ModelAssetStatus {
        id: asset.to_string(),
        kind: "bundle".to_string(),
        installed: info.installed,
        active: active_kind.as_str() == asset,
        installed_version: info.version.filter(|v| !v.is_empty()),
        model_id: info.model_id,
        upstream_revision: info.upstream_revision,
        bundled_voices: info.bundled_voices,
        installed_bytes: info.bytes,
        latest_version: None,
        latest_bytes: None,
        update_available: false,
        skipped: false,
        // A corrupt or partially downloaded directory still occupies disk;
        // let the user remove it even though it is not a usable bundle.
        deletable: info.bytes > 0 && active_kind.as_str() != asset,
        error: info.error,
    }
}

/// Return the unified model inventory: both versioned bundles and installed
/// Piper voices, with sizes and update state. `force` bypasses the 24-hour
/// channel cache and is used by the explicit "check for updates" action.
#[tauri::command]
pub async fn get_model_status(state: State<'_, AppState>, force: bool) -> CmdResult<ModelStatus> {
    let snapshot = model_lifecycle::channels(force).await;
    let model_state = ModelState::load(&model_state_path()?);
    let active_kind = state.engine_switcher.kind();
    let active_voice = state.engine_switcher.active_piper_voice().await;

    let mut rows = vec![
        bundle_row(
            model_lifecycle::SILERO_NATIVE,
            &state.silero_native_bundle_dir,
            active_kind,
        ),
        bundle_row(
            model_lifecycle::TERATTS,
            &state.teratts_bundle_dir,
            active_kind,
        ),
    ];
    for voice in catalog::VOICES {
        let dir = state.piper_voices_dir.join(voice.id);
        let installed = dir.join(voice.model_filename()).is_file()
            && dir.join(voice.model_config_filename()).is_file();
        let installed_bytes = model_lifecycle::dir_size(&dir);
        let active = active_kind == EngineKind::Piper && active_voice.as_deref() == Some(voice.id);
        rows.push(ModelAssetStatus {
            id: format!("piper:{}", voice.id),
            kind: "voice".to_string(),
            installed,
            active,
            installed_version: None,
            model_id: Some(voice.id.to_string()),
            upstream_revision: None,
            bundled_voices: Vec::new(),
            installed_bytes,
            latest_version: None,
            latest_bytes: None,
            update_available: false,
            skipped: false,
            deletable: installed_bytes > 0 && !active,
            error: None,
        });
    }
    Ok(model_lifecycle::status_from_rows(
        rows,
        &snapshot,
        &model_state,
    ))
}

/// Install or update one bundle from its channel. Falls back to the pinned
/// release when no channel is available (initial install on an offline
/// machine); the downloaded manifest still verifies every file.
async fn install_bundle_inner(state: &AppState, asset: &str, force_channel: bool) -> CmdResult<()> {
    let Some(bundle_dir) = bundle_dir(state, asset) else {
        return Err(CommandError::internal(
            "bundle.unknown_asset",
            vec![asset.to_string()],
        ));
    };
    if state.engine_switcher.kind().as_str() == asset {
        return Err(CommandError::internal("bundle.in_use", vec![]));
    }
    let _guard = state
        .bundle_download_lock
        .try_lock()
        .map_err(|_| CommandError::internal("bundle.download_in_progress", vec![]))?;

    let snapshot = model_lifecycle::channels(force_channel).await;
    let channel = snapshot.channels.get(asset);
    // Initial install may use the pinned release when no channel is reachable.
    // An update must not: falling back would silently replace a newer bundle
    // with the version pinned in this app build.
    let installed = model_lifecycle::inspect_bundle(asset, &bundle_dir).installed;
    let manifest_url = if let Some(channel) = channel {
        channel.manifest_url.clone()
    } else if installed {
        return Err(CommandError::internal(
            "bundle.channel_unavailable",
            vec![asset.to_string()],
        ));
    } else {
        pinned_manifest_url(asset).ok_or_else(|| {
            CommandError::internal("bundle.unknown_asset", vec![asset.to_string()])
        })?
    };
    let manifest_sha = channel.map(|c| c.manifest_sha256.as_str());
    let version = channel
        .map(|c| c.bundle_version.as_str())
        .unwrap_or_else(|| default_bundle_version(asset));
    let emitter = Arc::clone(&state.emitter);

    let result = match asset {
        model_lifecycle::SILERO_NATIVE => {
            model_lifecycle::install(
                &bundle_dir,
                asset,
                &manifest_url,
                manifest_sha,
                |bytes| {
                    silero_native::bundle::Manifest::parse(bytes)
                        .map(|manifest| {
                            manifest
                                .files
                                .into_iter()
                                .map(|file| crate::tts::bundle_download::BundleFile {
                                    asset: file.path.clone(),
                                    path: file.path,
                                    size: file.size,
                                    sha256: file.sha256,
                                })
                                .collect()
                        })
                        .map_err(|e| e.to_string())
                },
                version,
                &emitter,
            )
            .await
        }
        model_lifecycle::TERATTS => {
            model_lifecycle::install(
                &bundle_dir,
                asset,
                &manifest_url,
                manifest_sha,
                |bytes| {
                    let manifest: teratts::bundle::Manifest =
                        serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
                    if manifest.format_version != teratts::bundle::FORMAT_VERSION {
                        return Err(format!(
                            "unsupported bundle format {} (expected {})",
                            manifest.format_version,
                            teratts::bundle::FORMAT_VERSION
                        ));
                    }
                    Ok(manifest
                        .files
                        .into_iter()
                        .map(|file| crate::tts::bundle_download::BundleFile {
                            path: file.path,
                            asset: file.asset,
                            size: file.size,
                            sha256: file.sha256,
                        })
                        .collect())
                },
                version,
                &emitter,
            )
            .await
        }
        _ => unreachable!("bundle_dir accepted only known assets"),
    };
    result.map(|_| ()).map_err(bundle_install_error)
}

/// Keep the typed bundle failure code so the frontend can localize known
/// cases (active model, checksum, disk I/O) and fall back to the concrete
/// backend message for network/HTTP failures.
fn bundle_install_error(e: TtsError) -> CommandError {
    match e {
        TtsError::Ttsd { code, message } => {
            CommandError::internal(&code, vec![]).with_message(message)
        }
        TtsError::Ipc(e) => CommandError::internal("bundle.io_error", vec![e.to_string()]),
        other => CommandError::from(other),
    }
}

/// Install or update a model bundle. The active engine's bundle is protected
/// with `bundle.in_use`; the UI asks the user to switch engines first.
#[tauri::command]
pub async fn install_model_asset(state: State<'_, AppState>, asset: String) -> CmdResult<()> {
    install_bundle_inner(&state, &asset, true).await
}

/// Delete a downloaded model asset and return the reclaimed bytes. Refuses the
/// active bundle and the active Piper voice.
#[tauri::command]
pub async fn delete_model_asset(state: State<'_, AppState>, asset: String) -> CmdResult<u64> {
    if let Some(voice_id) = asset.strip_prefix("piper:") {
        let voice = catalog::lookup(voice_id)
            .ok_or_else(|| CommandError::internal("bundle.unknown_asset", vec![asset.clone()]))?;
        let active = state.engine_switcher.kind() == EngineKind::Piper
            && state.engine_switcher.active_piper_voice().await.as_deref() == Some(voice.id);
        if active {
            return Err(CommandError::internal("bundle.in_use", vec![]));
        }
        let dir = state.piper_voices_dir.join(voice.id);
        return model_lifecycle::remove_dir(&dir)
            .await
            .map_err(CommandError::from);
    }
    let Some(dir) = bundle_dir(&state, &asset) else {
        return Err(CommandError::internal(
            "bundle.unknown_asset",
            vec![asset.clone()],
        ));
    };
    if state.engine_switcher.kind().as_str() == asset {
        return Err(CommandError::internal("bundle.in_use", vec![]));
    }
    let _guard = state
        .bundle_download_lock
        .try_lock()
        .map_err(|_| CommandError::internal("bundle.download_in_progress", vec![]))?;
    model_lifecycle::remove_dir(&dir)
        .await
        .map_err(CommandError::from)
}

/// Defer a specific channel version: automatic checks stop advertising it,
/// while the manual "Update" action remains available.
#[tauri::command]
pub async fn skip_model_update(asset: String, version: String) -> CmdResult<()> {
    let path = model_state_path()?;
    let mut model_state = ModelState::load(&path);
    model_state.skip(&asset, &version);
    model_state.save(&path).map_err(|e| {
        CommandError::internal("models.state_save_failed", vec![]).with_message(e.to_string())
    })
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
/// Python package and the `uv` toolchain; Silero Native requires the
/// downloaded model bundle — see [`tts::availability`]. Cheap (filesystem
/// stats + one `uv --version` exec); safe to call on every Settings dialog
/// open.
#[tauri::command]
pub async fn get_available_engines(state: State<'_, AppState>) -> CmdResult<AvailableEngines> {
    let ttsd_dir = state.ttsd_dir.clone();
    let silero_native_bundle_dir = state.silero_native_bundle_dir.clone();
    let teratts_bundle_dir = state.teratts_bundle_dir.clone();
    tokio::task::spawn_blocking(move || {
        availability::probe(&ttsd_dir, &silero_native_bundle_dir, &teratts_bundle_dir)
    })
    .await
    .map_err(|e| {
        CommandError::internal("engines.probe_panicked", vec![]).with_message(e.to_string())
    })
}

/// Voice ids shipped by the installed TeraTTS bundle, in manifest order.
///
/// Feeds the Settings voice selector so it cannot drift from the bundle. Reads
/// only `manifest.json`; when the bundle is missing or unreadable the list is
/// empty — [`get_available_engines`] carries the reason the engine is unusable.
#[tauri::command]
pub async fn get_teratts_voices(state: State<'_, AppState>) -> CmdResult<Vec<String>> {
    let bundle_dir = state.teratts_bundle_dir.clone();
    tokio::task::spawn_blocking(move || crate::tts::teratts::voices(&bundle_dir))
        .await
        .map_err(|e| {
            CommandError::internal("teratts.voices_panicked", vec![]).with_message(e.to_string())
        })
}

/// Download the Silero Native model bundle on user demand. Idempotent —
/// files already present with a matching checksum are skipped. Progress is
/// delivered via the `bundle_download_started` / `bundle_download_progress`
/// / `bundle_download_finished` events; the `Result` here only reports the
/// final outcome so the frontend can show one final notification.
#[tauri::command]
pub async fn download_silero_native_bundle(state: State<'_, AppState>) -> CmdResult<()> {
    install_bundle_inner(&state, model_lifecycle::SILERO_NATIVE, false).await
}

/// Download the TeraTTSv2 model bundle on user demand. Idempotent — files
/// already present with a matching checksum are skipped. Progress is delivered
/// via the `bundle_download_started` / `bundle_download_progress` /
/// `bundle_download_finished` events with `engine: "teratts"`; the `Result`
/// here only reports the final outcome so the frontend can show one terminal
/// notification.
#[tauri::command]
pub async fn download_teratts_bundle(state: State<'_, AppState>) -> CmdResult<()> {
    install_bundle_inner(&state, model_lifecycle::TERATTS, false).await
}

/// Apply the persisted text-shaping settings to the shared pipeline. Called at
/// startup so the pipeline never narrates code blocks or English prose in a
/// mode the config did not ask for; `update_config` pushes later changes
/// itself.
pub fn apply_configured_text_settings(pipeline: &Arc<Mutex<TTSPipeline>>, config: &UIConfig) {
    let code_block_mode = crate::pipeline::normalizers::code_blocks::CodeBlockMode::from_config(
        &config.code_block_mode,
    );
    let english_reading = crate::pipeline::EnglishReading::from_config(&config.english_reading);
    let mut pipeline = pipeline.lock();
    pipeline.set_code_block_mode(code_block_mode);
    pipeline.set_english_reading(english_reading);
}

/// Merge a partial config patch into the current configuration, swap the
/// active TTS engine if needed, and persist. The engine swap runs *before*
/// the config is saved — if the user picked a Silero stack we cannot spawn,
/// the call returns an error and the previous config stays on disk.
///
/// A `code_block_mode` or `english_reading` change is pushed into the shared
/// pipeline after the save, so subsequent synthesis/preview runs pick it up
/// without a restart (synthesis already in flight finishes on the previous
/// values). The tray icon is created or removed after the save as well,
/// tracking `hide_to_tray_on_close`.
#[tauri::command]
pub async fn update_config<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    patch: UIConfigPatch,
) -> CmdResult<()> {
    let mut config = state.storage.load_config().unwrap_or_default();
    let previous_engine = state.engine_switcher.kind();
    // Remembered to roll the setting back when the tray icon cannot be
    // created: the persisted flag must match what the app can actually do.
    let hide_to_tray_before = config.hide_to_tray_on_close;
    apply_config_patch(&mut config, patch);

    state
        .engine_switcher
        .apply_config(&config.engine, &config.piper_voice)
        .await
        .map_err(|e| match e {
            // ttsd-side failures carry a dotted site id already (e.g.
            // "native.bundle_missing") — pass it through so the frontend can
            // show *why* the switch failed instead of the generic sentence;
            // unknown codes fall back to the message on the frontend.
            TtsError::Ttsd { code, message } => {
                CommandError::config(&code, vec![]).with_message(message)
            }
            other => CommandError::config("config.engine_switch_failed", vec![])
                .with_message(other.to_string()),
        })?;

    state
        .storage
        .save_config(&config)
        .map_err(CommandError::from)?;

    // The merged user dictionary is engine-scoped: only a real engine switch
    // needs a rebuild, and the pipeline lock keeps it atomic with the
    // dictionary commands. Other patches must not touch the dictionary.
    if state.engine_switcher.kind() != previous_engine {
        let mut pipeline = state.pipeline.lock();
        let book = state.dictionary_store.load_book();
        pipeline.set_user_dictionary(book.merged_for(state.engine_switcher.kind()));
    }

    apply_configured_text_settings(&state.pipeline, &config);

    if let Err(e) = crate::tray::set_enabled(&app, config.hide_to_tray_on_close) {
        // The icon could not be created (no system tray, unwritable runtime
        // dir). Roll the setting back so the stored state matches reality:
        // the switch must not promise a tray that does not exist.
        tracing::warn!("tray: failed to create the icon: {e}");
        config.hide_to_tray_on_close = hide_to_tray_before;
        state
            .storage
            .save_config(&config)
            .map_err(CommandError::from)?;
        return Err(
            CommandError::config("config.tray_unavailable", vec![]).with_message(e.to_string())
        );
    }
    Ok(())
}

/// Shared implementation for [`get_timestamps`]: an entry that exists but has
/// no timestamps file yields an empty vector; an unknown id is `not_found`.
fn load_timestamps_for_entry(storage: &StorageService, id: &str) -> CmdResult<Vec<WordTimestamp>> {
    // Verify entry exists.
    let entry = require_entry(storage, id)?;

    let timestamps = storage
        .load_timestamps(&entry.id)
        .map_err(CommandError::from)?
        .unwrap_or_default();

    Ok(timestamps)
}

/// Load and return word timestamps for an entry.
#[tauri::command]
pub async fn get_timestamps(
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<Vec<WordTimestamp>> {
    load_timestamps_for_entry(&state.storage, &id)
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
pub async fn clear_cache<R: Runtime>(
    app: AppHandle<R>,
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
    .map_err(|e| {
        CommandError::internal("cache.task_panicked", vec![]).with_message(e.to_string())
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

/// Absolute path to the on-disk data directory (`~/.local/share/ruvox/` on
/// Linux, `%LOCALAPPDATA%\com.ruvox.app` on Windows — see `crate::paths`).
/// The frontend uses this to display the path in Settings and to pass it to
/// `revealItemInDir` for opening the folder in the OS file manager.
#[tauri::command]
pub async fn get_cache_dir(state: State<'_, AppState>) -> CmdResult<String> {
    Ok(state.storage.data_dir().to_string_lossy().into_owned())
}

/// Absolute path of the per-user log directory (the same one `tauri-plugin-log`
/// writes its rotated files into). The frontend reveals it in the OS file
/// manager so the user can grab logs for a support request.
#[tauri::command]
pub async fn get_log_dir<R: Runtime>(app: AppHandle<R>) -> CmdResult<String> {
    let dir = app.path().app_log_dir().map_err(|e| {
        CommandError::internal("logs.dir_resolve_failed", vec![]).with_message(e.to_string())
    })?;
    // The logger creates the dir lazily on first write; create it now so the
    // frontend can reveal a real path even before any log line is flushed.
    // Run on a blocking thread so this async command does not park the
    // executor's worker.
    tauri::async_runtime::spawn_blocking({
        let dir = dir.clone();
        move || std::fs::create_dir_all(dir)
    })
    .await
    .map_err(|e| {
        CommandError::internal("logs.dir_create_failed", vec![]).with_message(e.to_string())
    })?
    .map_err(|e| {
        CommandError::internal("logs.dir_create_failed", vec![]).with_message(e.to_string())
    })?;
    Ok(dir.to_string_lossy().into_owned())
}

/// Upper bound for images fetched by [`fetch_image_bytes`]. The bytes travel
/// through IPC into the webview and end up on the clipboard; an uncapped
/// download would let a hostile page pin unbounded memory.
const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;

/// Fetch a remote image over http(s) and return its raw bytes for the
/// viewer's "Copy image" action (#231).
///
/// Lives in Rust instead of the frontend `tauri-plugin-http` capability so
/// the webview holds no blanket arbitrary-host network permission: each
/// request is validated here (scheme, content-type or magic bytes, size cap)
/// and the result only ever surfaces as clipboard-bound image bytes.
#[tauri::command]
pub async fn fetch_image_bytes(url: String) -> CmdResult<Vec<u8>> {
    let parsed = validate_image_url(&url)?;

    let response = http_client().get(parsed).send().await.map_err(|e| {
        CommandError::internal("image.fetch_failed", vec![]).with_message(e.to_string())
    })?;
    if !response.status().is_success() {
        return Err(CommandError::internal(
            "image.fetch_failed",
            vec![response.status().as_u16().to_string()],
        ));
    }
    // Reject early when Content-Length is known; still re-check after the
    // body is read — chunked responses have no reliable length up front.
    if let Some(len) = response.content_length() {
        ensure_image_size(len)?;
    }
    let declared_type = media_type(response.headers().get(reqwest::header::CONTENT_TYPE));
    let bytes = response.bytes().await.map_err(|e| {
        CommandError::internal("image.read_failed", vec![]).with_message(e.to_string())
    })?;
    ensure_image_size(bytes.len() as u64)?;
    // Content-type gate: trust a declared image/* type; otherwise fall back
    // to magic-byte sniffing — attachment endpoints commonly serve real
    // images as application/octet-stream.
    match declared_type.as_deref() {
        Some(ct) if ct.starts_with("image/") => {}
        other => {
            if image::guess_format(&bytes).is_err() {
                return Err(match other {
                    Some(ct) => CommandError::internal("image.not_image", vec![ct.to_string()]),
                    None => CommandError::internal("image.no_content_type", vec![]),
                });
            }
        }
    }
    Ok(bytes.into())
}

// Import commands live in commands/import.rs (domain seam, see review);
// the glob below re-exports them for lib.rs registration.
mod import;
pub use import::{fetch_url_text, pick_import_file, read_text_file};

// Audio export commands live in commands/export.rs (same domain-seam split).
mod export;
pub use export::{export_audio, pick_export_audio_path};

// User dictionary commands live in commands/dictionary.rs (same seam split).
mod dictionary;
pub use dictionary::{
    export_dictionary, get_dictionaries, import_dictionary, pick_dictionary_export_path,
    pick_dictionary_import_path, save_dictionary,
};

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Shared HTTP client for [`fetch_image_bytes`] and [`fetch_url_text`].
/// Built once (a fresh client per call would rebuild the TLS config every
/// time) and bounded: without a total timeout a stalled server would hang
/// the caller forever with no defined failure path.
fn http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("static reqwest client options are valid")
    })
}

/// What went wrong while parsing an absolute http(s) URL
/// ([`parse_http_url`]). Callers map variants onto their own wire-code
/// namespace (`image.*`, `import.*`).
enum HttpUrlError {
    Invalid { input: String },
    SchemeUnsupported { scheme: String },
}

/// Parse and validate an absolute http(s) URL for outbound fetches: only
/// absolute `http`/`https` URLs are accepted (no `file:`, `data:`, custom
/// schemes). Plain http stays allowed deliberately — CSP limits *display*
/// of plain-http images, but clipboard-bound bytes and text imports are
/// inert reads, so legacy plain-http sources remain reachable.
fn parse_http_url(url: &str) -> Result<reqwest::Url, HttpUrlError> {
    let parsed = reqwest::Url::parse(url).map_err(|_| HttpUrlError::Invalid {
        input: url.to_string(),
    })?;
    match parsed.scheme() {
        "https" | "http" => Ok(parsed),
        scheme => Err(HttpUrlError::SchemeUnsupported {
            scheme: format!("{scheme}:"),
        }),
    }
}

fn map_url_error(code_namespace: &str, e: HttpUrlError) -> CommandError {
    match e {
        HttpUrlError::Invalid { input } => {
            CommandError::internal(format!("{code_namespace}.url_invalid"), vec![input])
        }
        HttpUrlError::SchemeUnsupported { scheme } => CommandError::internal(
            format!("{code_namespace}.url_scheme_unsupported"),
            vec![scheme],
        ),
    }
}

/// Parse and validate an image URL for [`fetch_image_bytes`] on top of
/// [`parse_http_url`], keeping the historical `image.*` wire codes.
fn validate_image_url(url: &str) -> Result<reqwest::Url, CommandError> {
    parse_http_url(url).map_err(|e| map_url_error("image", e))
}

/// Lowercase media type of a Content-Type header (parameters stripped),
/// `None` when the header is absent, not valid UTF-8, or empty.
fn media_type(content_type: Option<&reqwest::header::HeaderValue>) -> Option<String> {
    let value = content_type?.to_str().ok()?;
    let mt = value.split(';').next()?.trim().to_ascii_lowercase();
    if mt.is_empty() { None } else { Some(mt) }
}

/// Enforce [`MAX_IMAGE_BYTES`] for both the header pre-check and the
/// post-read re-check of [`fetch_image_bytes`].
fn ensure_image_size(len: u64) -> Result<(), CommandError> {
    if len > MAX_IMAGE_BYTES as u64 {
        return Err(CommandError::internal(
            "image.too_large",
            vec![len.to_string(), MAX_IMAGE_BYTES.to_string()],
        ));
    }
    Ok(())
}

fn parse_entry_id(s: &str) -> CmdResult<EntryId> {
    s.parse::<uuid::Uuid>().map_err(|e| {
        CommandError::not_found("entry.id_invalid", vec![s.to_string()]).with_message(e.to_string())
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
        config.code_block_mode = UIConfig::canonical_code_block_mode(&v);
    }
    if let Some(v) = patch.english_reading {
        config.english_reading = UIConfig::canonical_english_reading(&v);
    }
    if let Some(v) = patch.reading_width {
        config.reading_width = UIConfig::canonical_reading_width(&v).to_owned();
    }
    if let Some(v) = patch.reading_scale {
        config.reading_scale = UIConfig::clamp_reading_scale(v);
    }
    if let Some(v) = patch.theme {
        config.theme = v;
    }
    if let Some(v) = patch.window_geometry {
        config.window_geometry = v;
    }
    if let Some(v) = patch.preview_dialog_enabled {
        config.preview_dialog_enabled = v;
    }
    if let Some(v) = patch.hide_to_tray_on_close {
        config.hide_to_tray_on_close = v;
    }
    if let Some(v) = patch.engine {
        config.engine = v;
    }
    if let Some(v) = patch.piper_voice {
        config.piper_voice = v;
    }
    if let Some(v) = patch.teratts_voice {
        config.teratts_voice = v;
    }
    if let Some(v) = patch.language {
        config.language = v;
    }
}

#[cfg(test)]
mod image_url_tests;
#[cfg(test)]
mod synthesis_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod updater_tests;

#[cfg(test)]
mod orchestration_tests;
