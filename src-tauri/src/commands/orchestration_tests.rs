//! Command-orchestration tests (issue #104) on the `MockRuntime` harness
//! (`crate::test_support`): real command handlers against a managed
//! [`AppState`] with real storage (TempDir), a stub TTS engine, and a
//! fake player. Background synthesis runs in spawned tasks, so state
//! transitions are awaited with `wait_until` instead of being assumed.

use super::*;
use crate::player::PlayerBackend;
use crate::test_support::{
    PlayerCall, TestApp, build_test_app, build_test_app_with_kind, record_events, wait_until,
};
use crate::tts::engine::EngineKind;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(5);

/// Absolute path of the storage audio directory for this test app.
fn audio_dir(t: &TestApp) -> PathBuf {
    t.state().storage.data_dir().join("audio")
}

/// Parse the id string returned by the commands into the storage UUID.
fn entry_uuid(id: &str) -> uuid::Uuid {
    id.parse().unwrap()
}

/// Paths of an entry's audio file and timestamps sidecar in the storage
/// audio directory.
fn audio_paths(t: &TestApp, uuid: &uuid::Uuid) -> (PathBuf, PathBuf) {
    let dir = audio_dir(t);
    (
        dir.join(format!("{uuid}.wav")),
        dir.join(format!("{uuid}.timestamps.json")),
    )
}

/// Wait until the entry reaches `status` in storage.
async fn wait_entry_status(t: &TestApp, uuid: &uuid::Uuid, status: EntryStatus) {
    wait_until(&format!("entry status {status:?}"), TIMEOUT, || {
        t.state()
            .storage
            .get_entry(uuid)
            .is_some_and(|e| e.status == status)
    })
    .await;
}

/// Payload of the most recent `entry_updated` event in the log.
fn last_entry_updated(log: &[(String, serde_json::Value)]) -> serde_json::Value {
    log.iter()
        .rev()
        .find(|(n, _)| n == "entry_updated")
        .unwrap_or_else(|| panic!("no entry_updated event in {log:?}"))
        .1
        .clone()
}

/// Add an entry through the real ingestion command and wait until the
/// background synthesis (StubEngine) has marked it `ready`.
async fn add_ready_entry(t: &TestApp) -> String {
    let id = add_text_entry(
        t.app.handle().clone(),
        t.state(),
        "текст для озвучки".to_string(),
        false,
        None,
        None,
    )
    .await
    .unwrap();
    wait_entry_status(t, &entry_uuid(&id), EntryStatus::Ready).await;
    id
}

// ── delete_entry ─────────────────────────────────────────────────────

/// Deleting the currently-playing entry stops playback before removing
/// the entry and its files. (The `playback_stopped` event itself is
/// emitted by the real `Player::stop` — `FakePlayer` deliberately emits
/// no events, so the assertion is on the recorded `stop` control call.)
#[tokio::test(flavor = "multi_thread")]
async fn delete_entry_stops_playback_and_removes_entry_and_files() {
    let t = build_test_app();
    let id = add_ready_entry(&t).await;
    let uuid = entry_uuid(&id);
    let (audio_file, ts_file) = audio_paths(&t, &uuid);
    assert!(audio_file.exists());
    assert!(ts_file.exists());

    play_entry(t.state(), id.clone()).await.unwrap();
    assert_eq!(t.player.current_entry_id().as_deref(), Some(id.as_str()));
    assert!(t.player.is_playing());

    delete_entry(t.state(), id.clone()).await.unwrap();

    // stop() was issued, after playback had started.
    let calls = t.player.calls();
    let play_idx = calls
        .iter()
        .position(|c| matches!(c, PlayerCall::Play))
        .unwrap();
    let stop_idx = calls
        .iter()
        .position(|c| matches!(c, PlayerCall::Stop))
        .expect("delete_entry must stop the playing entry");
    assert!(stop_idx > play_idx);
    assert!(!t.player.is_playing());

    assert!(t.state().storage.get_entry(&uuid).is_none());
    assert!(!audio_file.exists());
    assert!(!ts_file.exists());
}

/// Deleting an entry that is *not* the playing one leaves playback alone.
#[tokio::test(flavor = "multi_thread")]
async fn delete_entry_of_other_entry_keeps_playback_running() {
    let t = build_test_app();
    let playing = add_ready_entry(&t).await;
    let other = add_ready_entry(&t).await;

    play_entry(t.state(), playing.clone()).await.unwrap();
    delete_entry(t.state(), other.clone()).await.unwrap();

    assert!(
        t.player
            .calls()
            .iter()
            .all(|c| !matches!(c, PlayerCall::Stop))
    );
    assert!(t.player.is_playing());
    assert_eq!(
        t.player.current_entry_id().as_deref(),
        Some(playing.as_str())
    );
}

// ── regenerate_entry ─────────────────────────────────────────────────

/// Happy path: the old audio file is dropped, the entry is flagged
/// `was_regenerated` (and emitted), and a fresh synthesis brings it back
/// to `ready` with newly written audio.
#[tokio::test(flavor = "multi_thread")]
async fn regenerate_entry_replaces_audio_and_resynthesizes() {
    let t = build_test_app();
    let id = add_ready_entry(&t).await;
    let uuid = entry_uuid(&id);
    let (audio_file, _) = audio_paths(&t, &uuid);
    // Sentinel content: proves the new file was written from scratch,
    // not just left over from the first synthesis.
    std::fs::write(&audio_file, b"old-marker").unwrap();

    let events = record_events(&t.app, &["entry_updated"]);
    regenerate_entry(t.app.handle().clone(), t.state(), id.clone())
        .await
        .unwrap();

    wait_entry_status(&t, &uuid, EntryStatus::Ready).await;

    assert_eq!(
        std::fs::read(&audio_file).unwrap().as_slice(),
        b"stub audio"
    );

    let entry = t.state().storage.get_entry(&uuid).unwrap();
    assert!(entry.was_regenerated);
    assert!(entry.error_message.is_none());

    // The command emitted entry_updated carrying was_regenerated: true.
    let log = events.lock().unwrap();
    assert!(
        log.iter()
            .any(|(_, p)| p["entry"]["id"] == id && p["entry"]["was_regenerated"] == true)
    );
}

/// Regeneration is rejected while the entry is `processing`; the
/// in-flight synthesis must continue undisturbed.
#[tokio::test(flavor = "multi_thread")]
async fn regenerate_entry_rejects_processing_entry_and_synthesis_continues() {
    let t = build_test_app();
    t.engine.block_synthesis();
    let id = add_text_entry(
        t.app.handle().clone(),
        t.state(),
        "текст".to_string(),
        false,
        None,
        None,
    )
    .await
    .unwrap();
    let uuid = entry_uuid(&id);
    wait_entry_status(&t, &uuid, EntryStatus::Processing).await;

    let err = regenerate_entry(t.app.handle().clone(), t.state(), id.clone())
        .await
        .unwrap_err();
    match err {
        CommandError::SynthesisError { message } => {
            assert!(message.contains("уже синтезируется"))
        }
        other => panic!("expected SynthesisError, got {other:?}"),
    }

    // Releasing the gate lets the original synthesis finish normally.
    t.engine.release_synthesis();
    wait_entry_status(&t, &uuid, EntryStatus::Ready).await;
}

// ── cancel_synthesis (new #129 semantics) ─────────────────────────────

/// Cancelling an in-flight synthesis aborts the spawned task, flips the
/// entry back to `pending` (emitting `entry_updated`), and clears both
/// registries. Nothing is resurrected afterwards: no `ready` event, no
/// autoplay — even though the entry was added with `play_when_ready`.
#[tokio::test(flavor = "multi_thread")]
async fn cancel_synthesis_aborts_in_flight_task_and_keeps_entry_pending() {
    let t = build_test_app();
    t.engine.block_synthesis();
    let events = record_events(&t.app, &["entry_updated"]);
    let id = add_text_entry(
        t.app.handle().clone(),
        t.state(),
        "текст".to_string(),
        true,
        None,
        None,
    )
    .await
    .unwrap();
    let uuid = entry_uuid(&id);
    // Deterministic "inside the TTS stage": the marker is set right
    // before the (blocked) engine await.
    wait_until("entry entered TTS stage", TIMEOUT, || {
        t.state().synthesize_entered.lock().contains(&uuid)
    })
    .await;

    cancel_synthesis(t.app.handle().clone(), t.state(), id.clone())
        .await
        .unwrap();

    let entry = t.state().storage.get_entry(&uuid).unwrap();
    assert_eq!(entry.status, EntryStatus::Pending);
    assert!(t.state().synthesis_tasks.lock().is_empty());
    assert!(t.state().synthesize_entered.lock().is_empty());

    // The last entry_updated is the reset to pending.
    {
        let log = events.lock().unwrap();
        let payload = last_entry_updated(&log);
        assert_eq!(payload["entry"]["id"], id);
        assert_eq!(payload["entry"]["status"], "pending");
    }

    // The task was aborted at the blocked await: releasing the gate
    // changes nothing — no ready event and no autoplay.
    t.engine.release_synthesis();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let log = events.lock().unwrap();
    assert!(log.iter().all(|(_, p)| p["entry"]["status"] != "ready"));
    assert!(t.player.calls().is_empty());
}

/// Cancelling an idle (pending, no synthesis task) entry succeeds and
/// simply re-confirms `pending` with an `entry_updated` event.
#[tokio::test(flavor = "multi_thread")]
async fn cancel_synthesis_on_idle_entry_succeeds_and_stays_pending() {
    let t = build_test_app();
    let entry = t.state().storage.add_entry("текст".to_string()).unwrap();
    let id = entry.id.to_string();

    let events = record_events(&t.app, &["entry_updated"]);
    cancel_synthesis(t.app.handle().clone(), t.state(), id.clone())
        .await
        .unwrap();

    let stored = t.state().storage.get_entry(&entry.id).unwrap();
    assert_eq!(stored.status, EntryStatus::Pending);
    let log = events.lock().unwrap();
    assert!(
        log.iter()
            .any(|(_, p)| p["entry"]["id"] == id && p["entry"]["status"] == "pending")
    );
}

// ── play_entry ───────────────────────────────────────────────────────

/// A non-ready (pending) entry is rejected with `playback_error` and the
/// player is never touched.
#[tokio::test(flavor = "multi_thread")]
async fn play_entry_rejects_non_ready_entry() {
    let t = build_test_app();
    let entry = t.state().storage.add_entry("текст".to_string()).unwrap();

    let err = play_entry(t.state(), entry.id.to_string())
        .await
        .unwrap_err();
    match err {
        CommandError::PlaybackError { message } => assert!(message.contains("not ready")),
        other => panic!("expected PlaybackError, got {other:?}"),
    }
    assert!(t.player.calls().is_empty());
}

/// A ready entry is loaded (full audio path + entry id) and played.
#[tokio::test(flavor = "multi_thread")]
async fn play_entry_loads_audio_and_starts_playback() {
    let t = build_test_app();
    let id = add_ready_entry(&t).await;

    play_entry(t.state(), id.clone()).await.unwrap();

    let (expected_path, _) = audio_paths(&t, &entry_uuid(&id));
    let calls = t.player.calls();
    assert_eq!(calls.len(), 2);
    match &calls[0] {
        PlayerCall::Load(path, entry_id) => {
            assert_eq!(path, &expected_path);
            assert_eq!(entry_id, &id);
        }
        other => panic!("expected Load first, got {other:?}"),
    }
    assert!(matches!(calls[1], PlayerCall::Play));
    assert_eq!(t.player.current_entry_id().as_deref(), Some(id.as_str()));
    assert!(t.player.is_playing());
}

// ── update_config rollback ───────────────────────────────────────────

/// A failed engine switch (deterministic `engine_unknown` for "nemo")
/// aborts `update_config` with `config_error`: the previous config stays
/// on disk and the active engine is untouched.
#[tokio::test(flavor = "multi_thread")]
async fn update_config_failed_engine_switch_preserves_previous_config() {
    let t = build_test_app();
    let before = get_config(t.state()).await.unwrap();
    assert_eq!(before.engine, "silero_native");

    let patch = UIConfigPatch {
        engine: Some("nemo".to_string()),
        ..Default::default()
    };
    let err = update_config(t.state(), patch).await.unwrap_err();
    match err {
        CommandError::ConfigError { message } => {
            assert!(message.contains("не удалось переключить движок"))
        }
        other => panic!("expected ConfigError, got {other:?}"),
    }

    let after = get_config(t.state()).await.unwrap();
    assert_eq!(
        serde_json::to_value(&after).unwrap(),
        serde_json::to_value(&before).unwrap(),
        "config must be unchanged after a failed engine switch"
    );
    assert_eq!(t.state().tts.kind(), EngineKind::Piper);
}

/// Picking `silero_native` without a downloaded bundle aborts
/// `update_config` with `config_error` (the switcher fails fast with
/// `engine_unavailable`): the previous config stays on disk and the active
/// engine is untouched — same rollback contract as an unknown engine.
#[tokio::test(flavor = "multi_thread")]
async fn update_config_silero_native_without_bundle_preserves_previous_config() {
    let t = build_test_app();
    let before = get_config(t.state()).await.unwrap();
    assert_eq!(before.engine, "silero_native");

    let patch = UIConfigPatch {
        engine: Some("silero_native".to_string()),
        ..Default::default()
    };
    let err = update_config(t.state(), patch).await.unwrap_err();
    match err {
        CommandError::ConfigError { message } => {
            assert!(message.contains("не удалось переключить движок"))
        }
        other => panic!("expected ConfigError, got {other:?}"),
    }

    let after = get_config(t.state()).await.unwrap();
    assert_eq!(
        serde_json::to_value(&after).unwrap(),
        serde_json::to_value(&before).unwrap(),
        "config must be unchanged after a failed engine switch"
    );
    assert_eq!(t.state().tts.kind(), EngineKind::Piper);
}

/// `get_available_engines` reports `silero_native` as unavailable with a
/// Russian reason when no bundle is installed (the test app's bundle dir is
/// an empty TempDir).
#[tokio::test(flavor = "multi_thread")]
async fn get_available_engines_reports_silero_native_unavailable_without_bundle() {
    let t = build_test_app();
    let engines = get_available_engines(t.state()).await.unwrap();
    assert!(engines.piper.available);
    assert!(!engines.silero_native.available);
    let reason = engines.silero_native.reason.expect("reason set");
    assert!(
        reason.chars().any(|c| matches!(c, 'А'..='я' | 'ё' | 'Ё')),
        "reason should be Russian: {reason}"
    );
}

// ── events ───────────────────────────────────────────────────────────

/// `delete_audio` drops the audio/timestamps files, resets the entry to
/// `pending`, and emits `entry_updated` with the reset entry.
#[tokio::test(flavor = "multi_thread")]
async fn delete_audio_resets_entry_and_emits_entry_updated() {
    let t = build_test_app();
    let id = add_ready_entry(&t).await;
    let uuid = entry_uuid(&id);
    let (audio_file, ts_file) = audio_paths(&t, &uuid);

    let events = record_events(&t.app, &["entry_updated"]);
    delete_audio(t.app.handle().clone(), t.state(), id.clone())
        .await
        .unwrap();

    let entry = t.state().storage.get_entry(&uuid).unwrap();
    assert_eq!(entry.status, EntryStatus::Pending);
    assert!(entry.audio_path.is_none());
    assert!(entry.timestamps_path.is_none());
    assert!(entry.duration_sec.is_none());
    assert!(!audio_file.exists());
    assert!(!ts_file.exists());

    let log = events.lock().unwrap();
    let payload = last_entry_updated(&log);
    assert_eq!(payload["entry"]["id"], id);
    assert_eq!(payload["entry"]["status"], "pending");
    assert!(payload["entry"]["audio_path"].is_null());
}

// ── set_entry_format ─────────────────────────────────────────────────

/// Switching the format persists it and emits `entry_updated`, leaving the
/// synthesized artifacts (normalized text, audio, timestamps) untouched.
#[tokio::test(flavor = "multi_thread")]
async fn set_entry_format_persists_and_preserves_audio_artifacts() {
    let t = build_test_app();
    let id = add_ready_entry(&t).await;
    let uuid = entry_uuid(&id);
    let before = t.state().storage.get_entry(&uuid).unwrap();

    let events = record_events(&t.app, &["entry_updated"]);
    set_entry_format(
        t.app.handle().clone(),
        t.state(),
        id.clone(),
        TextFormat::Html,
    )
    .await
    .unwrap();

    let entry = t.state().storage.get_entry(&uuid).unwrap();
    assert_eq!(entry.format, Some(TextFormat::Html));
    assert_eq!(entry.normalized_text, before.normalized_text);
    assert_eq!(entry.audio_path, before.audio_path);
    assert_eq!(entry.timestamps_path, before.timestamps_path);
    assert_eq!(entry.status, EntryStatus::Ready);

    let log = events.lock().unwrap();
    let payload = last_entry_updated(&log);
    assert_eq!(payload["entry"]["id"], id);
    assert_eq!(payload["entry"]["format"], "html");
}

/// A well-formed but unknown id is a typed `not_found` error and emits
/// nothing.
#[tokio::test(flavor = "multi_thread")]
async fn set_entry_format_unknown_entry_is_rejected() {
    let t = build_test_app();
    let events = record_events(&t.app, &["entry_updated"]);

    let err = set_entry_format(
        t.app.handle().clone(),
        t.state(),
        uuid::Uuid::new_v4().to_string(),
        TextFormat::Plain,
    )
    .await
    .unwrap_err();

    assert!(matches!(err, CommandError::NotFound { .. }));
    assert!(events.lock().unwrap().is_empty());
}

// ── HTML ingestion ───────────────────────────────────────────────────

/// `add_text_entry` with `format: "html"` persists the sanitized
/// `html_source` and synthesizes the extracted text, not the markup.
#[tokio::test(flavor = "multi_thread")]
async fn add_text_entry_with_html_params_persists_source_and_synthesizes_text() {
    let t = build_test_app();

    let id = add_text_entry(
        t.app.handle().clone(),
        t.state(),
        "Вызови API".to_string(),
        false,
        Some(TextFormat::Html),
        Some("<p>Вызови <code>API</code></p>".to_string()),
    )
    .await
    .unwrap();
    let uuid = entry_uuid(&id);
    wait_entry_status(&t, &uuid, EntryStatus::Ready).await;

    let entry = t.state().storage.get_entry(&uuid).unwrap();
    assert_eq!(entry.format, Some(TextFormat::Html));
    assert_eq!(
        entry.html_source.as_deref(),
        Some("<p>Вызови <code>API</code></p>")
    );
    assert_eq!(entry.original_text, "Вызови API");
    // The pipeline normalized the extracted text (Latin is transliterated),
    // so the markup never reached synthesis.
    let normalized = entry.normalized_text.unwrap_or_default();
    assert!(normalized.contains("эй пи ай"), "normalized: {normalized}");
    assert!(!normalized.contains('<'), "normalized: {normalized}");
}

/// A TTS-stage failure flips the entry to `error` and emits
/// `entry_updated` (status `error`) *before* `tts_error` — the ordering
/// the spec's "Synthesis Failure Event" requirement pins.
#[tokio::test(flavor = "multi_thread")]
async fn tts_failure_emits_entry_updated_error_then_tts_error() {
    let t = build_test_app();
    t.engine.fail_with("stub synthesis boom");
    let events = record_events(&t.app, &["entry_updated", "tts_error"]);

    let id = add_text_entry(
        t.app.handle().clone(),
        t.state(),
        "текст".to_string(),
        false,
        None,
        None,
    )
    .await
    .unwrap();
    let uuid = entry_uuid(&id);
    // Wait on the event, not the storage status: the status flips to
    // `error` *before* the events are emitted, so polling the status
    // would race the emits. `tts_error` is emitted last, so once it is
    // in the log every preceding event is too.
    wait_until("tts_error emitted", TIMEOUT, || {
        events.lock().unwrap().iter().any(|(n, _)| n == "tts_error")
    })
    .await;

    let entry = t.state().storage.get_entry(&uuid).unwrap();
    assert_eq!(entry.status, EntryStatus::Error);
    assert!(
        entry
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("stub synthesis boom")
    );

    let log = events.lock().unwrap();
    let err_idx = log
        .iter()
        .position(|(n, p)| n == "entry_updated" && p["entry"]["status"] == "error")
        .expect("entry_updated with status error must be emitted");
    let tts_idx = log
        .iter()
        .position(|(n, _)| n == "tts_error")
        .expect("tts_error must be emitted");
    assert!(err_idx < tts_idx, "entry_updated(error) precedes tts_error");
    assert_eq!(log[tts_idx].1["entry_id"], id);
    assert!(
        log[tts_idx].1["message"]
            .as_str()
            .unwrap()
            .contains("stub synthesis boom")
    );
}

/// `seek_to` forwards the absolute position to the player. (The
/// immediate `playback_position` emit lives in the real `Player::seek`,
/// which needs mpv; `FakePlayer` emits no events by design. The emitter
/// loop's own output is covered by
/// `test_support::tests::position_emitter_ticks_then_finishes_at_eof`.)
#[tokio::test(flavor = "multi_thread")]
async fn seek_to_forwards_absolute_seek_to_player() {
    let t = build_test_app();
    seek_to(t.state(), 2.0).await.unwrap();
    assert!(
        t.player
            .calls()
            .iter()
            .any(|c| matches!(c, PlayerCall::Seek(p) if (*p - 2.0).abs() < f64::EPSILON))
    );
}

// ── preview_normalize (restored from the plan journal: removed as a
//    tautological helper test in #124, reinstated at command level) ────

/// The preview runs the real pipeline through the real command with a
/// managed `AppState` and leaves no trace: no history entry, no audio
/// files, no spawned synthesis task.
#[tokio::test(flavor = "multi_thread")]
async fn preview_normalize_returns_text_without_history_or_audio_side_effects() {
    let t = build_test_app();

    let result = preview_normalize(t.state(), "Вызови getUserData() через API".to_string())
        .await
        .unwrap();
    assert!(!result.normalized.is_empty());

    assert!(get_entries(t.state()).await.unwrap().is_empty());
    assert_eq!(std::fs::read_dir(audio_dir(&t)).unwrap().count(), 0);
    assert!(t.state().synthesis_tasks.lock().is_empty());
}

// ── input length limit (MAX_INPUT_CHARS = 100 000 codepoints, Piper only) ──

/// Oversized text is rejected by `add_text_entry` before persistence when the
/// active engine is Piper (the default test-app kind): typed `internal` error
/// naming the engine and the limit, no entry, no synthesis task.
#[tokio::test(flavor = "multi_thread")]
async fn add_text_entry_rejects_oversized_input_before_persistence() {
    let t = build_test_app();
    assert_eq!(t.state().tts.kind(), EngineKind::Piper);

    let err = add_text_entry(
        t.app.handle().clone(),
        t.state(),
        "а".repeat(MAX_INPUT_CHARS + 1),
        false,
        None,
        None,
    )
    .await
    .expect_err("oversized input must be rejected");
    match err {
        CommandError::Internal { message } => {
            assert!(
                message.contains("100 000"),
                "message names the limit: {message}"
            );
            assert!(
                message.contains("Piper"),
                "message names the engine: {message}"
            );
        }
        other => panic!("expected internal error, got {other:?}"),
    }

    assert!(get_entries(t.state()).await.unwrap().is_empty());
    assert!(t.state().synthesis_tasks.lock().is_empty());
}

/// Oversized text is rejected by `preview_normalize` before normalization
/// when the active engine is Piper.
#[tokio::test(flavor = "multi_thread")]
async fn preview_normalize_rejects_oversized_input() {
    let t = build_test_app();

    let err = preview_normalize(t.state(), "а".repeat(MAX_INPUT_CHARS + 1))
        .await
        .expect_err("oversized input must be rejected");
    match err {
        CommandError::Internal { message } => {
            assert!(
                message.contains("100 000"),
                "message names the limit: {message}"
            );
            assert!(
                message.contains("Piper"),
                "message names the engine: {message}"
            );
        }
        other => panic!("expected internal error, got {other:?}"),
    }
}

/// Text of exactly `MAX_INPUT_CHARS` codepoints is accepted and synthesized.
#[tokio::test(flavor = "multi_thread")]
async fn add_text_entry_accepts_input_at_limit() {
    let t = build_test_app();

    let id = add_text_entry(
        t.app.handle().clone(),
        t.state(),
        "а".repeat(MAX_INPUT_CHARS),
        false,
        None,
        None,
    )
    .await
    .expect("input at the limit must be accepted");
    wait_entry_status(&t, &entry_uuid(&id), EntryStatus::Ready).await;
}

/// With Silero active the length limit does not apply: Silero synthesizes in
/// bounded chunks, so oversized input is ingested and synthesized.
#[tokio::test(flavor = "multi_thread")]
async fn add_text_entry_accepts_oversized_input_with_silero() {
    let t = build_test_app_with_kind(EngineKind::Silero);
    assert_eq!(t.state().tts.kind(), EngineKind::Silero);

    let id = add_text_entry(
        t.app.handle().clone(),
        t.state(),
        "а".repeat(MAX_INPUT_CHARS + 1),
        false,
        None,
        None,
    )
    .await
    .expect("oversized input must be accepted when Silero is active");
    wait_entry_status(&t, &entry_uuid(&id), EntryStatus::Ready).await;
}

/// With Silero active `preview_normalize` normalizes oversized input instead
/// of rejecting it — in full, with no content dropped.
#[tokio::test(flavor = "multi_thread")]
async fn preview_normalize_accepts_oversized_input_with_silero() {
    let t = build_test_app_with_kind(EngineKind::Silero);

    let result = preview_normalize(t.state(), "а".repeat(MAX_INPUT_CHARS + 1))
        .await
        .expect("oversized input must be normalized when Silero is active");
    assert_eq!(result.normalized.chars().count(), MAX_INPUT_CHARS + 1);
}

/// The length guard is re-checked at synthesis time: an oversized entry that
/// was accepted while Silero was active (simulated here by inserting it
/// directly into storage) must fail with the limit message when synthesis
/// runs under Piper, instead of feeding Piper an unchunked oversized run.
#[tokio::test(flavor = "multi_thread")]
async fn synthesis_under_piper_fails_oversized_entry_accepted_under_silero() {
    let t = build_test_app();
    assert_eq!(t.state().tts.kind(), EngineKind::Piper);

    let entry = t
        .state()
        .storage
        .add_entry_with_source("а".repeat(MAX_INPUT_CHARS + 1), None, None)
        .unwrap();
    let uuid = entry.id;

    spawn_synthesis(
        SynthesisDeps::from_state(t.app.handle(), &t.state()),
        uuid,
        false,
    );

    wait_entry_status(&t, &uuid, EntryStatus::Error).await;
    let entry = t.state().storage.get_entry(&uuid).unwrap();
    let message = entry.error_message.expect("error entry carries a message");
    assert!(
        message.contains("Piper"),
        "message names the engine: {message}"
    );
}

// ── set_speed / set_volume range guards ──────────────────────────────

/// The real `set_speed` command accepts the documented inclusive range
/// [0.5, 3.0], forwards each value to the player, and persists the last
/// one to `speech_rate`.
#[tokio::test(flavor = "multi_thread")]
async fn set_speed_accepts_inclusive_bounds_and_persists() {
    let t = build_test_app();

    for speed in [0.5_f32, 1.0, 2.0, 3.0] {
        set_speed(t.state(), speed).await.unwrap();
    }

    let speeds: Vec<f32> = t
        .player
        .calls()
        .iter()
        .filter_map(|c| match c {
            PlayerCall::SetSpeed(s) => Some(*s),
            _ => None,
        })
        .collect();
    assert_eq!(speeds, vec![0.5, 1.0, 2.0, 3.0]);
    assert_eq!(t.state().storage.load_config().unwrap().speech_rate, 3.0);
}

/// Speeds outside [0.5, 3.0] are rejected with `config_error` (not
/// clamped) and never reach the player.
#[tokio::test(flavor = "multi_thread")]
async fn set_speed_rejects_out_of_range() {
    let t = build_test_app();

    for speed in [0.499_999_f32, 3.000_001, -1.0] {
        let err = set_speed(t.state(), speed).await.unwrap_err();
        assert!(
            matches!(err, CommandError::ConfigError { .. }),
            "speed {speed} must be rejected with config_error, got {err:?}"
        );
    }
    assert!(
        t.player
            .calls()
            .iter()
            .all(|c| !matches!(c, PlayerCall::SetSpeed(_)))
    );
}

/// The real `set_volume` command accepts the documented inclusive range
/// [0.0, 1.0] and forwards each value to the player.
#[tokio::test(flavor = "multi_thread")]
async fn set_volume_accepts_inclusive_bounds() {
    let t = build_test_app();

    for volume in [0.0_f32, 0.5, 1.0] {
        set_volume(t.state(), volume).await.unwrap();
    }

    let volumes: Vec<f32> = t
        .player
        .calls()
        .iter()
        .filter_map(|c| match c {
            PlayerCall::SetVolume(v) => Some(*v),
            _ => None,
        })
        .collect();
    assert_eq!(volumes, vec![0.0, 0.5, 1.0]);
}

/// Volumes outside [0.0, 1.0] are rejected with `config_error` (not
/// clamped) and never reach the player.
#[tokio::test(flavor = "multi_thread")]
async fn set_volume_rejects_out_of_range() {
    let t = build_test_app();

    for volume in [-0.000_001_f32, 1.000_001, -1.0] {
        let err = set_volume(t.state(), volume).await.unwrap_err();
        assert!(
            matches!(err, CommandError::ConfigError { .. }),
            "volume {volume} must be rejected with config_error, got {err:?}"
        );
    }
    assert!(
        t.player
            .calls()
            .iter()
            .all(|c| !matches!(c, PlayerCall::SetVolume(_)))
    );
}
