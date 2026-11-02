use super::synthesis::*;
use super::*;
use crate::storage::test_util::make_service;

#[tokio::test]
async fn run_normalization_returns_normalized_text_and_mapping() {
    let pipeline = Arc::new(Mutex::new(TTSPipeline::new()));
    let (normalized, _mapping, _spans, _mode) = run_normalization(
        pipeline,
        "Привет мир".to_string(),
        &crate::tts::profiles::SILERO_PROFILE,
    )
    .await
    .unwrap();
    assert!(!normalized.is_empty());
}

#[tokio::test]
async fn run_normalization_flags_empty_input_as_empty_text() {
    let pipeline = Arc::new(Mutex::new(TTSPipeline::new()));
    let err = run_normalization(
        pipeline,
        String::new(),
        &crate::tts::profiles::SILERO_PROFILE,
    )
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
    // fallback path that publishes the staged WAV as the entry's recording.
    let wav_filename = format!("{id}.wav");
    let audio_dir = storage.data_dir().join("audio");
    let wav_path = audio_dir.join(&wav_filename);
    let staged_path = audio_dir.join(staged_wav_filename(&id));
    std::fs::write(&staged_path, b"not a wav file").unwrap();
    // A previous recording must survive the encode attempt...
    std::fs::write(&wav_path, b"previous recording").unwrap();

    let output = SynthesizeOutput {
        timestamps: Vec::new(),
        duration_sec: 1.0,
    };

    let (ts_filename, audio_filename) =
        finalize_audio_files(&storage, &id, &output, staged_path.clone(), &wav_filename)
            .await
            .unwrap();
    assert!(ts_filename.is_some());
    assert_eq!(audio_filename, wav_filename);
    // ...until the complete staged WAV replaces it, which is what
    // `audio_filename` now points at.
    assert_eq!(std::fs::read(&wav_path).unwrap(), b"not a wav file");
    assert!(
        !staged_path.exists(),
        "the staged wav is published, not kept"
    );
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
    assert_eq!(
        SynthesisError::StoreFailed("disk full".into()).user_message(),
        "не удалось сохранить аудио: disk full",
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
        EntryStatus::Cancelled,
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

/// A discarded run that published under a different name must take its
/// timestamps with it: the canonical timestamps name is always "referenced",
/// so keeping it would pair new word timings with the old recording.
#[test]
fn stale_completion_discards_timestamps_of_a_different_audio_name() {
    let (storage, _dir) = make_service();
    let entry = storage.add_entry("текст".to_string()).unwrap();
    let id = entry.id;
    let audio_dir = storage.data_dir().join("audio");
    let wav_name = format!("{id}.wav");
    let opus_name = format!("{id}.opus");
    let ts_name = format!("{id}.timestamps.json");
    std::fs::write(audio_dir.join(&wav_name), b"old wav").unwrap();
    std::fs::write(audio_dir.join(&ts_name), b"old timings").unwrap();
    let mut stored = storage.get_entry(&id).unwrap();
    stored.status = EntryStatus::Cancelled;
    stored.audio_path = Some(wav_name.clone());
    stored.timestamps_path = Some(ts_name.clone());
    storage.update_entry(stored).unwrap();
    // The discarded run published Opus and rewrote the timestamps.
    std::fs::write(audio_dir.join(&opus_name), b"new opus").unwrap();
    std::fs::write(audio_dir.join(&ts_name), b"new timings").unwrap();

    let applied = apply_ready_if_current(
        &storage,
        &id,
        &fake_output(),
        Some(ts_name.clone()),
        &opus_name,
        fake_generation(),
    );

    assert!(!applied);
    assert!(
        audio_dir.join(&wav_name).exists(),
        "the entry's recording survives"
    );
    assert!(
        !audio_dir.join(&opus_name).exists(),
        "the discarded audio is removed"
    );
    assert!(
        !audio_dir.join(&ts_name).exists(),
        "timings describing the discarded audio are removed too"
    );
}

/// A successful publish retires the recording it replaced, so a WAV
/// fallback does not linger after the entry moved to Opus.
#[test]
fn applied_completion_retires_the_replaced_audio_file() {
    let (storage, _dir) = make_service();
    let entry = storage.add_entry("текст".to_string()).unwrap();
    let id = entry.id;
    let audio_dir = storage.data_dir().join("audio");
    let wav_name = format!("{id}.wav");
    let opus_name = format!("{id}.opus");
    std::fs::write(audio_dir.join(&wav_name), b"old wav").unwrap();
    std::fs::write(audio_dir.join(&opus_name), b"new opus").unwrap();
    set_status(&storage, &entry, EntryStatus::Processing);
    let mut stored = storage.get_entry(&id).unwrap();
    stored.audio_path = Some(wav_name.clone());
    storage.update_entry(stored).unwrap();

    let applied = apply_ready_if_current(
        &storage,
        &id,
        &fake_output(),
        None,
        &opus_name,
        fake_generation(),
    );

    assert!(applied);
    assert!(
        !audio_dir.join(&wav_name).exists(),
        "the replaced recording is retired"
    );
    assert_eq!(
        storage.get_entry(&id).unwrap().audio_path.as_deref(),
        Some(opus_name.as_str())
    );
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
        bundle_version: None,
        model_id: None,
        upstream_revision: None,
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
    for status in [
        EntryStatus::Ready,
        EntryStatus::Error,
        EntryStatus::Cancelled,
    ] {
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

    let (updated, entered_tts) = cancel_entry(&storage, &tasks, &entered, &id.to_string()).unwrap();

    assert_eq!(updated.status, EntryStatus::Cancelled);
    assert!(!entered_tts);
    assert!(tasks.lock().is_empty());
    let join_err = sleeper.await.unwrap_err();
    assert!(join_err.is_cancelled());
}

/// Cancel flips the entry to `pending`, removes both registry keys, and
/// the registered task is actually aborted.
#[tokio::test]
async fn cancel_entry_aborts_registered_task_and_sets_cancelled() {
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

    let (updated, entered_tts) = cancel_entry(&storage, &tasks, &entered, &id.to_string()).unwrap();

    assert_eq!(updated.status, EntryStatus::Cancelled);
    assert!(entered_tts, "entry was marked as inside the TTS stage");
    assert!(tasks.lock().is_empty());
    assert!(entered.lock().is_empty());
    assert_eq!(
        storage.get_entry(&id).unwrap().status,
        EntryStatus::Cancelled
    );

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
