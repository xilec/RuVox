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

// The harness is scaffold (issue #103): the proof tests at the bottom
// exercise only part of its API — the rest (`record_events`, `wait_until`,
// `FakePlayer` scripting, `TestApp` fields) is consumed by the follow-up
// command/event test issues.
#![allow(dead_code)]

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use parking_lot::Mutex as ParkingMutex;
use serde_json::Value;
use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
use tauri::{App, Listener, Manager};
use tempfile::TempDir;

use crate::pipeline::TTSPipeline;
use crate::player::{PlayerBackend, Result as PlayerResult};
use crate::state::AppState;
use crate::storage::service::StorageService;
use crate::tts::engine::EngineKind;
use crate::tts::supervisor::test_helpers::recording_emitter;
use crate::tts::{CharMappingEntry, EngineSwitcher, SynthesizeOutput, TtsEngine, TtsError};

// ---------------------------------------------------------------------------
// StubEngine
// ---------------------------------------------------------------------------

/// Minimal in-memory TTS engine. `synthesize` writes a placeholder file to
/// `out_wav` (the synthesis pipeline only needs *some* bytes there; the Opus
/// transcode is best-effort and keeps the WAV on failure) and returns a
/// fixed-duration output. Never touches the network, ONNX, or a subprocess.
pub struct StubEngine;

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
/// Floats are stored raw; compare with epsilon where it matters.
#[derive(Debug, Clone)]
pub enum PlayerCall {
    Load(PathBuf, String),
    Play,
    Pause,
    Resume,
    Stop,
    Seek(f64),
    SetSpeed(f32),
    SetVolume(f32),
    EnsureMpvAlive,
    MarkDestroyed,
}

/// Test double for [`PlayerBackend`].
///
/// Records every control call into [`FakePlayer::calls`] and tracks the
/// playing/loaded flags the way the real `Player` does. Unlike the real
/// player it emits **no** Tauri events (`playback_started` etc.) — it holds
/// no `AppHandle`; tests assert on recorded calls and on the position
/// emitter's own output instead.
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
        let mut inner = self.inner.lock();
        inner.calls.push(PlayerCall::Pause);
        inner.is_playing = false;
        Ok(())
    }

    fn resume(&self) -> PlayerResult<()> {
        let mut inner = self.inner.lock();
        inner.calls.push(PlayerCall::Resume);
        inner.is_playing = true;
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
        self.inner.lock().calls.push(PlayerCall::EnsureMpvAlive);
        Ok(())
    }

    fn mark_destroyed(&self) {
        self.inner.lock().calls.push(PlayerCall::MarkDestroyed);
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
    /// Events emitted through the state's `emitter` (engine layer).
    pub engine_events: EventLog,
    _storage_dir: TempDir,
    _voices_dir: TempDir,
    _ttsd_dir: TempDir,
}

impl TestApp {
    /// Convenience accessor for the managed state.
    pub fn state(&self) -> tauri::State<'_, AppState> {
        self.app.state::<AppState>()
    }
}

/// Build the test app: production command set, managed `AppState` with fakes.
pub fn build_test_app() -> TestApp {
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

    let (emitter, engine_events) = recording_emitter();

    let stub: Arc<dyn TtsEngine> = Arc::new(StubEngine);
    let switcher = Arc::new(EngineSwitcher::new(
        Arc::clone(&stub),
        EngineKind::Piper,
        Some("stub-voice".to_string()),
        // No Silero supervisor in tests: `kill_current_ttsd` degrades to a
        // no-op, which is all the harness needs.
        None,
        voices_dir.path().to_path_buf(),
        ttsd_dir.path().to_path_buf(),
        Arc::clone(&emitter),
    ));
    let player = Arc::new(FakePlayer::new());

    app.manage(AppState {
        storage,
        tts: switcher.clone(),
        engine_switcher: switcher,
        ttsd_dir: ttsd_dir.path().to_path_buf(),
        piper_voices_dir: voices_dir.path().to_path_buf(),
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
        engine_events,
        _storage_dir: storage_dir,
        _voices_dir: voices_dir,
        _ttsd_dir: ttsd_dir,
    }
}

// ---------------------------------------------------------------------------
// Async waiting
// ---------------------------------------------------------------------------

/// Poll `predicate` (every 10 ms) until it holds or `timeout` elapses.
///
/// Background synthesis runs via `tokio::spawn`, so a command returning
/// `Ok` does *not* mean the side effects (entry status flip, events) have
/// landed yet — tests must await them. Panics naming `what` on timeout.
pub async fn wait_until<F>(what: &str, timeout: Duration, mut predicate: F)
where
    F: FnMut() -> bool,
{
    let start = Instant::now();
    loop {
        if predicate() {
            return;
        }
        assert!(
            start.elapsed() < timeout,
            "timed out after {timeout:?} waiting for {what}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
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
}
