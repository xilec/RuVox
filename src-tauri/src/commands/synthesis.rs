//! Background synthesis: the pipeline handoff, the engine call, staged audio
//! publication and the queue-lifecycle transitions that follow a run.

use super::*;

/// Everything [`spawn_synthesis`] needs, snapshotted in one shot so call
/// sites stay one line. Built from the managed state via
/// [`SynthesisDeps::from_state`] in commands; the tray handler (which runs
/// before the state exists) fills the struct literal directly.
pub(crate) struct SynthesisDeps<R: Runtime> {
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
/// user-visible string written into `TextEntry.error_message`. `TtsFailed`
/// always emits a `tts_error` event (an engine error is the classic toast); any
/// other failure emits it only when the entry kept a playable recording — a
/// failed regeneration restores `ready`, where the stored message would
/// otherwise be invisible.
#[derive(Debug)]
pub(super) enum SynthesisError {
    PipelinePanic(String),
    EmptyText,
    TtsFailed(String),
    /// The audio was synthesized but could not be published (transcode or
    /// rename failure). The entry keeps its previous recording.
    StoreFailed(String),
}

impl SynthesisError {
    pub(super) fn user_message(&self) -> String {
        match self {
            Self::PipelinePanic(msg) => format!("pipeline task panicked: {msg}"),
            Self::EmptyText => "нормализация вернула пустой текст".to_string(),
            Self::TtsFailed(msg) => msg.clone(),
            Self::StoreFailed(msg) => format!("не удалось сохранить аудио: {msg}"),
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
pub(super) async fn run_pipeline_normalization(
    pipeline: Arc<Mutex<TTSPipeline>>,
    text: String,
    profile: &'static TextProfile,
) -> Result<(String, CharMapping, Vec<Span>, String), tokio::task::JoinError> {
    tokio::task::spawn_blocking(move || {
        let mut p = pipeline.lock();
        let processed = p.process_for_engine(&text, profile);
        let code_block_mode = p.code_block_mode().as_config_str().to_string();
        (
            processed.text,
            processed.mapping,
            processed.spans,
            code_block_mode,
        )
    })
    .await
}

/// Phase 1: run Rust text pipeline (CPU-bound, runs in blocking thread).
pub(super) async fn run_normalization(
    pipeline: Arc<Mutex<TTSPipeline>>,
    original_text: String,
    profile: &'static TextProfile,
) -> Result<(String, CharMapping, Vec<Span>, String), SynthesisError> {
    let (normalized, mapping, spans, code_block_mode) =
        run_pipeline_normalization(pipeline, original_text, profile)
            .await
            .map_err(|e| SynthesisError::PipelinePanic(e.to_string()))?;

    if normalized.is_empty() {
        return Err(SynthesisError::EmptyText);
    }
    Ok((normalized, mapping, spans, code_block_mode))
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
        EngineKind::TeraTts => config.teratts_voice.clone(),
    }
}

/// Staging name for the engine's WAV output. The canonical `<id>.wav` /
/// `<id>.opus` names are publications: a new recording is written here and only
/// renamed into place once it is complete, so a failed run never touches the
/// entry's stored audio.
pub(super) fn staged_wav_filename(entry_id: &EntryId) -> String {
    format!("{entry_id}.next.wav")
}

/// Partial artifacts a synthesis run may leave in the audio dir, for the
/// stale-result cleanup. They are never the entry's published recording.
fn transient_audio_names(entry_id: &EntryId) -> [String; 2] {
    [
        staged_wav_filename(entry_id),
        format!("{entry_id}.{}", crate::audio::OPUS_PART_EXTENSION),
    ]
}

/// Everything later synthesis phases need from [`synthesize_audio`]: the
/// engine output plus the inputs resolved *for this request* — the snapshot
/// must record the voice/settings actually used, not a fresh config read
/// (the user may flip settings while a synthesis is in flight).
#[derive(Debug)]
pub(super) struct SynthOutcome {
    pub(super) output: SynthesizeOutput,
    pub(super) out_wav_path: PathBuf,
    pub(super) wav_filename: String,
    pub(super) voice: String,
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
pub(super) async fn synthesize_audio(
    tts: &dyn TtsEngine,
    storage: &StorageService,
    piper_voices_dir: &std::path::Path,
    emitter: &crate::tts::supervisor::Emitter,
    synthesize_entered: &Mutex<HashSet<EntryId>>,
    entry_id: &EntryId,
    normalized: String,
    spans: Vec<Span>,
    mapping: &CharMapping,
) -> Result<SynthOutcome, SynthesisError> {
    // Engines write their WAV under a staging name: the entry's published
    // recording must stay untouched until the new audio is complete. The
    // canonical `<id>.wav` name is only used to publish a WAV fallback (and for
    // the generation snapshot).
    let wav_filename = format!("{entry_id}.wav");
    let out_wav_path = storage
        .data_dir()
        .join("audio")
        .join(staged_wav_filename(entry_id));
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
        .synthesize_spans(
            normalized.clone(),
            spans.clone(),
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
                .synthesize_spans(
                    normalized,
                    spans,
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
        bundle_version: info.bundle_version,
        model_id: info.model_id,
        upstream_revision: info.upstream_revision,
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

/// Phases 5 + 5b: publish the fresh audio and persist word timestamps.
///
/// Publication is ordered and atomic: the engine wrote `<id>.next.wav`; the
/// Opus encoder writes a `<id>.opus.part` sibling and renames it over
/// `<id>.opus` (or the staged WAV is renamed over `<id>.wav` when the encode
/// fails). A publish failure returns an error and leaves the entry's previous
/// recording untouched — the timestamps are saved only after the audio is in
/// place, so a failed publish cannot pair new word timings with old audio.
///
/// Timestamp save stays best-effort (`None` on failure): the audio is already
/// published and playback works without the word mapping.
pub(super) async fn finalize_audio_files(
    storage: &StorageService,
    entry_id: &EntryId,
    output: &SynthesizeOutput,
    staged_wav_path: PathBuf,
    wav_filename: &str,
) -> Result<(Option<String>, String), SynthesisError> {
    let audio_dir = storage.data_dir().join("audio");
    let opus_path = audio_dir.join(format!("{entry_id}.opus"));
    let publish_path = audio_dir.join(wav_filename);

    let staged_for_publish = staged_wav_path.clone();
    let opus_for_publish = opus_path.clone();
    let publish = tokio::task::spawn_blocking(move || {
        crate::audio::encode_wav_to_opus_atomic(&staged_for_publish, &opus_for_publish)
    })
    .await;

    let audio_filename = match publish {
        Ok(Ok(())) => {
            if let Err(e) = std::fs::remove_file(&staged_wav_path) {
                warn!("failed to remove staged wav for {entry_id}: {e}");
            }
            format!("{entry_id}.opus")
        }
        Ok(Err(e)) => {
            // The encoder rejected the WAV (e.g. an unsupported layout): keep
            // the staged file as the entry's recording, publishing it with a
            // rename so the previous audio survives a rename failure too.
            warn!("opus encode failed for {entry_id}, keeping wav: {e}");
            std::fs::rename(&staged_wav_path, &publish_path).map_err(|rename_err| {
                SynthesisError::StoreFailed(format!(
                    "staged wav {staged_wav_path:?} -> {publish_path:?}: {rename_err}"
                ))
            })?;
            wav_filename.to_string()
        }
        Err(e) => {
            warn!("opus encode task panicked for {entry_id}, keeping wav: {e}");
            std::fs::rename(&staged_wav_path, &publish_path).map_err(|rename_err| {
                SynthesisError::StoreFailed(format!(
                    "staged wav {staged_wav_path:?} -> {publish_path:?}: {rename_err}"
                ))
            })?;
            wav_filename.to_string()
        }
    };

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

    Ok((ts_filename, audio_filename))
}

/// Stale-completion guard: a synthesis completion or failure applies only
/// while the entry is still `processing`. A cancelled entry is `cancelled`
/// (terminal until regenerated), so its late result must be discarded
/// instead of resurrecting it to `ready` / `error`.
pub(super) fn completion_is_current(status: EntryStatus) -> bool {
    status == EntryStatus::Processing
}

/// Best-effort removal of one file from the audio dir. Missing is fine (a
/// cleanup may run after the file was already published or never written).
fn remove_audio_file(storage: &StorageService, name: &str) {
    let path = storage.data_dir().join("audio").join(name);
    match std::fs::remove_file(&path) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(e) => warn!("failed to remove audio file {}: {e}", path.display()),
    }
}

/// Best-effort removal of the staging files a run may have left behind
/// (`<id>.next.wav`, `<id>.opus.part`). Skipped while the entry is back in
/// flight: a newer regeneration owns those names and deleting them would make
/// its publish fail with `StoreFailed`.
pub(super) fn discard_transient_files(storage: &StorageService, entry_id: &EntryId) {
    let in_flight = storage
        .get_entry(entry_id)
        .is_some_and(|e| matches!(e.status, EntryStatus::Pending | EntryStatus::Processing));
    if in_flight {
        debug!("keeping transient audio files for {entry_id}: a new run owns them");
        return;
    }
    for name in transient_audio_names(entry_id) {
        remove_audio_file(storage, &name);
    }
}

/// Best-effort removal of the audio/timestamp files a discarded late result
/// wrote into the audio dir. Missing files are fine (e.g. the Opus transcode
/// never ran on the failure path).
///
/// A file the entry still references is kept: regeneration reuses the canonical
/// per-entry names, so a late result of a cancelled run and the recording that
/// run was going to replace can share a filename — deleting it by name would
/// wipe the restored audio.
pub(super) fn discard_late_files<I: IntoIterator<Item = String>>(
    storage: &StorageService,
    entry_id: &EntryId,
    names: I,
) {
    let referenced = storage.get_entry(entry_id).map(|e| {
        [e.audio_path, e.timestamps_path]
            .into_iter()
            .flatten()
            .collect::<Vec<String>>()
    });
    for name in names {
        if referenced
            .as_ref()
            .is_some_and(|referenced| referenced.contains(&name))
        {
            debug!("keeping stale result file {name}: the entry still references it");
            continue;
        }
        remove_audio_file(storage, &name);
    }
}

/// Phase 6 core (no Tauri handles, so the stale guard is unit-testable):
/// mark the entry `Ready` with audio + timestamp paths — but only if it is
/// still `processing`. A late completion for a non-`processing` entry is
/// discarded together with the files it just wrote. Returns `true` when the
/// result was applied.
pub(super) fn apply_ready_if_current(
    storage: &StorageService,
    entry_id: &EntryId,
    output: &SynthesizeOutput,
    ts_filename: Option<String>,
    audio_filename: &str,
    generation: GenerationParams,
) -> bool {
    // Read before the update: a successful publish retires the recording it
    // replaced (a WAV fallback left behind by a fresh Opus file).
    let previous_audio = storage.get_entry(entry_id).and_then(|e| e.audio_path);
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
        // The timestamps file always carries the canonical name, so the entry
        // "references" it even when the discarded run overwrote it with timings
        // for audio the entry did not keep. Wrong word timings are worse than
        // none: drop them with the audio they describe.
        let entry_kept_the_new_audio = storage
            .get_entry(entry_id)
            .and_then(|e| e.audio_path)
            .as_deref()
            == Some(audio_filename);
        let keep_ts = if entry_kept_the_new_audio {
            ts_filename.clone()
        } else {
            None
        };
        discard_late_files(
            storage,
            entry_id,
            [Some(audio_filename.to_string()), keep_ts]
                .into_iter()
                .flatten()
                .chain(transient_audio_names(entry_id)),
        );
        if !entry_kept_the_new_audio {
            if let Some(ts) = ts_filename {
                remove_audio_file(storage, &ts);
            }
        }
    } else if let Some(previous) = previous_audio {
        // Retire the file the fresh recording replaced, but only if the entry
        // still points at the fresh one: a concurrent regeneration may have
        // published something else since.
        let still_ours = storage.get_entry(entry_id).and_then(|e| e.audio_path);
        if previous != audio_filename && still_ours.as_deref() == Some(audio_filename) {
            remove_audio_file(storage, &previous);
        }
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
pub(crate) fn spawn_synthesis<R: Runtime + 'static>(
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
                let (normalized, mapping, spans, applied_code_block_mode) = run_normalization(
                    Arc::clone(&pipeline),
                    entry.original_text.clone(),
                    tts.text_profile(),
                )
                .await?;
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
                    spans,
                    &mapping,
                )
                .await?;
                // Read the header before finalize_audio_files publishes the
                // staged WAV and removes it.
                let sample_rate = wav_sample_rate(&outcome.out_wav_path);
                let (ts_filename, audio_filename) = finalize_audio_files(
                    &storage,
                    &entry_id,
                    &outcome.output,
                    outcome.out_wav_path,
                    &outcome.wav_filename,
                )
                .await?;
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
                // normalization), so the stale guard only applies to failures
                // raised after `mark_processing`.
                let require_processing = matches!(
                    err,
                    SynthesisError::TtsFailed(_) | SynthesisError::StoreFailed(_)
                );
                let applied = set_entry_error(&storage, &app, &entry_id, &msg, require_processing);
                // The run is no longer in flight (or a newer one owns the
                // staging names): drop the partial engine output. This also
                // covers a run aborted before `finalize_audio_files`.
                discard_transient_files(&storage, &entry_id);
                if applied {
                    // A failed regeneration keeps a playable entry; without the
                    // event the outcome is indistinguishable from a successful
                    // run, because nothing renders `error_message`.
                    let restores_a_playable_entry = storage
                        .get_entry(&entry_id)
                        .is_some_and(|e| e.audio_path.is_some());
                    if restores_a_playable_entry || matches!(err, SynthesisError::TtsFailed(_)) {
                        let _ = app.emit(
                            "tts_error",
                            json!({ "entry_id": entry_id.to_string(), "message": msg }),
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
pub(super) fn cleanup_finished_handle(
    tasks: &Mutex<HashMap<EntryId, AbortHandle>>,
    entry_id: &EntryId,
) {
    let mut tasks = tasks.lock();
    if tasks.get(entry_id).is_some_and(AbortHandle::is_finished) {
        tasks.remove(entry_id);
    }
}

/// Core of [`set_entry_error`] without Tauri handles: record the failure. With
/// `require_processing` the stale-completion guard applies — a failure arriving
/// for an entry that left `processing` (e.g. cancelled to `cancelled`) is
/// discarded together with the files the late request may have written.
/// Normalization-stage failures pass `false` because the entry is legitimately
/// still `pending` before `mark_processing` runs.
///
/// The status follows the entry's own state: an entry that still owns a
/// recording (a regeneration that failed without replacing it) returns to
/// `ready`, so the audio stays playable; an entry without one becomes `error`.
/// Returns `true` when the failure was applied.
pub(super) fn apply_error_if_current(
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
            // A failed regeneration must not hide a recording it never
            // replaced: an entry that still owns audio returns to `ready`,
            // which is the only status playback accepts. A first synthesis (no
            // `audio_path`) keeps the terminal `error` outcome.
            e.status = if e.audio_path.is_some() {
                EntryStatus::Ready
            } else {
                EntryStatus::Error
            };
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
                    entry_id,
                    [
                        format!("{entry_id}.wav"),
                        format!("{entry_id}.opus"),
                        format!("{entry_id}.timestamps.json"),
                    ]
                    .into_iter()
                    .chain(transient_audio_names(entry_id)),
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
