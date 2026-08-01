//! Test harness for Tauri commands and events (issue #103).
//!
//! Builds a real [`tauri::App`] on the `MockRuntime` (no display, no webview
//! process) with the production command set ([`crate::invoke_handler`]) and a
//! fully managed [`AppState`] whose external pieces are fakes:
//!
//! - **storage**: real [`StorageService`] over a [`TempDir`] — real JSON
//!   persistence, zero filesystem pollution;
//! - **tts**: [`StubEngine`] behind a real [`EngineSwitcher`], so
//!   `update_config` exercises the genuine switch logic;
//! - **emitter**: the supervisor's `recording_emitter`;
//! - **player**: [`FakePlayer`] — records every control call and scripts
//!   `position_sec()` polls, so the position-emitter loop (ticks, EOF →
//!   `playback_finished`) runs without mpv or a window;
//! - **tray channel**: absent (`tray_cmd_tx: None`).
//!
//! Two usage styles, both shown in the proof tests at the bottom:
//!
//! 1. call a command handler directly with `app.state::<AppState>()`;
//! 2. go through the IPC router with [`tauri::test::get_ipc_response`], which
//!    additionally pins command-name routing and the JSON wire contract.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex as ParkingMutex;
use serde_json::Value;
use tauri::test::{MockRuntime, mock_builder, mock_context, noop_assets};
use tauri::{App, Listener, Manager};
use tempfile::TempDir;

use crate::pipeline::TTSPipeline;
use crate::player::{PlayerBackend, Result as PlayerResult};
use crate::state::AppState;
use crate::storage::service::StorageService;
use crate::tts::engine::EngineKind;
use crate::tts::supervisor::test_helpers::recording_emitter;
use crate::tts::{CharMappingEntry, EngineSwitcher, SynthesizeOutput, TtsEngine, TtsError};

// The polling helper lives in `tts::supervisor::test_helpers`, shared with
// the integration tests under `tests/`; re-exported so command tests import
// the whole harness from one place.
pub use crate::tts::supervisor::test_helpers::wait_until;

// ---------------------------------------------------------------------------
// StubEngine
// ---------------------------------------------------------------------------

/// Minimal in-memory TTS engine. `synthesize` writes a placeholder file to
/// `out_wav` (the synthesis pipeline only needs *some* bytes there; the Opus
/// transcode is best-effort and keeps the WAV on failure) and returns a
/// fixed-duration output. Never touches the network, ONNX, or a subprocess.
///
/// Two knobs steer `synthesize` off the happy path, both settable after the
/// app is built (via [`TestApp::engine`]):
///
/// - [`StubEngine::fail_with`]: every call fails with a `TtsError` carrying
///   the message — drives the TTS-failure event path (`tts_error`).
/// - [`StubEngine::block_synthesis`] / [`StubEngine::release_synthesis`]:
///   calls park on a one-shot gate until released, giving tests a
///   deterministic "synthesis in flight" window for `cancel_synthesis` and
///   `regenerate_entry`-rejection scenarios.
#[derive(Default)]
pub struct StubEngine {
    fail_with: ParkingMutex<Option<String>>,
    gate: ParkingMutex<Option<Arc<tokio::sync::Notify>>>,
}

impl StubEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Make every subsequent `synthesize` call fail with `message`.
    pub fn fail_with(&self, message: &str) {
        *self.fail_with.lock() = Some(message.to_string());
    }

    /// Block subsequent `synthesize` calls until [`StubEngine::release_synthesis`].
    pub fn block_synthesis(&self) {
        *self.gate.lock() = Some(Arc::new(tokio::sync::Notify::new()));
    }

    /// Unblock every `synthesize` call parked since [`StubEngine::block_synthesis`].
    pub fn release_synthesis(&self) {
        if let Some(gate) = self.gate.lock().take() {
            gate.notify_waiters();
        }
    }
}

#[async_trait]
impl TtsEngine for StubEngine {
    fn kind(&self) -> EngineKind {
        EngineKind::Piper
    }

    async fn warmup(&self) -> Result<(), TtsError> {
        Ok(())
    }

    async fn spawn_initial_warmup(&self) {}

    async fn synthesize(
        &self,
        _text: String,
        _voice: String,
        _sample_rate: u32,
        out_wav: String,
        _char_mapping: Option<Vec<CharMappingEntry>>,
    ) -> Result<SynthesizeOutput, TtsError> {
        let gate = self.gate.lock().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        if let Some(message) = self.fail_with.lock().clone() {
            return Err(TtsError::Ttsd {
                code: "stub_failure".to_string(),
                message,
            });
        }
        std::fs::write(&out_wav, b"stub audio").map_err(TtsError::Ipc)?;
        Ok(SynthesizeOutput {
            timestamps: Vec::new(),
            duration_sec: 1.0,
        })
    }

    async fn shutdown(&self) -> Result<(), TtsError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// FakePlayer
// ---------------------------------------------------------------------------

/// A playback-control call received by [`FakePlayer`], in receive order.
/// Only the calls tests assert on are recorded; `pause`/`resume`,
/// `ensure_mpv_alive` and `mark_destroyed` update the tracked flags but
/// leave no trace here. Floats are stored raw; compare with epsilon where
/// it matters.
#[derive(Debug, Clone)]
pub enum PlayerCall {
    Load(PathBuf, String),
    Play,
    Stop,
    Seek(f64),
    SetSpeed(f32),
    SetVolume(f32),
}

/// Test double for [`PlayerBackend`].
///
/// Records the control calls tests assert on into [`FakePlayer::calls`] and
/// tracks the playing/loaded flags the way the real `Player` does. Unlike
/// the real player it emits **no** Tauri events (`playback_started` etc.) —
/// it holds no `AppHandle`; tests assert on recorded calls and on the
/// position emitter's own output instead.
///
/// `position_sec()` answers from a scripted queue (see
/// [`FakePlayer::script_positions`]); once the script is exhausted it returns
/// `None`, which — with a known `duration_sec` — the position emitter treats
/// as EOF. This is the intended way to drive the EOF → `playback_finished`
/// path.
#[derive(Default)]
pub struct FakePlayer {
    inner: ParkingMutex<FakeInner>,
    destroyed: AtomicBool,
}

#[derive(Default)]
struct FakeInner {
    calls: Vec<PlayerCall>,
    current_entry_id: Option<String>,
    is_playing: bool,
    duration_sec: Option<f64>,
    positions: VecDeque<Option<f64>>,
}

impl FakePlayer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of the recorded control calls, in receive order.
    pub fn calls(&self) -> Vec<PlayerCall> {
        self.inner.lock().calls.clone()
    }

    /// What `duration_sec()` reports from now on.
    pub fn set_duration_sec(&self, duration_sec: Option<f64>) {
        self.inner.lock().duration_sec = duration_sec;
    }

    /// Queue the values returned by successive `position_sec()` polls.
    pub fn script_positions(&self, positions: impl IntoIterator<Item = Option<f64>>) {
        self.inner.lock().positions = positions.into_iter().collect();
    }
}

impl PlayerBackend for FakePlayer {
    fn load(&self, path: &std::path::Path, entry_id: String) -> PlayerResult<()> {
        let mut inner = self.inner.lock();
        inner
            .calls
            .push(PlayerCall::Load(path.to_path_buf(), entry_id.clone()));
        inner.current_entry_id = Some(entry_id);
        inner.is_playing = false;
        Ok(())
    }

    fn play(&self) -> PlayerResult<()> {
        let mut inner = self.inner.lock();
        inner.calls.push(PlayerCall::Play);
        inner.is_playing = true;
        Ok(())
    }

    fn pause(&self) -> PlayerResult<()> {
        self.inner.lock().is_playing = false;
        Ok(())
    }

    fn resume(&self) -> PlayerResult<()> {
        self.inner.lock().is_playing = true;
        Ok(())
    }

    fn stop(&self) -> PlayerResult<()> {
        let mut inner = self.inner.lock();
        inner.calls.push(PlayerCall::Stop);
        inner.is_playing = false;
        Ok(())
    }

    fn seek(&self, position_sec: f64) -> PlayerResult<()> {
        self.inner.lock().calls.push(PlayerCall::Seek(position_sec));
        Ok(())
    }

    fn set_speed(&self, speed: f32) -> PlayerResult<()> {
        self.inner.lock().calls.push(PlayerCall::SetSpeed(speed));
        Ok(())
    }

    fn set_volume(&self, volume: f32) -> PlayerResult<()> {
        self.inner.lock().calls.push(PlayerCall::SetVolume(volume));
        Ok(())
    }

    fn position_sec(&self) -> Option<f64> {
        self.inner.lock().positions.pop_front().unwrap_or(None)
    }

    fn duration_sec(&self) -> Option<f64> {
        self.inner.lock().duration_sec
    }

    fn current_entry_id(&self) -> Option<String> {
        self.inner.lock().current_entry_id.clone()
    }

    fn is_playing(&self) -> bool {
        self.inner.lock().is_playing
    }

    fn ensure_mpv_alive(&self) -> PlayerResult<()> {
        Ok(())
    }

    fn mark_destroyed(&self) {
        self.destroyed.store(true, Ordering::SeqCst);
    }

    fn clear_is_playing(&self) {
        self.inner.lock().is_playing = false;
    }

    fn seek_suppression_active(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Event recording
// ---------------------------------------------------------------------------

/// Shared log of `(event_name, payload)` pairs.
pub type EventLog = Arc<Mutex<Vec<(String, Value)>>>;

/// Record every emission of the given events into a shared log.
///
/// `App::listen_any` subscribes per event name (Tauri has no wildcard
/// listener), so pass the names the test cares about. Rust-side `app.emit`
/// reaches these listeners synchronously, no webview needed.
pub fn record_events(app: &App<MockRuntime>, event_names: &[&str]) -> EventLog {
    let log: EventLog = Arc::new(Mutex::new(Vec::new()));
    for name in event_names {
        let log = Arc::clone(&log);
        let name_owned = name.to_string();
        app.listen_any(*name, move |event| {
            let payload = serde_json::from_str(event.payload()).unwrap_or(Value::Null);
            log.lock().unwrap().push((name_owned.clone(), payload));
        });
    }
    log
}

// ---------------------------------------------------------------------------
// App builder
// ---------------------------------------------------------------------------

/// A mock-runtime app with a fully managed [`AppState`], plus handles to the
/// fakes and the TempDir guards (dropping them would delete the storage
/// dirs mid-test).
pub struct TestApp {
    pub app: App<MockRuntime>,
    pub player: Arc<FakePlayer>,
    /// The stub TTS engine behind the switcher — steer `synthesize` via its
    /// knobs (`fail_with`, `block_synthesis`).
    pub engine: Arc<StubEngine>,
    _storage_dir: TempDir,
    _voices_dir: TempDir,
    _ttsd_dir: TempDir,
    _bundle_dir: TempDir,
}

impl TestApp {
    /// Convenience accessor for the managed state.
    pub fn state(&self) -> tauri::State<'_, AppState> {
        self.app.state::<AppState>()
    }
}

/// Build the test app: production command set, managed `AppState` with fakes.
pub fn build_test_app() -> TestApp {
    build_test_app_with_kind(EngineKind::Piper)
}

/// Build the test app with the `EngineSwitcher` reporting the given engine
/// kind. Only the switcher's atomic kind is affected — the stub engine behind
/// it is unchanged, so synthesis behavior stays stub-driven.
pub fn build_test_app_with_kind(kind: EngineKind) -> TestApp {
    let app = mock_builder()
        .invoke_handler(crate::invoke_handler())
        .build(mock_context(noop_assets()))
        .expect("failed to build mock app");

    let storage_dir = TempDir::new().expect("storage tempdir");
    let storage = Arc::new(
        StorageService::with_cache_dir(storage_dir.path().to_path_buf()).expect("storage service"),
    );
    let voices_dir = TempDir::new().expect("voices tempdir");
    let ttsd_dir = TempDir::new().expect("ttsd tempdir");
    let bundle_dir = TempDir::new().expect("bundle tempdir");

    let (emitter, _engine_events) = recording_emitter();

    let engine = Arc::new(StubEngine::new());
    let stub: Arc<dyn TtsEngine> = engine.clone();
    let switcher = Arc::new(EngineSwitcher::new(
        Arc::clone(&stub),
        kind,
        Some("stub-voice".to_string()),
        voices_dir.path().to_path_buf(),
        ttsd_dir.path().to_path_buf(),
        bundle_dir.path().to_path_buf(),
        Arc::clone(&emitter),
    ));
    let player = Arc::new(FakePlayer::new());

    app.manage(AppState {
        storage,
        tts: switcher.clone(),
        engine_switcher: switcher,
        ttsd_dir: ttsd_dir.path().to_path_buf(),
        piper_voices_dir: voices_dir.path().to_path_buf(),
        silero_native_bundle_dir: bundle_dir.path().to_path_buf(),
        emitter,
        player: player.clone(),
        pipeline: Arc::new(ParkingMutex::new(TTSPipeline::new())),
        tray_cmd_tx: None,
        user_quit: Arc::new(AtomicBool::new(false)),
        synthesis_tasks: Arc::new(ParkingMutex::new(std::collections::HashMap::new())),
        synthesize_entered: Arc::new(ParkingMutex::new(std::collections::HashSet::new())),
    });

    TestApp {
        app,
        player,
        engine,
        _storage_dir: storage_dir,
        _voices_dir: voices_dir,
        _ttsd_dir: ttsd_dir,
        _bundle_dir: bundle_dir,
    }
}

/// Write a silero-native manifest + payload files into `dir` so the
/// stat-only bundle probe passes. Sizes and hashes are honest, though the
/// probe itself only checks presence and size.
pub fn write_fake_bundle(dir: &std::path::Path, files: &[(&str, &[u8])]) {
    use sha2::{Digest, Sha256};
    let entries: Vec<serde_json::Value> = files
        .iter()
        .map(|(name, contents)| {
            std::fs::write(dir.join(name), contents).unwrap();
            serde_json::json!({
                "path": name,
                "size": contents.len(),
                "sha256": format!("{:x}", Sha256::new().chain_update(contents).finalize()),
            })
        })
        .collect();
    let manifest = serde_json::json!({
        "model_id": "test",
        "opset": 17,
        "export_date_utc": "2026-01-01T00:00:00+00:00",
        "files": entries,
    });
    std::fs::write(dir.join("manifest.json"), manifest.to_string()).unwrap();
}

// ---------------------------------------------------------------------------
// Proof tests
// ---------------------------------------------------------------------------

mod tests {
    use super::*;

    /// Style (a): direct handler call. `get_entries` on a fresh app returns
    /// the empty history — proves a command body runs against a managed
    /// `AppState` under `MockRuntime`, no webview involved.
    #[tokio::test(flavor = "multi_thread")]
    async fn get_entries_direct_call_returns_empty_list() {
        let t = build_test_app();
        let entries = crate::commands::get_entries(t.state()).await.unwrap();
        assert!(entries.is_empty());
    }

    /// Style (b): full IPC path. `WebviewWindowBuilder` under `MockRuntime`
    /// plus `get_ipc_response` pins command-name routing and the JSON wire
    /// contract (`[]` for an empty history).
    #[tokio::test(flavor = "multi_thread")]
    async fn get_entries_over_ipc_returns_empty_json_list() {
        let t = build_test_app();
        let webview = tauri::WebviewWindowBuilder::new(&t.app, "main", Default::default())
            .build()
            .expect("mock webview window");

        let res = tauri::test::get_ipc_response(
            &webview,
            tauri::webview::InvokeRequest {
                cmd: "get_entries".into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                // Local origin differs by platform: Windows/Android serve the
                // app from http://tauri.localhost, elsewhere it's the
                // tauri:// custom protocol. A non-local URL would trip the
                // remote-origin ACL check and get the command rejected.
                url: if cfg!(any(windows, target_os = "android")) {
                    "http://tauri.localhost"
                } else {
                    "tauri://localhost"
                }
                .parse()
                .unwrap(),
                body: tauri::ipc::InvokeBody::default(),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.to_string(),
            },
        )
        .map(|b| {
            b.deserialize::<Vec<crate::storage::schema::TextEntry>>()
                .unwrap()
        });

        let entries = res.expect("get_entries IPC call failed");
        assert!(entries.is_empty());
    }

    /// The position-emitter loop (production code in `player`) driven by a
    /// scripted [`FakePlayer`]: while playing, each 100 ms tick pops one
    /// scripted position into a `playback_position` event; once the script is
    /// exhausted `position_sec()` yields `None`, which — with a known
    /// duration — the loop treats as EOF and closes out with
    /// `playback_finished` + `playback_stopped`, clearing the playing flag.
    #[tokio::test(flavor = "multi_thread")]
    async fn position_emitter_ticks_then_finishes_at_eof() {
        let t = build_test_app();
        t.player
            .load(std::path::Path::new("/audio.wav"), "entry-1".to_string())
            .unwrap();
        t.player.play().unwrap();
        t.player.set_duration_sec(Some(1.0));
        t.player.script_positions([Some(0.1), Some(0.2), Some(0.3)]);

        let events = record_events(
            &t.app,
            &["playback_position", "playback_finished", "playback_stopped"],
        );
        crate::player::spawn_position_emitter(t.player.clone(), t.app.handle().clone());

        wait_until("playback_finished", Duration::from_secs(5), || {
            events
                .lock()
                .unwrap()
                .iter()
                .any(|(name, _)| name == "playback_finished")
        })
        .await;

        {
            let log = events.lock().unwrap();
            let positions: Vec<f64> = log
                .iter()
                .filter(|(name, _)| name == "playback_position")
                .map(|(_, payload)| payload["position_sec"].as_f64().unwrap())
                .collect();
            assert_eq!(positions, vec![0.1, 0.2, 0.3]);
            assert!(
                log.iter()
                    .all(|(_, payload)| payload.get("entry_id").is_none_or(|id| id == "entry-1"))
            );

            // EOF ordering: playback_finished immediately before playback_stopped.
            let finished_idx = log
                .iter()
                .position(|(name, _)| name == "playback_finished")
                .unwrap();
            assert_eq!(log[finished_idx + 1].0, "playback_stopped");
            assert_eq!(
                log[finished_idx].1,
                serde_json::json!({ "entry_id": "entry-1" })
            );
        }

        // The loop cleared the flag, so playback_finished fires exactly once.
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(!t.player.is_playing());
        let log = events.lock().unwrap();
        assert_eq!(
            log.iter()
                .filter(|(name, _)| name == "playback_finished")
                .count(),
            1
        );
    }
}
