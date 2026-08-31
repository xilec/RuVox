//! Tauri command handlers (IPC Layer 1: Frontend → Backend).
//!
//! All commands use `Result<T, CommandError>` so that errors are serialized as
//! typed JSON objects (`{ "type": "...", "code": "...", "params": [...],
//! "message": "..." }`) which the frontend localizes by `code` and falls back
//! to `message` for unknown codes.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tauri_plugin_mpv::MpvExt;
use tokio::task::AbortHandle;
use tracing::{info, warn};

use crate::pipeline::TTSPipeline;
use crate::pipeline::tracked_text::CharMapping;
use crate::state::AppState;
use crate::storage::schema::{
    EntryId, EntrySource, EntryStatus, GenerationParams, ModelParams, TextEntry, TextFormat,
    UIConfig, UIConfigPatch, WordTimestamp,
};
use crate::storage::service::{StorageError, StorageService};
use crate::tts::engine::EngineKind;
use crate::tts::piper::download::download_voice;
use crate::tts::{
    AvailableEngines, CharMappingEntry, ModelInfo, SynthesizeOutput, TtsEngine, TtsError,
    availability,
};

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

/// Everything [`spawn_synthesis`] needs, snapshotted in one shot so call
/// sites stay one line. Built from the managed state via
/// [`SynthesisDeps::from_state`] in commands; the tray handler (which runs
/// before the state exists) fills the struct literal directly.
pub struct SynthesisDeps<R: Runtime> {
    pub app: AppHandle<R>,
    pub storage: Arc<StorageService>,
    pub tts: Arc<dyn TtsEngine>,
    pub piper_voices_dir: PathBuf,
    pub emitter: crate::tts::supervisor::Emitter,
    pub player: Arc<dyn crate::player::PlayerBackend>,
    pub pipeline: Arc<Mutex<TTSPipeline>>,
    pub synthesis_tasks: Arc<Mutex<HashMap<EntryId, AbortHandle>>>,
    pub synthesize_entered: Arc<Mutex<HashSet<EntryId>>>,
}

// Manual impl: `#[derive(Clone)]` would add an `R: Clone` bound, which
// `Runtime` does not guarantee (every field is cheap-Clone regardless).
impl<R: Runtime> Clone for SynthesisDeps<R> {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
            storage: Arc::clone(&self.storage),
            tts: Arc::clone(&self.tts),
            piper_voices_dir: self.piper_voices_dir.clone(),
            emitter: Arc::clone(&self.emitter),
            player: Arc::clone(&self.player),
            pipeline: Arc::clone(&self.pipeline),
            synthesis_tasks: Arc::clone(&self.synthesis_tasks),
            synthesize_entered: Arc::clone(&self.synthesize_entered),
        }
    }
}

impl<R: Runtime> SynthesisDeps<R> {
    /// Snapshot the synthesis-relevant pieces of the managed app state.
    pub fn from_state(app: &AppHandle<R>, state: &AppState) -> Self {
        Self {
            app: app.clone(),
            storage: Arc::clone(&state.storage),
            tts: Arc::clone(&state.tts),
            piper_voices_dir: state.piper_voices_dir.clone(),
            emitter: Arc::clone(&state.emitter),
            player: Arc::clone(&state.player),
            pipeline: Arc::clone(&state.pipeline),
            synthesis_tasks: Arc::clone(&state.synthesis_tasks),
            synthesize_entered: Arc::clone(&state.synthesize_entered),
        }
    }
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

/// Shared core for [`run_normalization`] and [`preview_normalization`]: runs
/// the CPU-bound pipeline on a blocking thread. Also returns the code block
/// narration mode captured in the same critical section that produced the
/// text, so the generation snapshot can record the mode actually applied
/// (a config change racing a synthesis cannot falsify it). The raw
/// `JoinError` is returned so each caller maps a pipeline panic to its own
/// error type.
async fn run_pipeline_normalization(
    pipeline: Arc<Mutex<TTSPipeline>>,
    text: String,
) -> Result<(String, CharMapping, String), tokio::task::JoinError> {
    tokio::task::spawn_blocking(move || {
        let mut p = pipeline.lock();
        let (normalized, mapping) = p.process_with_char_mapping(&text);
        let code_block_mode = p.code_block_mode().as_config_str().to_string();
        (normalized, mapping, code_block_mode)
    })
    .await
}

/// Phase 1: run Rust text pipeline (CPU-bound, runs in blocking thread).
async fn run_normalization(
    pipeline: Arc<Mutex<TTSPipeline>>,
    original_text: String,
) -> Result<(String, CharMapping, String), SynthesisError> {
    let (normalized, mapping, code_block_mode) =
        run_pipeline_normalization(pipeline, original_text)
            .await
            .map_err(|e| SynthesisError::PipelinePanic(e.to_string()))?;

    if normalized.is_empty() {
        return Err(SynthesisError::EmptyText);
    }
    Ok((normalized, mapping, code_block_mode))
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

/// Voice id for the engine actually serving the request: Piper uses
/// `piper_voice` (e.g. "ruslan"), both Silero engines (ttsd and native) use
/// `speaker` (e.g. "xenia"). Keyed on the active engine kind, not the
/// persisted `config.engine`, so a session with a startup-fallback engine
/// still gets its own voice choice.
fn voice_for_engine(kind: EngineKind, config: &UIConfig) -> String {
    match kind {
        EngineKind::Piper => config.piper_voice.clone(),
        EngineKind::Silero | EngineKind::SileroNative => config.speaker.clone(),
    }
}

/// Everything later synthesis phases need from [`synthesize_audio`]: the
/// engine output plus the inputs resolved *for this request* — the snapshot
/// must record the voice/settings actually used, not a fresh config read
/// (the user may flip settings while a synthesis is in flight).
#[derive(Debug)]
struct SynthOutcome {
    output: SynthesizeOutput,
    out_wav_path: PathBuf,
    wav_filename: String,
    voice: String,
}

/// Phases 3–4: determine the WAV path / config / char-mapping inputs and
/// call `tts.synthesize`. Returns the synthesize output along with the
/// resolved WAV path / filename (so [`finalize_audio_files`] can transcode
/// to Opus without rebuilding them) and the resolved voice (so
/// [`build_generation_snapshot`] records what was actually used).
///
/// When the engine returns `voice_not_installed` and the active engine is
/// Piper, the function auto-fetches the voice files via
/// [`crate::tts::piper::download::download_voice`] and retries once. The
/// retry runs only on Piper because Silero is bundled — its Python venv
/// already includes the model.
#[allow(clippy::too_many_arguments)]
async fn synthesize_audio(
    tts: &dyn TtsEngine,
    storage: &StorageService,
    piper_voices_dir: &std::path::Path,
    emitter: &crate::tts::supervisor::Emitter,
    synthesize_entered: &Mutex<HashSet<EntryId>>,
    entry_id: &EntryId,
    normalized: String,
    mapping: &CharMapping,
) -> Result<SynthOutcome, SynthesisError> {
    // ttsd writes WAV; finalize_audio_files transcodes it to Opus right after.
    let wav_filename = format!("{entry_id}.wav");
    let out_wav_path = storage.data_dir().join("audio").join(&wav_filename);
    let out_wav = out_wav_path.to_string_lossy().into_owned();

    let config = storage.load_config().unwrap_or_default();
    let tts_char_mapping = if mapping.char_map.is_empty() {
        None
    } else {
        Some(char_mapping_to_entries(mapping))
    };

    // Voice id is engine-specific: Piper uses `piper_voice` (e.g. "ruslan"),
    // both Silero engines (ttsd and native) use `speaker` (e.g. "xenia").
    // Keeping them in two distinct config fields means flipping engines
    // preserves each side's choice.
    // The choice keys on the engine actually serving this request
    // (`tts.kind()`), not the persisted `config.engine`: when the startup
    // fallback swapped engines (e.g. silero_native without a bundle runs
    // Piper for the session), the active engine must get its own voice id.
    let voice = voice_for_engine(tts.kind(), &config);

    // Track the entry as "inside the TTS stage" so `cancel_synthesis` knows
    // the ttsd subprocess must be killed. If the task is aborted at this
    // await, `cancel_synthesis` removes the marker itself.
    synthesize_entered.lock().insert(*entry_id);
    let attempt = tts
        .synthesize(
            normalized.clone(),
            voice.clone(),
            config.sample_rate,
            out_wav.clone(),
            tts_char_mapping.clone(),
        )
        .await;
    synthesize_entered.lock().remove(entry_id);

    let output = match attempt {
        Ok(o) => o,
        Err(TtsError::Ttsd { code, message })
            if code == "voice_not_installed" && tts.kind() == EngineKind::Piper =>
        {
            info!("voice \"{voice}\" not installed; auto-downloading then retrying ({message})");
            crate::tts::piper::download::download_voice(piper_voices_dir, &voice, emitter)
                .await
                .map_err(|e| SynthesisError::TtsFailed(e.to_string()))?;
            synthesize_entered.lock().insert(*entry_id);
            let retry = tts
                .synthesize(
                    normalized,
                    voice.clone(),
                    config.sample_rate,
                    out_wav,
                    tts_char_mapping,
                )
                .await;
            synthesize_entered.lock().remove(entry_id);
            retry.map_err(|e| SynthesisError::TtsFailed(e.to_string()))?
        }
        Err(e) => return Err(SynthesisError::TtsFailed(e.to_string())),
    };

    Ok(SynthOutcome {
        output,
        out_wav_path,
        wav_filename,
        voice,
    })
}

/// Actual output sample rate of a rendered WAV, read from the header. The
/// engine has already written the complete file by the time this runs, so a
/// header read captures the produced rate for every engine (including
/// Piper's voice-fixed rate). `None` when the file cannot be read.
fn wav_sample_rate(wav_path: &std::path::Path) -> Option<u32> {
    hound::WavReader::open(wav_path)
        .ok()
        .map(|reader| reader.spec().sample_rate)
}

/// Codec of the stored audio file, keyed off the final filename: the Opus
/// transcode renamed the intermediate WAV to `.opus`, or the encode failed
/// and the `.wav` stayed as the playback fallback.
fn audio_codec_for(audio_filename: &str) -> Option<&'static str> {
    if audio_filename.ends_with(".opus") {
        Some("Ogg Opus")
    } else if audio_filename.ends_with(".wav") {
        Some("WAV")
    } else {
        None
    }
}

fn model_params(info: ModelInfo) -> ModelParams {
    ModelParams {
        name: info.name,
        sha256: info.sha256,
    }
}

/// Build the per-entry synthesis-parameter snapshot (spec `ipc-commands`,
/// "Generation Parameters Snapshot"). `voice` / `config` / `sample_rate` come
/// from the synthesis request itself — the resolved voice and the WAV header
/// read *before* the Opus transcode removes the intermediate file — not from
/// a fresh config read. Runs after [`finalize_audio_files`], when the final
/// audio file exists. Every lookup is best-effort: unknown values stay
/// `None` — a failed manifest read or a missing file must never fail a
/// completed synthesis.
async fn build_generation_snapshot(
    tts: &dyn TtsEngine,
    storage: &StorageService,
    voice: &str,
    code_block_mode: &str,
    normalized: &str,
    sample_rate: Option<u32>,
    audio_filename: &str,
) -> GenerationParams {
    let model = tts.model_info(voice).await.map(model_params);

    let normalized_sha = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        Some(format!("{:x}", hasher.finalize()))
    };

    let audio_path = storage.data_dir().join("audio").join(audio_filename);
    let audio_bytes = std::fs::metadata(&audio_path).ok().map(|m| m.len());

    GenerationParams {
        engine: tts.kind().as_str().to_string(),
        voice: voice.to_string(),
        sample_rate,
        model,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        code_block_mode: Some(code_block_mode.to_string()),
        normalized_text_sha256: normalized_sha,
        audio_codec: audio_codec_for(audio_filename).map(str::to_string),
        audio_bytes,
    }
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

/// Stale-completion guard: a synthesis completion or failure applies only
/// while the entry is still `processing`. A cancelled entry is already back
/// at `pending`, so its late result must be discarded instead of
/// resurrecting it to `ready` / `error`.
fn completion_is_current(status: EntryStatus) -> bool {
    status == EntryStatus::Processing
}

/// Best-effort removal of the audio/timestamp files a discarded late result
/// wrote into the audio dir. Missing files are fine (e.g. the Opus transcode
/// never ran on the failure path).
fn discard_late_files<I: IntoIterator<Item = String>>(storage: &StorageService, names: I) {
    let audio_dir = storage.data_dir().join("audio");
    for name in names {
        let path = audio_dir.join(name);
        match std::fs::remove_file(&path) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => warn!("failed to remove stale result file {}: {e}", path.display()),
        }
    }
}

/// Phase 6 core (no Tauri handles, so the stale guard is unit-testable):
/// mark the entry `Ready` with audio + timestamp paths — but only if it is
/// still `processing`. A late completion for a non-`processing` entry is
/// discarded together with the files it just wrote. Returns `true` when the
/// result was applied.
fn apply_ready_if_current(
    storage: &StorageService,
    entry_id: &EntryId,
    output: &SynthesizeOutput,
    ts_filename: Option<String>,
    audio_filename: &str,
    generation: GenerationParams,
) -> bool {
    // Atomic under a single storage lock (issue #179): the `processing` check
    // and the ready-field mutation cannot be raced by a concurrent cancel.
    let applied = storage.update_entry_if(
        entry_id,
        |e| completion_is_current(e.status),
        |e| {
            e.status = EntryStatus::Ready;
            e.audio_path = Some(audio_filename.to_string());
            e.timestamps_path = ts_filename.clone();
            e.duration_sec = Some(output.duration_sec);
            e.audio_generated_at = Some(chrono::Utc::now().naive_utc());
            e.generation_count = e.generation_count.saturating_add(1);
            e.generation = Some(generation);
        },
    );

    if !applied {
        // A late result whose entry vanished or left `processing` is dropped
        // together with the files it just wrote.
        if let Some(status) = storage.get_entry(entry_id).map(|e| e.status) {
            info!("discarding stale completion for {entry_id} (status: {status:?})");
        }
        discard_late_files(
            storage,
            [Some(audio_filename.to_string()), ts_filename]
                .into_iter()
                .flatten(),
        );
    }
    applied
}

/// Phase 6: apply the synthesis result and, when it was applied, emit
/// `entry_updated`. Returns whether the entry reached `ready` — the caller
/// gates autoplay on it.
fn mark_ready_and_emit<R: Runtime>(
    storage: &StorageService,
    app: &AppHandle<R>,
    entry_id: &EntryId,
    output: &SynthesizeOutput,
    ts_filename: Option<String>,
    audio_filename: &str,
    generation: GenerationParams,
) -> bool {
    let applied = apply_ready_if_current(
        storage,
        entry_id,
        output,
        ts_filename,
        audio_filename,
        generation,
    );
    if applied {
        if let Some(entry) = storage.get_entry(entry_id) {
            emit_entry_updated(app, &entry);
        }
        info!("synthesis complete: entry_id={entry_id}");
    }
    applied
}

/// Phase 7: kick off auto-play. Errors are logged and swallowed — failed
/// auto-play must not flip the entry into `Error`.
fn autoplay(player: &dyn crate::player::PlayerBackend, audio_path: PathBuf, entry_id: &EntryId) {
    if let Err(e) = player.load(&audio_path, entry_id.to_string()) {
        warn!("auto-play load failed: {e}");
    } else if let Err(e) = player.play() {
        warn!("auto-play play failed: {e}");
    }
}

/// Run the full synthesis pipeline for `entry_id` in a background task.
///
/// The task's `AbortHandle` is registered in `synthesis_tasks` so
/// `cancel_synthesis` can abort it; a detached reaper removes the task's
/// registry entry once it terminates, taking care not to remove a newer
/// task's handle for the same entry.
pub fn spawn_synthesis<R: Runtime + 'static>(
    deps: SynthesisDeps<R>,
    entry_id: EntryId,
    play_when_ready: bool,
) {
    let SynthesisDeps {
        app,
        storage,
        tts,
        piper_voices_dir,
        emitter,
        player,
        pipeline,
        synthesis_tasks,
        synthesize_entered,
    } = deps;
    let entered_for_task = Arc::clone(&synthesize_entered);
    let tasks_for_cleanup = Arc::clone(&synthesis_tasks);
    let entered_for_cleanup = Arc::clone(&synthesize_entered);
    let handle = tokio::spawn(async move {
        async {
            let Some(entry) = storage.get_entry(&entry_id) else {
                warn!("synthesis task: entry {entry_id} vanished before synthesis started");
                return;
            };

            let result: Result<(), SynthesisError> = async {
                let (normalized, mapping, applied_code_block_mode) =
                    run_normalization(Arc::clone(&pipeline), entry.original_text.clone()).await?;
                mark_processing(&storage, &app, &entry_id, &normalized);
                let normalized_for_snapshot = normalized.clone();
                let outcome = synthesize_audio(
                    tts.as_ref(),
                    &storage,
                    &piper_voices_dir,
                    &emitter,
                    &entered_for_task,
                    &entry_id,
                    normalized,
                    &mapping,
                )
                .await?;
                // Read the header before finalize_audio_files transcodes the
                // WAV to Opus and removes the intermediate file.
                let sample_rate = wav_sample_rate(&outcome.out_wav_path);
                let (ts_filename, audio_filename) = finalize_audio_files(
                    &storage,
                    &entry_id,
                    &outcome.output,
                    outcome.out_wav_path,
                    &outcome.wav_filename,
                )
                .await;
                let generation = build_generation_snapshot(
                    tts.as_ref(),
                    &storage,
                    &outcome.voice,
                    &applied_code_block_mode,
                    &normalized_for_snapshot,
                    sample_rate,
                    &audio_filename,
                )
                .await;
                let applied = mark_ready_and_emit(
                    &storage,
                    &app,
                    &entry_id,
                    &outcome.output,
                    ts_filename,
                    &audio_filename,
                    generation,
                );
                if applied && play_when_ready {
                    let path = storage.data_dir().join("audio").join(&audio_filename);
                    autoplay(player.as_ref(), path, &entry_id);
                }
                Ok(())
            }
            .await;

            if let Err(err) = result {
                let msg = err.user_message();
                tracing::error!("synthesis failed for {entry_id}: {msg}");
                // Normalization-stage failures arrive while the entry is
                // legitimately still `pending` (mark_processing runs after
                // normalization), so the stale guard only applies to
                // TTS-stage failures.
                let require_processing = matches!(err, SynthesisError::TtsFailed(_));
                let applied = set_entry_error(&storage, &app, &entry_id, &msg, require_processing);
                if applied {
                    if let SynthesisError::TtsFailed(tts_msg) = err {
                        let _ = app.emit(
                            "tts_error",
                            json!({ "entry_id": entry_id.to_string(), "message": tts_msg }),
                        );
                    }
                }
            }
        }
        .await;
    });

    // Registry cleanup runs in a reaper that awaits the task's JoinHandle:
    // only then is the task's registered handle truly `is_finished()`, which
    // lets the cleanup distinguish its own handle from a newer live one — a
    // task spawned later for the same entry (e.g. by regenerate_entry) must
    // stay cancellable. The reaper is detached (its JoinHandle dropped).
    let abort_handle = handle.abort_handle();
    drop(tokio::spawn(async move {
        let result = handle.await;
        cleanup_finished_handle(&tasks_for_cleanup, &entry_id);
        // `synthesize_entered` is unmarked right after the synthesize await
        // on the normal path and by `cancel_synthesis` on the abort path;
        // only a panic can leave the entry marked, so only then clean up.
        if let Err(e) = &result {
            if e.is_panic() {
                entered_for_cleanup.lock().remove(&entry_id);
            }
        }
    }));

    let already_finished = abort_handle.is_finished();
    let mut tasks = synthesis_tasks.lock();
    tasks.insert(entry_id, abort_handle);
    if already_finished {
        // The task already finished and its reaper already ran — don't
        // leave a stale handle behind.
        tasks.remove(&entry_id);
    }
}

/// Remove `entry_id` from the abort registry only if the registered handle
/// has already finished — i.e. it belongs to the completed task being
/// cleaned up, not to a newer live task for the same entry.
fn cleanup_finished_handle(tasks: &Mutex<HashMap<EntryId, AbortHandle>>, entry_id: &EntryId) {
    let mut tasks = tasks.lock();
    if tasks.get(entry_id).is_some_and(AbortHandle::is_finished) {
        tasks.remove(entry_id);
    }
}

/// Core of [`set_entry_error`] without Tauri handles: flip the entry to
/// `error`. With `require_processing` the stale-completion guard applies —
/// a failure arriving for an entry that left `processing` (e.g. cancelled
/// back to `pending`) is discarded together with the files the late request
/// may have written. Normalization-stage failures pass `false` because the
/// entry is legitimately still `pending` before `mark_processing` runs.
/// Returns `true` when the error was applied.
fn apply_error_if_current(
    storage: &StorageService,
    entry_id: &EntryId,
    message: &str,
    require_processing: bool,
) -> bool {
    // Atomic under a single storage lock (issue #179): when `require_processing`
    // is set, the `processing` check and the error mutation cannot be raced by a
    // concurrent cancel. Normalization-stage failures pass `false` (the entry is
    // legitimately still `pending` before `mark_processing` runs), so the
    // predicate always applies.
    let applied = storage.update_entry_if(
        entry_id,
        |e| !require_processing || completion_is_current(e.status),
        |e| {
            e.status = EntryStatus::Error;
            e.error_message = Some(message.to_string());
        },
    );

    if !applied && require_processing {
        // The entry vanished (deleted) or left `processing`: this stale failure
        // is dropped. A dying ttsd may have left a partial WAV; remove every
        // candidate file so it does not orphan.
        match storage.get_entry(entry_id).map(|e| e.status) {
            Some(status) if !completion_is_current(status) => {
                info!("discarding stale failure for {entry_id} (status: {status:?})");
                discard_late_files(
                    storage,
                    [
                        format!("{entry_id}.wav"),
                        format!("{entry_id}.opus"),
                        format!("{entry_id}.timestamps.json"),
                    ],
                );
            }
            _ => {}
        }
    }
    applied
}

fn set_entry_error<R: Runtime>(
    storage: &StorageService,
    app: &AppHandle<R>,
    entry_id: &EntryId,
    message: &str,
    require_processing: bool,
) -> bool {
    let applied = apply_error_if_current(storage, entry_id, message, require_processing);
    if applied {
        if let Some(entry) = storage.get_entry(entry_id) {
            emit_entry_updated(app, &entry);
        }
    }
    applied
}

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
/// mapping. Unlike [`run_normalization`], empty input is not an error — the
/// preview dialog must show even an empty normalization result.
///
/// Takes no storage/TTS handles, so a preview can never create history
/// entries or kick off synthesis.
async fn preview_normalization(
    pipeline: Arc<Mutex<TTSPipeline>>,
    text: String,
) -> CmdResult<(String, CharMapping)> {
    let (normalized, mapping, _code_block_mode) = run_pipeline_normalization(pipeline, text)
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
        preview_normalization(Arc::clone(&state.pipeline), text).await?;
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

/// Persist the display format of an entry and notify the UI.
///
/// Display-only: `normalized_text`, audio, and timestamps are untouched, so
/// no `Processing` guard is needed — the change cannot race the synthesis
/// pipeline.
#[tauri::command]
pub async fn set_entry_format<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
    format: TextFormat,
) -> CmdResult<()> {
    let mut entry = require_entry(&state.storage, &id)?;
    entry.format = Some(format);
    state
        .storage
        .update_entry(entry.clone())
        .map_err(CommandError::from)?;
    emit_entry_updated(&app, &entry);
    Ok(())
}

/// Regenerate audio for an existing entry: drop its current audio + timestamps,
/// reset status to `Pending`, and re-run the synthesis pipeline. Useful when
/// the user has changed `speaker`, `speech_rate`, or other normalization
/// settings and wants the cached audio to reflect them. With `play_when_ready`,
/// the fresh audio autoplays on success (same rule as the initial synthesis).
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

    // If this entry is currently playing, stop playback so the about-to-be-
    // deleted audio file is not held open by the player.
    if state.player.current_entry_id().as_deref() == Some(&id) {
        let _ = state.player.stop();
    }

    state
        .storage
        .delete_audio(&uuid)
        .map_err(CommandError::from)?;

    let mut entry = state.storage.get_entry(&uuid).ok_or_else(|| {
        CommandError::not_found("entry.not_found", vec![id.to_string()])
            .with_message("entry vanished after delete_audio")
    })?;
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
/// synthesis task (if registered) and flip the entry back to `pending`.
/// Returns the updated entry plus whether the task had entered the TTS
/// stage — the caller kills ttsd only in that case.
///
/// Only a `processing` or `pending` entry may be cancelled: the spec
/// sanctions just the `processing → pending` transition, and silently
/// regressing a `ready` / `error` entry to `pending` would orphan its
/// audio from the state machine (playback requires `ready`). `pending`
/// is allowed: cancellation is idempotent for a queued/idle entry, and a
/// just-added entry briefly sits in `pending` with its synthesis task
/// already registered — cancelling must still abort it. `ready`,
/// `playing` and `error` fail with `synthesis_error` and change nothing.
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
        EntryStatus::Ready | EntryStatus::Error | EntryStatus::Playing
    ) {
        return Err(CommandError::synthesis(
            "entry.cannot_cancel",
            vec![id.to_string(), entry_status_str(live_status).to_string()],
        ));
    }

    // Atomic transition: flip to `pending` only if the entry is still
    // `processing`/`pending` at apply time. A concurrent synthesis completion
    // that already moved it out of these states wins, and the stale `pending`
    // clone is NOT persisted.
    let applied = storage.update_entry_if(
        &uuid,
        |e| matches!(e.status, EntryStatus::Processing | EntryStatus::Pending),
        |e| e.status = EntryStatus::Pending,
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

    let entry = storage
        .get_entry(&uuid)
        .ok_or_else(|| CommandError::not_found("entry.not_found", vec![id.to_string()]))?;
    Ok((entry, entered_tts))
}

/// Cancel an in-progress or queued synthesis job: abort the entry's
/// synthesis task and flip the entry back to `pending`. If the task had
/// already entered the TTS stage, the current ttsd subprocess is killed too
/// (the supervisor transparently respawns it on the next request). A late
/// completion belonging to the cancelled entry is discarded by the
/// stale-completion guard in `apply_ready_if_current` /
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
    let bundle_dir = state.silero_native_bundle_dir.clone();
    tokio::task::spawn_blocking(move || availability::probe(&ttsd_dir, &bundle_dir))
        .await
        .map_err(|e| {
            CommandError::internal("engines.probe_panicked", vec![]).with_message(e.to_string())
        })
}

/// Download the Silero Native model bundle on user demand. Idempotent —
/// files already present with a matching checksum are skipped. Progress is
/// delivered via the `bundle_download_started` / `bundle_download_progress`
/// / `bundle_download_finished` events; the `Result` here only reports the
/// final outcome so the frontend can show one final notification.
#[tauri::command]
pub async fn download_silero_native_bundle(state: State<'_, AppState>) -> CmdResult<()> {
    let bundle_dir = state.silero_native_bundle_dir.clone();
    let emitter = Arc::clone(&state.emitter);
    crate::tts::silero_native::download::download_bundle(&bundle_dir, &emitter)
        .await
        .map_err(CommandError::from)
}

/// Apply the persisted `code_block_mode` to the shared pipeline. Called at
/// startup so the pipeline never narrates code blocks in a mode the config
/// did not ask for; `update_config` pushes later changes itself.
pub fn apply_configured_code_block_mode(pipeline: &Arc<Mutex<TTSPipeline>>, config: &UIConfig) {
    let mode = crate::pipeline::normalizers::code_blocks::CodeBlockMode::from_config(
        &config.code_block_mode,
    );
    pipeline.lock().set_code_block_mode(mode);
}

/// Merge a partial config patch into the current configuration, swap the
/// active TTS engine if needed, and persist. The engine swap runs *before*
/// the config is saved — if the user picked a Silero stack we cannot spawn,
/// the call returns an error and the previous config stays on disk.
///
/// A `code_block_mode` change is pushed into the shared pipeline after the
/// save, so subsequent synthesis/preview runs pick it up without a restart
/// (synthesis already in flight finishes on the previous mode).
#[tauri::command]
pub async fn update_config(state: State<'_, AppState>, patch: UIConfigPatch) -> CmdResult<()> {
    let mut config = state.storage.load_config().unwrap_or_default();
    let code_block_mode_patch = patch.code_block_mode.clone();
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

    if code_block_mode_patch.is_some() {
        let mode = crate::pipeline::normalizers::code_blocks::CodeBlockMode::from_config(
            &config.code_block_mode,
        );
        state.pipeline.lock().set_code_block_mode(mode);
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
    if let Some(v) = patch.language {
        config.language = v;
    }
}

#[cfg(test)]
mod image_url_tests {
    use super::CommandError;
    use super::{MAX_IMAGE_BYTES, ensure_image_size, media_type, validate_image_url};

    #[test]
    fn accepts_absolute_http_and_https_urls() {
        for url in ["https://example.com/a.png", "http://example.com/a.png"] {
            let parsed = validate_image_url(url).unwrap();
            assert_eq!(parsed.as_str(), url);
        }
    }

    #[test]
    fn rejects_non_http_schemes() {
        for url in [
            "file:///etc/passwd",
            "data:image/png;base64,AAAA",
            "ftp://example.com/a.png",
        ] {
            let err = validate_image_url(url).unwrap_err();
            match err {
                CommandError::Internal { code, .. } => {
                    assert_eq!(code, "image.url_scheme_unsupported")
                }
                other => panic!("{url}: expected Internal, got {other:?}"),
            }
        }
    }

    #[test]
    fn rejects_unparseable_urls() {
        assert!(validate_image_url("not a url").is_err());
    }

    #[test]
    fn size_cap_admits_the_boundary_and_rejects_one_byte_over() {
        assert!(ensure_image_size(MAX_IMAGE_BYTES as u64).is_ok());
        assert!(ensure_image_size(MAX_IMAGE_BYTES as u64 + 1).is_err());
    }

    #[test]
    fn media_type_strips_parameters_and_lowercases() {
        let value = reqwest::header::HeaderValue::from_static("Image/PNG; charset=utf-8");
        assert_eq!(media_type(Some(&value)).as_deref(), Some("image/png"),);
    }

    #[test]
    fn media_type_is_none_for_missing_empty_or_invalid_headers() {
        assert_eq!(media_type(None), None);
        let empty = reqwest::header::HeaderValue::from_static("");
        assert_eq!(media_type(Some(&empty)), None);
        let invalid = reqwest::header::HeaderValue::from_bytes(&[0xff, 0xfe]).unwrap();
        assert_eq!(media_type(Some(&invalid)), None);
    }
}

#[cfg(test)]
mod updater_tests {
    use super::updater_supported_with;

    // `cargo test` runs on Linux CI (the windows-build job is compile-only),
    // so the Windows branch is pinned by a cfg-gated test for Windows
    // runners and by compilation everywhere else.
    #[test]
    #[cfg(target_os = "linux")]
    fn appimage_env_presence_decides_on_linux() {
        assert!(updater_supported_with(Some(std::ffi::OsString::from(
            "/opt/RuVox.AppImage"
        ))));
        assert!(!updater_supported_with(None));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn updater_is_always_supported_on_windows() {
        assert!(updater_supported_with(None));
        assert!(updater_supported_with(Some(std::ffi::OsString::from(
            "C:\\apps\\RuVox.exe"
        ))));
    }
}

#[cfg(test)]
mod synthesis_tests {
    use super::*;
    use crate::storage::test_util::make_service;

    #[tokio::test]
    async fn run_normalization_returns_normalized_text_and_mapping() {
        let pipeline = Arc::new(Mutex::new(TTSPipeline::new()));
        let (normalized, _mapping, _mode) = run_normalization(pipeline, "Привет мир".to_string())
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
        let (storage, _dir) = make_service();
        let entry = storage.add_entry("text".to_string()).unwrap();
        let id = entry.id;

        // The encoder requires a valid RIFF header; bogus bytes force the
        // best-effort path that keeps the .wav file as audio_filename.
        let wav_filename = format!("{id}.wav");
        let wav_path = storage.data_dir().join("audio").join(&wav_filename);
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

    // ── Stale-completion guard ───────────────────────────────────────────

    /// The guard decision is pure: only `processing` lets a late result
    /// through; every other status discards it.
    #[test]
    fn completion_guard_only_allows_processing() {
        assert!(completion_is_current(EntryStatus::Processing));
        for status in [
            EntryStatus::Pending,
            EntryStatus::Ready,
            EntryStatus::Playing,
            EntryStatus::Error,
        ] {
            assert!(!completion_is_current(status), "{status:?} must be stale");
        }
    }

    fn fake_output() -> SynthesizeOutput {
        SynthesizeOutput {
            timestamps: Vec::new(),
            duration_sec: 1.0,
        }
    }

    fn fake_generation() -> GenerationParams {
        GenerationParams {
            engine: "silero_native".to_string(),
            voice: "xenia".to_string(),
            sample_rate: Some(24000),
            model: None,
            app_version: "test".to_string(),
            code_block_mode: Some("read".to_string()),
            normalized_text_sha256: None,
            audio_codec: Some("Ogg Opus".to_string()),
            audio_bytes: None,
        }
    }

    fn set_status(storage: &StorageService, entry: &TextEntry, status: EntryStatus) {
        let mut updated = entry.clone();
        updated.status = status;
        storage.update_entry(updated).unwrap();
    }

    /// A late completion for a non-`processing` entry changes no status and
    /// removes the audio/timestamp files it just wrote.
    #[test]
    fn stale_ready_completion_is_discarded_with_files() {
        let (storage, _dir) = make_service();
        let entry = storage.add_entry("текст".to_string()).unwrap(); // pending
        let id = entry.id;
        let audio_dir = storage.data_dir().join("audio");
        let audio_name = format!("{id}.opus");
        let ts_name = format!("{id}.timestamps.json");
        std::fs::write(audio_dir.join(&audio_name), b"opus").unwrap();
        std::fs::write(audio_dir.join(&ts_name), b"{}").unwrap();

        let applied = apply_ready_if_current(
            &storage,
            &id,
            &fake_output(),
            Some(ts_name.clone()),
            &audio_name,
            fake_generation(),
        );

        assert!(!applied);
        let stored = storage.get_entry(&id).unwrap();
        assert_eq!(stored.status, EntryStatus::Pending);
        assert!(stored.audio_path.is_none());
        assert!(stored.timestamps_path.is_none());
        assert!(
            stored.generation.is_none(),
            "stale completion must not resurrect a snapshot"
        );
        assert!(!audio_dir.join(&audio_name).exists());
        assert!(!audio_dir.join(&ts_name).exists());
    }

    /// The happy path: a completion arriving while the entry is `processing`
    /// applies and populates the ready fields.
    #[test]
    fn ready_completion_applies_while_processing() {
        let (storage, _dir) = make_service();
        let entry = storage.add_entry("текст".to_string()).unwrap();
        set_status(&storage, &entry, EntryStatus::Processing);
        let id = entry.id;

        let applied = apply_ready_if_current(
            &storage,
            &id,
            &fake_output(),
            Some(format!("{id}.timestamps.json")),
            &format!("{id}.opus"),
            fake_generation(),
        );

        assert!(applied);
        let stored = storage.get_entry(&id).unwrap();
        assert_eq!(stored.status, EntryStatus::Ready);
        assert_eq!(
            stored.audio_path.as_deref(),
            Some(format!("{id}.opus").as_str())
        );
        assert_eq!(stored.duration_sec, Some(1.0));
        assert!(stored.audio_generated_at.is_some());
        assert_eq!(stored.generation_count, 1);
        assert_eq!(
            stored.generation.as_ref().map(|g| g.voice.as_str()),
            Some("xenia")
        );
    }

    /// Every successful bake increments the count and replaces the snapshot:
    /// a re-synthesis with a different voice overwrites the old parameters.
    #[test]
    fn second_completion_bumps_generation_count_and_refreshes_snapshot() {
        let (storage, _dir) = make_service();
        let entry = storage.add_entry("текст".to_string()).unwrap();
        set_status(&storage, &entry, EntryStatus::Processing);
        let id = entry.id;

        assert!(apply_ready_if_current(
            &storage,
            &id,
            &fake_output(),
            None,
            &format!("{id}.opus"),
            fake_generation(),
        ));

        let mut regenerated = fake_generation();
        regenerated.voice = "baya".to_string();
        regenerated.model = Some(ModelParams {
            name: "ru_RU-baya-medium.onnx".to_string(),
            sha256: None,
        });
        set_status(
            &storage,
            &storage.get_entry(&id).unwrap(),
            EntryStatus::Processing,
        );

        assert!(apply_ready_if_current(
            &storage,
            &id,
            &fake_output(),
            None,
            &format!("{id}.opus"),
            regenerated,
        ));

        let stored = storage.get_entry(&id).unwrap();
        assert_eq!(stored.generation_count, 2);
        let generation = stored.generation.expect("snapshot refreshed");
        assert_eq!(generation.voice, "baya");
        assert_eq!(
            generation.model.expect("model set").name,
            "ru_RU-baya-medium.onnx"
        );
    }

    /// A late TTS-stage failure for a non-`processing` entry changes no
    /// status and removes the candidate files a dying ttsd may have written.
    #[test]
    fn stale_failure_is_discarded_with_candidate_files() {
        let (storage, _dir) = make_service();
        let entry = storage.add_entry("текст".to_string()).unwrap(); // pending
        let id = entry.id;
        let wav = storage.data_dir().join("audio").join(format!("{id}.wav"));
        std::fs::write(&wav, b"partial").unwrap();

        let applied = apply_error_if_current(&storage, &id, "ttsd died", true);

        assert!(!applied);
        let stored = storage.get_entry(&id).unwrap();
        assert_eq!(stored.status, EntryStatus::Pending);
        assert!(stored.error_message.is_none());
        assert!(!wav.exists());
    }

    /// A TTS-stage failure while the entry is `processing` applies normally.
    #[test]
    fn tts_failure_applies_while_processing() {
        let (storage, _dir) = make_service();
        let entry = storage.add_entry("текст".to_string()).unwrap();
        set_status(&storage, &entry, EntryStatus::Processing);

        let applied = apply_error_if_current(&storage, &entry.id, "ttsd died", true);

        assert!(applied);
        let stored = storage.get_entry(&entry.id).unwrap();
        assert_eq!(stored.status, EntryStatus::Error);
        assert_eq!(stored.error_message.as_deref(), Some("ttsd died"));
    }

    /// Normalization-stage failures arrive while the entry is legitimately
    /// still `pending` — they must not be discarded by the guard.
    #[test]
    fn normalization_failure_applies_from_pending() {
        let (storage, _dir) = make_service();
        let entry = storage.add_entry("текст".to_string()).unwrap(); // pending

        let applied = apply_error_if_current(&storage, &entry.id, "empty", false);

        assert!(applied);
        let stored = storage.get_entry(&entry.id).unwrap();
        assert_eq!(stored.status, EntryStatus::Error);
        assert_eq!(stored.error_message.as_deref(), Some("empty"));
    }

    // ── cancel_entry ───────────────────────────────────────────────────────

    /// Cancelling an unknown id fails with `not_found` and touches nothing.
    #[test]
    fn cancel_entry_unknown_id_is_not_found() {
        let (storage, _dir) = make_service();
        let tasks = Mutex::new(HashMap::new());
        let entered = Mutex::new(HashSet::new());
        let id = uuid::Uuid::new_v4().to_string();

        let err = cancel_entry(&storage, &tasks, &entered, &id).unwrap_err();
        match err {
            CommandError::NotFound { code, params, .. } => {
                assert_eq!(code, "entry.not_found");
                assert_eq!(params, vec![id]);
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    /// Cancelling a `ready` or `error` entry fails with `synthesis_error`
    /// and changes nothing: the stored status is untouched and the
    /// registries keep their keys. (`playing` can't be exercised here —
    /// storage normalizes it to `ready` on save; the guard covers it the
    /// same way.)
    #[tokio::test]
    async fn cancel_entry_rejects_terminal_statuses() {
        for status in [EntryStatus::Ready, EntryStatus::Error] {
            let (storage, _dir) = make_service();
            let entry = storage.add_entry("текст".to_string()).unwrap();
            set_status(&storage, &entry, status);
            let id = entry.id;

            let tasks: Mutex<HashMap<EntryId, AbortHandle>> = Mutex::new(HashMap::new());
            let entered: Mutex<HashSet<EntryId>> = Mutex::new(HashSet::new());
            let sleeper = tokio::spawn(async {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            });
            tasks.lock().insert(id, sleeper.abort_handle());
            entered.lock().insert(id);

            let err = cancel_entry(&storage, &tasks, &entered, &id.to_string()).unwrap_err();
            match err {
                CommandError::SynthesisError { code, params, .. } => {
                    assert_eq!(code, "entry.cannot_cancel", "{status:?}");
                    assert_eq!(params[0], id.to_string(), "{status:?}: {params:?}");
                }
                other => panic!("{status:?}: expected SynthesisError, got {other:?}"),
            }
            assert_eq!(storage.get_entry(&id).unwrap().status, status);
            assert!(tasks.lock().contains_key(&id));
            assert!(entered.lock().contains(&id));

            sleeper.abort();
        }
    }

    /// Cancelling a `pending` entry is allowed and idempotent: a just-added
    /// entry briefly sits in `pending` with its synthesis task already
    /// registered, and cancelling must still abort that task.
    #[tokio::test]
    async fn cancel_entry_pending_entry_aborts_registered_task() {
        let (storage, _dir) = make_service();
        let entry = storage.add_entry("текст".to_string()).unwrap(); // pending
        let id = entry.id;

        let tasks: Mutex<HashMap<EntryId, AbortHandle>> = Mutex::new(HashMap::new());
        let entered: Mutex<HashSet<EntryId>> = Mutex::new(HashSet::new());
        let sleeper = tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        });
        tasks.lock().insert(id, sleeper.abort_handle());

        let (updated, entered_tts) =
            cancel_entry(&storage, &tasks, &entered, &id.to_string()).unwrap();

        assert_eq!(updated.status, EntryStatus::Pending);
        assert!(!entered_tts);
        assert!(tasks.lock().is_empty());
        let join_err = sleeper.await.unwrap_err();
        assert!(join_err.is_cancelled());
    }

    /// Cancel flips the entry to `pending`, removes both registry keys, and
    /// the registered task is actually aborted.
    #[tokio::test]
    async fn cancel_entry_aborts_registered_task_and_sets_pending() {
        let (storage, _dir) = make_service();
        let entry = storage.add_entry("текст".to_string()).unwrap();
        set_status(&storage, &entry, EntryStatus::Processing);
        let id = entry.id;

        let tasks: Mutex<HashMap<EntryId, AbortHandle>> = Mutex::new(HashMap::new());
        let entered: Mutex<HashSet<EntryId>> = Mutex::new(HashSet::new());
        let sleeper = tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        });
        tasks.lock().insert(id, sleeper.abort_handle());
        entered.lock().insert(id);

        let (updated, entered_tts) =
            cancel_entry(&storage, &tasks, &entered, &id.to_string()).unwrap();

        assert_eq!(updated.status, EntryStatus::Pending);
        assert!(entered_tts, "entry was marked as inside the TTS stage");
        assert!(tasks.lock().is_empty());
        assert!(entered.lock().is_empty());
        assert_eq!(storage.get_entry(&id).unwrap().status, EntryStatus::Pending);

        let join_err = sleeper.await.unwrap_err();
        assert!(join_err.is_cancelled());
    }

    /// An entry that never reached the TTS stage reports `entered_tts =
    /// false`, so the caller does not kill ttsd.
    #[tokio::test]
    async fn cancel_entry_without_tts_stage_reports_false() {
        let (storage, _dir) = make_service();
        let entry = storage.add_entry("текст".to_string()).unwrap();
        set_status(&storage, &entry, EntryStatus::Processing);
        let id = entry.id;

        let tasks: Mutex<HashMap<EntryId, AbortHandle>> = Mutex::new(HashMap::new());
        let entered: Mutex<HashSet<EntryId>> = Mutex::new(HashSet::new());
        let sleeper = tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        });
        tasks.lock().insert(id, sleeper.abort_handle());

        let (_updated, entered_tts) =
            cancel_entry(&storage, &tasks, &entered, &id.to_string()).unwrap();

        assert!(!entered_tts);
        let join_err = sleeper.await.unwrap_err();
        assert!(join_err.is_cancelled());
    }

    /// Registry cleanup removes only finished handles: a live handle (a
    /// newer task spawned for the same entry) must survive, otherwise the
    /// newer task would become uncancellable.
    #[tokio::test]
    async fn cleanup_finished_handle_removes_only_finished_handles() {
        let tasks: Mutex<HashMap<EntryId, AbortHandle>> = Mutex::new(HashMap::new());
        let id = uuid::Uuid::new_v4();

        // Finished handle (the completed task's own) → removed.
        let done = tokio::spawn(async {});
        let done_abort = done.abort_handle();
        done.await.unwrap();
        tasks.lock().insert(id, done_abort);
        cleanup_finished_handle(&tasks, &id);
        assert!(tasks.lock().is_empty());

        // Live handle (a newer task for the same entry) → kept.
        let live = tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        });
        tasks.lock().insert(id, live.abort_handle());
        cleanup_finished_handle(&tasks, &id);
        assert!(tasks.lock().contains_key(&id));
        live.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_util::make_service;
    use std::collections::HashMap;
    use std::path::Path;
    use test_case::test_case;

    // ── apply_config_patch ──────────────────────────────────────────────────

    #[test]
    fn apply_configured_code_block_mode_maps_config_values() {
        use crate::pipeline::normalizers::code_blocks::CodeBlockMode;

        let pipeline = Arc::new(Mutex::new(TTSPipeline::new()));
        let config = UIConfig {
            code_block_mode: "read".to_string(),
            ..UIConfig::default()
        };
        apply_configured_code_block_mode(&pipeline, &config);
        assert_eq!(pipeline.lock().code_block_mode(), CodeBlockMode::Full);

        // The legacy alias and unknown values resolve to the default mode.
        let config = UIConfig {
            code_block_mode: "skip".to_string(),
            ..UIConfig::default()
        };
        apply_configured_code_block_mode(&pipeline, &config);
        assert_eq!(pipeline.lock().code_block_mode(), CodeBlockMode::Brief);
    }

    /// Every `UIConfigPatch` field, in isolation: set to a value distinct
    /// from `UIConfig::default()` and confirm it lands, and nothing else
    /// changes as a side effect. Table-driven so adding a 16th field later
    /// only needs a new entry, not a new copy-pasted test function.
    #[test]
    fn apply_config_patch_applies_each_field_in_isolation() {
        // Compile-time guard: exhaustive destructuring (no `..`) breaks the
        // build when a field is added to `UIConfigPatch`, forcing the table
        // below (and `apply_config_patch` itself) to be extended in step.
        let UIConfigPatch {
            speaker: _,
            sample_rate: _,
            speech_rate: _,
            notify_on_ready: _,
            notify_on_error: _,
            text_format: _,
            max_cache_size_mb: _,
            code_block_mode: _,
            theme: _,
            player_hotkeys: _,
            window_geometry: _,
            preview_dialog_enabled: _,
            engine: _,
            piper_voice: _,
            language: _,
        } = UIConfigPatch::default();

        let mut custom_hotkeys = HashMap::new();
        custom_hotkeys.insert("play_pause".to_string(), "Enter".to_string());

        struct Case {
            field: &'static str,
            patch: UIConfigPatch,
        }

        let cases = vec![
            Case {
                field: "speaker",
                patch: UIConfigPatch {
                    speaker: Some("helga".to_string()),
                    ..Default::default()
                },
            },
            Case {
                field: "sample_rate",
                patch: UIConfigPatch {
                    sample_rate: Some(16000),
                    ..Default::default()
                },
            },
            Case {
                field: "speech_rate",
                patch: UIConfigPatch {
                    speech_rate: Some(1.5),
                    ..Default::default()
                },
            },
            Case {
                field: "notify_on_ready",
                patch: UIConfigPatch {
                    notify_on_ready: Some(false),
                    ..Default::default()
                },
            },
            Case {
                field: "notify_on_error",
                patch: UIConfigPatch {
                    notify_on_error: Some(false),
                    ..Default::default()
                },
            },
            Case {
                field: "text_format",
                patch: UIConfigPatch {
                    text_format: Some("markdown".to_string()),
                    ..Default::default()
                },
            },
            Case {
                field: "max_cache_size_mb",
                patch: UIConfigPatch {
                    max_cache_size_mb: Some(1000),
                    ..Default::default()
                },
            },
            Case {
                field: "code_block_mode",
                patch: UIConfigPatch {
                    code_block_mode: Some("read".to_string()),
                    ..Default::default()
                },
            },
            Case {
                field: "theme",
                patch: UIConfigPatch {
                    theme: Some("dark".to_string()),
                    ..Default::default()
                },
            },
            Case {
                field: "player_hotkeys",
                patch: UIConfigPatch {
                    player_hotkeys: Some(custom_hotkeys.clone()),
                    ..Default::default()
                },
            },
            Case {
                field: "preview_dialog_enabled",
                patch: UIConfigPatch {
                    preview_dialog_enabled: Some(false),
                    ..Default::default()
                },
            },
            Case {
                field: "engine",
                patch: UIConfigPatch {
                    engine: Some("silero".to_string()),
                    ..Default::default()
                },
            },
            Case {
                field: "piper_voice",
                patch: UIConfigPatch {
                    piper_voice: Some("irina".to_string()),
                    ..Default::default()
                },
            },
        ];

        for case in cases {
            let mut config = UIConfig::default();
            let before = serde_json::to_value(&config).unwrap();

            apply_config_patch(&mut config, case.patch);

            let after = serde_json::to_value(&config).unwrap();
            let before_obj = before.as_object().unwrap();
            let after_obj = after.as_object().unwrap();

            let changed: Vec<&String> = before_obj
                .keys()
                .filter(|k| before_obj[*k] != after_obj[*k])
                .collect();

            assert_eq!(
                changed,
                vec![case.field],
                "patching {} should change exactly that field",
                case.field
            );
        }
    }

    /// `window_geometry` is `Option<Option<[i32; 4]>>` in the patch: the outer
    /// `Option` says whether the patch touches the field at all, the inner
    /// one says whether the new value is "set" or "cleared". Distinct enough
    /// from the other fields to warrant its own cases rather than a table row.
    #[test]
    fn apply_config_patch_window_geometry_set_and_clear() {
        // Absent from the patch (outer None) -> old value untouched.
        let mut config = UIConfig {
            window_geometry: Some([1, 2, 3, 4]),
            ..UIConfig::default()
        };
        apply_config_patch(&mut config, UIConfigPatch::default());
        assert_eq!(config.window_geometry, Some([1, 2, 3, 4]));

        // Patch sets a new geometry (outer Some, inner Some).
        let mut config = UIConfig::default();
        assert_eq!(config.window_geometry, None);
        apply_config_patch(
            &mut config,
            UIConfigPatch {
                window_geometry: Some(Some([10, 20, 30, 40])),
                ..Default::default()
            },
        );
        assert_eq!(config.window_geometry, Some([10, 20, 30, 40]));

        // Patch explicitly clears geometry (outer Some, inner None).
        let mut config = UIConfig {
            window_geometry: Some([5, 6, 7, 8]),
            ..UIConfig::default()
        };
        apply_config_patch(
            &mut config,
            UIConfigPatch {
                window_geometry: Some(None),
                ..Default::default()
            },
        );
        assert_eq!(config.window_geometry, None);
    }

    /// An all-`None` patch (the wire format for "nothing changed") must leave
    /// every field exactly as it was, even when the config already diverges
    /// from `UIConfig::default()`.
    #[test]
    fn apply_config_patch_all_none_leaves_config_untouched() {
        let mut custom_hotkeys = HashMap::new();
        custom_hotkeys.insert("play_pause".to_string(), "Enter".to_string());

        let mut config = UIConfig {
            speaker: "helga".to_string(),
            sample_rate: 16000,
            speech_rate: 1.5,
            notify_on_ready: false,
            notify_on_error: false,
            text_format: "markdown".to_string(),
            max_cache_size_mb: 1000,
            code_block_mode: "skip".to_string(),
            theme: "dark".to_string(),
            player_hotkeys: custom_hotkeys,
            window_geometry: Some([1, 2, 3, 4]),
            preview_dialog_enabled: false,
            engine: "silero".to_string(),
            piper_voice: "irina".to_string(),
            language: "ru".to_string(),
        };
        let before = serde_json::to_value(&config).unwrap();

        apply_config_patch(&mut config, UIConfigPatch::default());

        let after = serde_json::to_value(&config).unwrap();
        assert_eq!(before, after);
    }

    // ── parse_entry_id ───────────────────────────────────────────────────────

    /// Assert that `err` is a `CommandError::NotFound` with the expected code.
    fn assert_not_found(err: CommandError, expected_code: &str) {
        match err {
            CommandError::NotFound { code, .. } => assert_eq!(code, expected_code),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    /// Well-formed UUID strings round-trip: `parse_entry_id` returns the same
    /// UUID `Uuid::parse_str` would. Covers the nil UUID and a v4-shaped value.
    #[test_case("00000000-0000-0000-0000-000000000000"; "nil")]
    #[test_case("550e8400-e29b-41d4-a716-446655440000"; "v4")]
    fn parse_entry_id_accepts_valid(input: &str) {
        let parsed = parse_entry_id(input).unwrap();
        assert_eq!(parsed, uuid::Uuid::parse_str(input).unwrap());
    }

    /// Malformed ids are rejected as `CommandError::NotFound` with code
    /// `entry.id_invalid` (the input travels in `params`, the parse error
    /// detail in `message`). Trailing whitespace is significant: the parser
    /// does not trim.
    #[test_case(""; "empty")]
    #[test_case("not-a-uuid"; "garbage")]
    #[test_case("00000000-0000-0000-0000-000000000000 "; "trailing_whitespace")]
    fn parse_entry_id_rejects(input: &str) {
        assert_not_found(parse_entry_id(input).unwrap_err(), "entry.id_invalid");
    }

    // ── char_mapping_to_entries ──────────────────────────────────────────────

    #[test]
    fn char_mapping_to_entries_normal_case_preserves_order() {
        // Deliberately out-of-order orig ranges: the function must emit
        // entries in char_map iteration order (by norm index), not sorted by
        // orig_start.
        let mapping = CharMapping {
            original: "orig".to_string(),
            transformed: "abc".to_string(),
            char_map: vec![(5, 6), (0, 1), (3, 4)],
        };

        let entries = char_mapping_to_entries(&mapping);
        assert_eq!(entries.len(), 3);

        assert_eq!(entries[0].norm_start, 0);
        assert_eq!(entries[0].norm_end, 1);
        assert_eq!(entries[0].orig_start, 5);
        assert_eq!(entries[0].orig_end, 6);

        assert_eq!(entries[1].norm_start, 1);
        assert_eq!(entries[1].norm_end, 2);
        assert_eq!(entries[1].orig_start, 0);
        assert_eq!(entries[1].orig_end, 1);

        assert_eq!(entries[2].norm_start, 2);
        assert_eq!(entries[2].norm_end, 3);
        assert_eq!(entries[2].orig_start, 3);
        assert_eq!(entries[2].orig_end, 4);
    }

    #[test]
    fn char_mapping_to_entries_empty_char_map_yields_empty_entries() {
        let mapping = CharMapping {
            original: String::new(),
            transformed: String::new(),
            char_map: vec![],
        };
        assert!(char_mapping_to_entries(&mapping).is_empty());
    }

    // ── CommandError::from conversions ────────────────────────────────────────

    /// `From<StorageError>`: `NotFound` maps to the `not_found` type with the
    /// entry id in `params`, every other variant to `storage_error` with the
    /// source error's `Display` as the raw fallback `message`. Empty params
    /// and absent detail are omitted from the wire format.
    #[test_case(
        StorageError::NotFound(uuid::Uuid::nil()),
        json!({
            "type": "not_found",
            "code": "entry.not_found",
            "params": ["00000000-0000-0000-0000-000000000000"],
        });
        "not_found_carries_id"
    )]
    #[test_case(
        StorageError::NoDataDir,
        json!({
            "type": "storage_error",
            "code": "storage.failure",
            "message": "per-user data dir unavailable (dirs resolution returned None)",
        });
        "other_variant"
    )]
    fn command_error_from_storage(source: StorageError, expected: serde_json::Value) {
        let err: CommandError = source.into();
        assert_eq!(serde_json::to_value(&err).unwrap(), expected);
    }

    /// `From<TtsError>`: every variant maps to `synthesis_error` with code
    /// `synthesis.failed` and the source error's `Display` as the raw
    /// fallback `message`.
    #[test_case(TtsError::Died, "ttsd subprocess has exited"; "died")]
    #[test_case(
        TtsError::Ttsd {
            code: "voice_not_installed".to_string(),
            message: "voice xenia missing".to_string(),
        },
        "ttsd error [voice_not_installed]: voice xenia missing";
        "ttsd_preserves_code_and_message"
    )]
    fn command_error_from_tts(source: TtsError, expected_message: &str) {
        let err: CommandError = source.into();
        let value = serde_json::to_value(&err).unwrap();
        assert_eq!(
            value,
            json!({
                "type": "synthesis_error",
                "code": "synthesis.failed",
                "message": expected_message,
            })
        );
    }

    // ── serde: CleanupMode / ClearCacheArgs ───────────────────────────────────

    /// Accepted `CleanupMode` payloads deserialize to the right variant:
    /// `size_limit` carries `target_mb` (mapped to `Some`), `all` has no
    /// payload (mapped to `None`).
    #[test_case(json!({ "mode": "size_limit", "target_mb": 250 }) => Some(250); "size_limit")]
    #[test_case(json!({ "mode": "all" }) => None; "all")]
    fn cleanup_mode_deserializes(value: serde_json::Value) -> Option<u32> {
        match serde_json::from_value::<CleanupMode>(value).unwrap() {
            CleanupMode::SizeLimit { target_mb } => Some(target_mb),
            CleanupMode::All => None,
        }
    }

    /// Malformed `mode` tags are rejected: the Rust variant name `SizeLimit`
    /// (the tag is `rename_all = "snake_case"`) and any unknown tag are both
    /// invalid on the wire.
    #[test_case(json!({ "mode": "SizeLimit", "target_mb": 250 }); "pascal_case_variant_name")]
    #[test_case(json!({ "mode": "bogus" }); "unknown_tag")]
    fn cleanup_mode_rejects(value: serde_json::Value) {
        assert!(serde_json::from_value::<CleanupMode>(value).is_err());
    }

    #[test]
    fn clear_cache_args_delete_texts_defaults_to_false_when_omitted() {
        let args: ClearCacheArgs =
            serde_json::from_value(json!({ "mode": { "mode": "all" } })).unwrap();
        assert!(!args.delete_texts);
        assert!(matches!(args.mode, CleanupMode::All));
    }

    #[test]
    fn clear_cache_args_round_trips_explicit_delete_texts() {
        let args: ClearCacheArgs = serde_json::from_value(json!({
            "mode": { "mode": "size_limit", "target_mb": 42 },
            "delete_texts": true,
        }))
        .unwrap();
        assert!(args.delete_texts);
        match args.mode {
            CleanupMode::SizeLimit { target_mb } => assert_eq!(target_mb, 42),
            CleanupMode::All => panic!("expected SizeLimit"),
        }
    }

    // ── preview_normalize ────────────────────────────────────────────────────

    /// The preview returns the normalized text together with a char mapping
    /// whose `transformed` matches the returned text and whose `char_map`
    /// covers every codepoint of it (the `CharMapping` invariant).
    #[tokio::test]
    async fn preview_normalization_returns_normalized_text_and_mapping() {
        let pipeline = Arc::new(Mutex::new(TTSPipeline::new()));
        let input = "Привет, мир 42!".to_string();

        let (normalized, mapping) = preview_normalization(pipeline, input.clone())
            .await
            .unwrap();

        assert!(!normalized.is_empty());
        assert_eq!(mapping.original, input);
        assert_eq!(mapping.transformed, normalized);
        assert_eq!(mapping.char_map.len(), normalized.chars().count());
    }

    /// Unlike `run_normalization` (which flags empty input as
    /// `SynthesisError::EmptyText`), the preview path must not fail on empty
    /// text — the dialog shows whatever the pipeline produced.
    #[tokio::test]
    async fn preview_normalization_allows_empty_text() {
        let pipeline = Arc::new(Mutex::new(TTSPipeline::new()));
        let (normalized, _mapping) = preview_normalization(pipeline, String::new())
            .await
            .unwrap();
        assert!(normalized.is_empty());
    }

    // ── get_entry ────────────────────────────────────────────────────────────

    /// A well-formed UUID that is not in the history resolves to `Ok(None)`
    /// (serialized as `null` on the wire), not to an error.
    #[test]
    fn lookup_entry_returns_none_for_unknown_valid_id() {
        let (storage, _dir) = make_service();
        let id = uuid::Uuid::new_v4().to_string();

        let result = lookup_entry(&storage, &id).unwrap();
        assert!(result.is_none());
    }

    /// Sanity counterpart to the miss case: an existing id round-trips to the
    /// stored entry.
    #[test]
    fn lookup_entry_returns_entry_for_known_id() {
        let (storage, _dir) = make_service();
        let entry = storage.add_entry("привет".to_string()).unwrap();

        let found = lookup_entry(&storage, &entry.id.to_string())
            .unwrap()
            .expect("entry just added must be found");
        assert_eq!(found.id, entry.id);
        assert_eq!(found.original_text, "привет");
    }

    // ── get_timestamps ───────────────────────────────────────────────────────

    /// An entry that exists but has no timestamps file on disk yields the
    /// documented empty result: `load_timestamps` returns `None`, which the
    /// command maps to an empty vector via `unwrap_or_default`.
    #[test]
    fn load_timestamps_for_entry_returns_empty_vec_when_no_timestamps_file() {
        let (storage, _dir) = make_service();
        let entry = storage.add_entry("текст без аудио".to_string()).unwrap();

        let timestamps = load_timestamps_for_entry(&storage, &entry.id.to_string()).unwrap();
        assert!(timestamps.is_empty());
    }

    /// An unknown (but well-formed) id is rejected as `not_found` before any
    /// file lookup happens.
    #[test]
    fn load_timestamps_for_entry_errors_for_unknown_id() {
        let (storage, _dir) = make_service();
        let id = uuid::Uuid::new_v4().to_string();

        let err = load_timestamps_for_entry(&storage, &id).unwrap_err();
        assert_not_found(err, "entry.not_found");
    }

    // ── synthesize_audio: voice selection and retry gate follow the active engine ──

    use async_trait::async_trait;
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicBool, Ordering};

    use crate::tts::supervisor::test_helpers::recording_emitter;
    use tempfile::TempDir;

    /// Fake engine that records every `voice` argument it is called with.
    /// `kind` reports whichever engine the test wants to simulate as active
    /// (mimicking the `EngineSwitcher` after a startup fallback);
    /// `fail_first_call` makes the first `synthesize` fail the way a Piper
    /// engine with missing voice files does.
    struct RecordingEngine {
        kind: EngineKind,
        voices: std::sync::Mutex<Vec<String>>,
        fail_first_call: AtomicBool,
    }

    impl RecordingEngine {
        fn new(kind: EngineKind) -> Self {
            Self {
                kind,
                voices: std::sync::Mutex::new(Vec::new()),
                fail_first_call: AtomicBool::new(false),
            }
        }

        fn failing_first_call(kind: EngineKind) -> Self {
            let engine = Self::new(kind);
            engine.fail_first_call.store(true, Ordering::SeqCst);
            engine
        }

        fn recorded_voices(&self) -> Vec<String> {
            self.voices.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl TtsEngine for RecordingEngine {
        fn kind(&self) -> EngineKind {
            self.kind
        }

        async fn warmup(&self) -> Result<(), TtsError> {
            Ok(())
        }

        async fn spawn_initial_warmup(&self) {}

        async fn synthesize(
            &self,
            _text: String,
            voice: String,
            _sample_rate: u32,
            _out_wav: String,
            _char_mapping: Option<Vec<CharMappingEntry>>,
        ) -> Result<SynthesizeOutput, TtsError> {
            self.voices.lock().unwrap().push(voice);
            if self.fail_first_call.swap(false, Ordering::SeqCst) {
                return Err(TtsError::Ttsd {
                    code: "voice_not_installed".to_string(),
                    message: "voice not installed".to_string(),
                });
            }
            Ok(SynthesizeOutput {
                timestamps: Vec::new(),
                duration_sec: 1.0,
            })
        }

        async fn shutdown(&self) -> Result<(), TtsError> {
            Ok(())
        }
    }

    fn empty_mapping() -> CharMapping {
        CharMapping {
            original: String::new(),
            transformed: String::new(),
            char_map: vec![],
        }
    }

    /// Persist the config a fresh-install session would have: the default
    /// `silero_native` engine plus per-engine voice ids.
    fn persist_config(storage: &StorageService, engine: &str) {
        storage
            .save_config(&UIConfig {
                engine: engine.to_string(),
                piper_voice: "ruslan".to_string(),
                speaker: "aidar".to_string(),
                ..UIConfig::default()
            })
            .unwrap();
    }

    /// A Piper fallback session (persisted config names `silero_native`, the
    /// bundle is missing, so the switcher serves Piper) must pass the Piper
    /// voice id to the engine, not the Silero speaker id.
    #[tokio::test]
    async fn synthesize_audio_on_piper_fallback_uses_piper_voice() {
        let (storage, _dir) = make_service();
        persist_config(&storage, "silero_native");
        let engine = RecordingEngine::new(EngineKind::Piper);
        let (emitter, _log) = recording_emitter();
        let voices_dir = TempDir::new().unwrap();
        let entered = Mutex::new(HashSet::new());

        let outcome = synthesize_audio(
            &engine,
            &storage,
            voices_dir.path(),
            &emitter,
            &entered,
            &uuid::Uuid::new_v4(),
            "текст".to_string(),
            &empty_mapping(),
        )
        .await
        .unwrap();

        assert_eq!(engine.recorded_voices(), vec!["ruslan"]);
        // The outcome must carry the resolved voice for the snapshot.
        assert_eq!(outcome.voice, "ruslan");
    }

    /// The reverse case must not coerce either: an active Silero Native
    /// engine gets the Silero `speaker` even when the persisted config names
    /// `piper`.
    #[tokio::test]
    async fn synthesize_audio_on_silero_native_uses_speaker_even_with_piper_config() {
        let (storage, _dir) = make_service();
        persist_config(&storage, "piper");
        let engine = RecordingEngine::new(EngineKind::SileroNative);
        let (emitter, _log) = recording_emitter();
        let voices_dir = TempDir::new().unwrap();
        let entered = Mutex::new(HashSet::new());

        let outcome = synthesize_audio(
            &engine,
            &storage,
            voices_dir.path(),
            &emitter,
            &entered,
            &uuid::Uuid::new_v4(),
            "текст".to_string(),
            &empty_mapping(),
        )
        .await
        .unwrap();

        assert_eq!(engine.recorded_voices(), vec!["aidar"]);
        assert_eq!(outcome.voice, "aidar");
    }

    /// A `voice_not_installed` failure on the active Piper engine enters the
    /// auto-download path even when the persisted config names a Silero
    /// engine. The voice files are pre-seeded in the temp voices dir, so
    /// `download_voice` runs fully offline (both files skipped) and the
    /// retry succeeds; the `voice_download_started` event and the second
    /// synthesize call pin that the gate keyed on the active engine.
    #[tokio::test]
    async fn synthesize_audio_on_piper_fallback_auto_downloads_missing_voice() {
        let (storage, _dir) = make_service();
        persist_config(&storage, "silero_native");
        let engine = RecordingEngine::failing_first_call(EngineKind::Piper);
        let (emitter, log) = recording_emitter();
        let voices_dir = TempDir::new().unwrap();
        let voice_dir = voices_dir.path().join("ruslan");
        std::fs::create_dir_all(&voice_dir).unwrap();
        std::fs::write(voice_dir.join("ru_RU-ruslan-medium.onnx.json"), b"{}").unwrap();
        std::fs::write(voice_dir.join("ru_RU-ruslan-medium.onnx"), b"onnx").unwrap();
        let entered = Mutex::new(HashSet::new());

        synthesize_audio(
            &engine,
            &storage,
            voices_dir.path(),
            &emitter,
            &entered,
            &uuid::Uuid::new_v4(),
            "текст".to_string(),
            &empty_mapping(),
        )
        .await
        .unwrap();

        assert_eq!(engine.recorded_voices(), vec!["ruslan", "ruslan"]);
        let log = log.lock().unwrap();
        assert!(
            log.iter().any(|(name, _)| name == "voice_download_started"),
            "expected a voice_download_started event, got {log:?}"
        );
    }

    /// When the auto-download itself fails, the error surfaces instead of a
    /// retry loop: the persisted config names a catalog-unknown Piper voice,
    /// so `download_voice` fails fast with `voice_unknown` (no network).
    #[tokio::test]
    async fn synthesize_audio_on_piper_fallback_surfaces_failed_voice_download() {
        let (storage, _dir) = make_service();
        storage
            .save_config(&UIConfig {
                engine: "silero_native".to_string(),
                piper_voice: "ghost".to_string(),
                speaker: "aidar".to_string(),
                ..UIConfig::default()
            })
            .unwrap();
        let engine = RecordingEngine::failing_first_call(EngineKind::Piper);
        let (emitter, _log) = recording_emitter();
        let voices_dir = TempDir::new().unwrap();
        let entered = Mutex::new(HashSet::new());

        let err = synthesize_audio(
            &engine,
            &storage,
            voices_dir.path(),
            &emitter,
            &entered,
            &uuid::Uuid::new_v4(),
            "текст".to_string(),
            &empty_mapping(),
        )
        .await
        .unwrap_err();

        assert!(
            matches!(err, SynthesisError::TtsFailed(_)),
            "expected TtsFailed, got {err:?}"
        );
        // One pre-download attempt, no retry after the failed download.
        assert_eq!(engine.recorded_voices(), vec!["ghost"]);
    }

    /// Seed a ready entry with a stored audio file: set `audio_path` and
    /// write `audio/<name>` under the storage data dir.
    fn seed_audio_entry(storage: &StorageService, audio_name: &str, contents: &[u8]) -> EntryId {
        let mut entry = storage.add_entry("текст".to_string()).unwrap();
        entry.audio_path = Some(audio_name.to_string());
        entry.status = EntryStatus::Ready;
        storage.update_entry(entry.clone()).unwrap();
        let path = storage.data_dir().join("audio").join(audio_name);
        std::fs::write(path, contents).unwrap();
        entry.id
    }

    #[test]
    fn export_audio_to_copies_bytes_and_keeps_cache_intact() {
        let (storage, _dir) = make_service();
        let contents = b"ogg opus payload";
        let id = seed_audio_entry(&storage, "audio.opus", contents);
        let target_dir = TempDir::new().unwrap();
        let target = target_dir.path().join("export.opus");

        super::export::export_audio_to(&storage, &id.to_string(), &target).unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), contents);
        // The cached original must stay in place.
        assert!(storage.data_dir().join("audio").join("audio.opus").exists());
    }

    #[test]
    fn export_audio_to_fails_with_no_audio_when_entry_has_no_file() {
        let (storage, _dir) = make_service();
        let entry = storage.add_entry("текст".to_string()).unwrap();

        let err = super::export::export_audio_to(
            &storage,
            &entry.id.to_string(),
            Path::new("/tmp/ruvox-export-should-not-exist.opus"),
        )
        .unwrap_err();

        assert_not_found(err, "export.no_audio");
    }

    #[test]
    fn export_audio_to_fails_with_no_audio_when_cache_file_was_evicted() {
        let (storage, _dir) = make_service();
        let id = seed_audio_entry(&storage, "audio.opus", b"payload");
        // Evict the cached file: the entry still references it.
        std::fs::remove_file(storage.data_dir().join("audio").join("audio.opus")).unwrap();
        let target_dir = TempDir::new().unwrap();

        let err = super::export::export_audio_to(
            &storage,
            &id.to_string(),
            &target_dir.path().join("export.opus"),
        )
        .unwrap_err();

        assert_not_found(err, "export.no_audio");
    }

    #[test]
    fn export_audio_to_fails_with_not_found_for_missing_entry() {
        let (storage, _dir) = make_service();
        let target_dir = TempDir::new().unwrap();

        let err = super::export::export_audio_to(
            &storage,
            &uuid::Uuid::new_v4().to_string(),
            &target_dir.path().join("export.opus"),
        )
        .unwrap_err();

        assert_not_found(err, "entry.not_found");
    }

    #[test]
    fn export_audio_to_maps_target_io_failure_to_copy_failed() {
        let (storage, _dir) = make_service();
        let contents = b"ogg opus payload";
        let id = seed_audio_entry(&storage, "audio.opus", contents);
        // Target parent directory does not exist — the copy fails at the OS
        // level (the save dialog guarantees an existing parent in real use;
        // this pins the error mapping).
        let target_dir = TempDir::new().unwrap();
        let target = target_dir.path().join("missing-dir").join("export.opus");

        let err = super::export::export_audio_to(&storage, &id.to_string(), &target).unwrap_err();

        match err {
            CommandError::Internal { code, .. } => assert_eq!(code, "export.copy_failed"),
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    /// A real Opus file exported to a `.wav` target is decoded to PCM WAV,
    /// and the cached original stays untouched (#252).
    #[test]
    fn export_audio_to_wav_target_converts_opus_audio() {
        let (storage, _dir) = make_service();
        let (id, _src_dir) = seed_opus_entry_from_sine(&storage);

        let target_dir = TempDir::new().unwrap();
        let target = target_dir.path().join("export.wav");

        super::export::export_audio_to(&storage, &id.to_string(), &target).unwrap();

        let reader = hound::WavReader::open(&target).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, 48_000);
        assert_eq!(spec.sample_format, hound::SampleFormat::Int);
        assert!(
            reader.duration() > 44_000,
            "expected ~1 s of audio, got {}",
            reader.duration()
        );
        assert!(
            storage
                .data_dir()
                .join("audio")
                .join(format!("{id}.opus"))
                .exists(),
            "cached opus original must stay in place"
        );
    }

    /// A corrupt `.opus` exported to a `.wav` target fails with the
    /// conversion error, not the copy error (#252).
    #[test]
    fn export_audio_to_wav_target_maps_decode_failure_to_convert_failed() {
        let (storage, _dir) = make_service();
        let id = seed_audio_entry(&storage, "audio.opus", b"corrupt payload");
        let target_dir = TempDir::new().unwrap();
        let target = target_dir.path().join("export.wav");

        let err = super::export::export_audio_to(&storage, &id.to_string(), &target).unwrap_err();

        match err {
            CommandError::Internal { code, .. } => assert_eq!(code, "export.convert_failed"),
            other => panic!("expected Internal, got {other:?}"),
        }
        assert!(!target.exists(), "no target file must be left behind");
    }

    /// A failure before the decoder writes anything (corrupt source) must
    /// not delete a file the user already had at the target path — the save
    /// dialog confirmed overwriting it, but this export never started (#252
    /// review follow-up).
    #[test]
    fn export_audio_to_wav_failure_keeps_preexisting_target() {
        let (storage, _dir) = make_service();
        let id = seed_audio_entry(&storage, "audio.opus", b"corrupt payload");
        let target_dir = TempDir::new().unwrap();
        let target = target_dir.path().join("export.wav");
        std::fs::write(&target, b"user's previous export").unwrap();

        let err = super::export::export_audio_to(&storage, &id.to_string(), &target).unwrap_err();

        match err {
            CommandError::Internal { code, .. } => assert_eq!(code, "export.convert_failed"),
            other => panic!("expected Internal, got {other:?}"),
        }
        assert_eq!(
            std::fs::read(&target).unwrap(),
            b"user's previous export",
            "the pre-existing target file must survive a failed conversion"
        );
    }

    /// The extension dispatch is case-insensitive: an `export.WAV` target
    /// converts just like a lowercase one.
    #[test]
    fn export_audio_to_uppercase_wav_target_converts() {
        let (storage, _dir) = make_service();
        let (id, _wav_dir) = seed_opus_entry_from_sine(&storage);

        let target_dir = TempDir::new().unwrap();
        let target = target_dir.path().join("export.WAV");

        super::export::export_audio_to(&storage, &id.to_string(), &target).unwrap();

        let reader = hound::WavReader::open(&target).unwrap();
        assert_eq!(reader.spec().sample_rate, 48_000);
    }

    /// A target without a recognizable extension is a byte-for-byte copy
    /// (spec: "every other combination") — the source bytes must arrive
    /// verbatim.
    #[test]
    fn export_audio_to_extensionless_target_copies_bytes() {
        let (storage, _dir) = make_service();
        let contents = b"ogg opus payload";
        let id = seed_audio_entry(&storage, "audio.opus", contents);
        let target_dir = TempDir::new().unwrap();
        let target = target_dir.path().join("exported-audio");

        super::export::export_audio_to(&storage, &id.to_string(), &target).unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), contents);
    }

    /// Seed a ready entry whose cached audio is a real Opus file (a sine
    /// WAV encoded through `encode_wav_to_opus`), so conversion tests
    /// exercise an actually decodable stream.
    fn seed_opus_entry_from_sine(storage: &StorageService) -> (EntryId, TempDir) {
        let id = storage.add_entry("текст".to_string()).unwrap().id;
        let dir = TempDir::new().unwrap();
        let wav_path = dir.path().join("src.wav");
        crate::storage::test_util::write_sine_wav(&wav_path, 24_000, 440.0, 0.25);
        crate::audio::encode_wav_to_opus(
            &wav_path,
            &storage.data_dir().join("audio").join(format!("{id}.opus")),
        )
        .unwrap();
        let mut entry = storage.get_entry(&id).unwrap();
        entry.audio_path = Some(format!("{id}.opus"));
        entry.status = EntryStatus::Ready;
        storage.update_entry(entry).unwrap();
        (id, dir)
    }
}

#[cfg(test)]
mod orchestration_tests;
