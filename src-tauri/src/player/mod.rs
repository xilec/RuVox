//! Audio player backed by tauri-plugin-mpv.
//!
//! # Crate choice rationale
//!
//! Two candidates were evaluated:
//!
//! - **tauri-plugin-mpv v0.5.2** (github.com/nini22P/tauri-plugin-mpv, MPL-2.0)
//!   Controls an mpv subprocess via JSON IPC. Supports arbitrary mpv commands
//!   including `get_property "time-pos"` for polling, and passes custom CLI
//!   arguments (`--no-video`, `--af=scaletempo2`) at process init time.
//!   Integrates cleanly with the Tauri AppHandle we already hold.
//!
//! - **libmpv2 v5.0.3** (github.com/kohsine/libmpv-rs, LGPL-2.1)
//!   Direct bindings to the libmpv C API. More powerful (observe_property,
//!   event loop), but pulls in unsafe C FFI and would conflict with mpv-unwrapped
//!   linking on Nix without additional care.
//!
//! **Decision: tauri-plugin-mpv.**
//! Position polling via `get_property` IPC round-trips every 100 ms is
//! adequate for word-highlight sync.

use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use serde_json::json;
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_mpv::{MpvCommand, MpvConfig, MpvExt};
use tokio::time::{interval, Duration};
use tracing::{debug, warn};

pub const WINDOW_LABEL: &str = "main";

#[derive(Debug, thiserror::Error)]
pub enum PlayerError {
    #[error("mpv init failed: {0}")]
    Init(String),
    #[error("mpv operation failed: {0}")]
    Op(String),
    #[error("file not found: {0}")]
    FileNotFound(String),
}

pub type Result<T> = std::result::Result<T, PlayerError>;

// ---------------------------------------------------------------------------
// Internal state (behind a Mutex so Player can be Arc<Player>+Send+Sync)
// ---------------------------------------------------------------------------

struct State {
    current_entry_id: Option<String>,
    /// Cached duration in seconds, set after loadfile + get_property duration.
    duration_sec: Option<f64>,
    /// True while mpv is not paused and not stopped.
    is_playing: bool,
    /// While `Some(deadline)` and `Instant::now() < deadline`, the position
    /// emitter skips its periodic `playback_position` emit.  Set by `seek()`
    /// to mask the race where mpv hasn't processed the seek yet but the
    /// 100 ms emitter tick would still report the pre-seek `time-pos`.
    seek_suppress_until: Option<Instant>,
}

// ---------------------------------------------------------------------------
// Public Player struct
// ---------------------------------------------------------------------------

pub struct Player<R: Runtime> {
    app: AppHandle<R>,
    state: Mutex<State>,
    /// False after mpv is destroyed (on exit). All subsequent commands
    /// short-circuit to avoid tauri-plugin-mpv's internal unwrap() panicking
    /// when the instance has been removed from its map.
    mpv_alive: AtomicBool,
}

impl<R: Runtime> Player<R> {
    /// Create and initialise a new Player.
    ///
    /// Starts an mpv subprocess with `--af=scaletempo2` — pitch-correct
    /// speed scaling — and additionally enforces the audio filter at
    /// runtime, because tauri-plugin-mpv's arg forwarding has been observed
    /// to miss the filter chain on some setups, which results in speed
    /// changes raising pitch.
    pub fn new(app: AppHandle<R>) -> Result<Self> {
        Self::init_mpv(&app)?;

        let this = Self {
            app,
            state: Mutex::new(State {
                current_entry_id: None,
                duration_sec: None,
                is_playing: false,
                seek_suppress_until: None,
            }),
            mpv_alive: AtomicBool::new(true),
        };

        if let Err(e) = this.mpv_command(json!(["set_property", "af", "scaletempo2"])) {
            warn!("failed to set runtime af=scaletempo2: {e}");
        }

        debug!("mpv player initialised (scaletempo2, audio-only)");
        Ok(this)
    }

    /// Spawn the mpv subprocess with our standard config.  Used by both
    /// `new()` and `ensure_mpv_alive()` (re-init after the plugin destroys
    /// mpv on the main window's CloseRequested event).
    fn init_mpv(app: &AppHandle<R>) -> Result<()> {
        let config = MpvConfig {
            path: "mpv".to_string(),
            args: vec![
                "--no-video".to_string(),
                "--no-ytdl".to_string(),
                "--af=scaletempo2".to_string(),
            ],
            observed_properties: vec![],
            ipc_timeout_ms: 2000,
            show_mpv_output: false,
        };
        app.mpv()
            .init(config, WINDOW_LABEL)
            .map_err(|e| PlayerError::Init(e.to_string()))?;
        Ok(())
    }

    /// Re-initialise mpv if the subprocess is gone.  tauri-plugin-mpv has its
    /// own CloseRequested handler that always destroys mpv when the main
    /// window's close is requested — even if we hide the window instead of
    /// closing it for the tray-on-close UX.  Calling this method on
    /// window-show restores playback capability.  Idempotent.
    pub fn ensure_mpv_alive(&self) -> Result<()> {
        let alive = self.mpv_alive.load(Ordering::SeqCst);
        let in_map = self
            .app
            .mpv()
            .instances
            .lock()
            .map(|m| m.contains_key(WINDOW_LABEL))
            .unwrap_or(false);
        if alive && in_map {
            return Ok(());
        }

        // Reset state — the previous file is gone with mpv.
        {
            let mut s = self.state.lock();
            s.current_entry_id = None;
            s.duration_sec = None;
            s.is_playing = false;
            s.seek_suppress_until = None;
        }

        Self::init_mpv(&self.app)?;
        self.mpv_alive.store(true, Ordering::SeqCst);

        if let Err(e) = self.mpv_command(json!(["set_property", "af", "scaletempo2"])) {
            warn!("failed to set af=scaletempo2 after reinit: {e}");
        }

        // Tell the frontend that any cached player state (current entry,
        // position, duration) is stale.
        let _ = self.app.emit("playback_stopped", json!({}));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Playback control
    // -----------------------------------------------------------------------

    /// Load a WAV file and associate it with `entry_id`.
    /// Does not start playback automatically — call [`play`] after.
    pub fn load(&self, path: &Path, entry_id: String) -> Result<()> {
        if !path.exists() {
            return Err(PlayerError::FileNotFound(
                path.to_string_lossy().into_owned(),
            ));
        }

        let path_str = path.to_string_lossy().into_owned();

        self.mpv_command(json!(["loadfile", path_str]))?;
        // Pause immediately after load so the caller can call play() explicitly.
        self.mpv_command(json!(["set_property", "pause", true]))?;

        {
            let mut s = self.state.lock();
            s.current_entry_id = Some(entry_id);
            s.duration_sec = None;
            s.is_playing = false;
        }

        // Attempt to read duration; mpv may not have it ready yet — that is
        // fine, spawn_position_emitter will update it on the first poll.
        if let Ok(dur) = self.read_property_f64("duration") {
            self.state.lock().duration_sec = Some(dur);
        }

        Ok(())
    }

    /// Start (or resume) playback.
    pub fn play(&self) -> Result<()> {
        self.mpv_command(json!(["set_property", "pause", false]))?;
        let (entry_id, duration_sec) = {
            let mut s = self.state.lock();
            s.is_playing = true;
            (s.current_entry_id.clone(), s.duration_sec)
        };
        if let Some(id) = entry_id {
            let _ = self.app.emit(
                "playback_started",
                json!({ "entry_id": id, "duration_sec": duration_sec }),
            );
        }
        Ok(())
    }

    /// Pause playback.
    pub fn pause(&self) -> Result<()> {
        self.mpv_command(json!(["set_property", "pause", true]))?;
        let pos = self.position_sec().unwrap_or(0.0);
        let entry_id = {
            let mut s = self.state.lock();
            s.is_playing = false;
            s.current_entry_id.clone()
        };
        if let Some(id) = entry_id {
            let _ = self.app.emit(
                "playback_paused",
                json!({ "entry_id": id, "position_sec": pos }),
            );
        }
        Ok(())
    }

    /// Resume from a paused state.  Emits `playback_started` so that the UI
    /// flips the play/pause toggle back to "pause" (frontend state is in
    /// terms of events, not idempotent flags).
    pub fn resume(&self) -> Result<()> {
        self.mpv_command(json!(["set_property", "pause", false]))?;
        let (entry_id, duration_sec) = {
            let mut s = self.state.lock();
            s.is_playing = true;
            (s.current_entry_id.clone(), s.duration_sec)
        };
        if let Some(id) = entry_id {
            let _ = self.app.emit(
                "playback_started",
                json!({ "entry_id": id, "duration_sec": duration_sec }),
            );
        }
        Ok(())
    }

    /// Stop playback and clear the loaded file.
    pub fn stop(&self) -> Result<()> {
        self.mpv_command(json!(["stop"]))?;
        self.state.lock().is_playing = false;
        let _ = self.app.emit("playback_stopped", json!({}));
        Ok(())
    }

    /// Seek to an absolute position (seconds).
    ///
    /// mpv executes `seek` asynchronously: the IPC call returns before
    /// `time-pos` reflects the new position, so the next position-emitter
    /// tick would otherwise report the pre-seek value and snap the UI
    /// thumb back.  To avoid that we (a) immediately emit a
    /// `playback_position` event with the target so the frontend syncs
    /// without waiting for a tick, and (b) suppress emitter ticks for a
    /// short window to hide the stale `time-pos` poll.
    pub fn seek(&self, position_sec: f64) -> Result<()> {
        self.mpv_command(json!(["seek", position_sec, "absolute"]))?;
        let (entry_id, duration_sec) = {
            let mut s = self.state.lock();
            s.seek_suppress_until = Some(Instant::now() + Duration::from_millis(300));
            (s.current_entry_id.clone(), s.duration_sec)
        };
        if let Some(id) = entry_id {
            let _ = self.app.emit(
                "playback_position",
                json!({
                    "position_sec": position_sec,
                    "entry_id": id,
                    "duration_sec": duration_sec,
                }),
            );
        }
        Ok(())
    }

    /// Set playback speed (0.5–2.0). scaletempo2 keeps pitch correct.
    pub fn set_speed(&self, speed: f32) -> Result<()> {
        self.mpv_command(json!(["set_property", "speed", speed]))?;
        Ok(())
    }

    /// Set volume (0.0–1.0 → mpv 0–100).
    pub fn set_volume(&self, volume: f32) -> Result<()> {
        let mpv_vol = (volume.clamp(0.0, 1.0) * 100.0) as f64;
        self.mpv_command(json!(["set_property", "volume", mpv_vol]))?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // State queries
    // -----------------------------------------------------------------------

    /// Current playback position in seconds, or `None` if nothing is loaded.
    pub fn position_sec(&self) -> Option<f64> {
        self.read_property_f64("time-pos").ok()
    }

    /// Total duration in seconds, or `None` if not yet available.
    pub fn duration_sec(&self) -> Option<f64> {
        // Try the cached value first; if absent, query mpv.
        let cached = self.state.lock().duration_sec;
        if cached.is_some() {
            return cached;
        }
        if let Ok(d) = self.read_property_f64("duration") {
            self.state.lock().duration_sec = Some(d);
            return Some(d);
        }
        None
    }

    /// Returns the `entry_id` of the currently loaded entry, or `None`.
    pub fn current_entry_id(&self) -> Option<String> {
        self.state.lock().current_entry_id.clone()
    }

    /// True if mpv is actively playing (not paused, not stopped).
    pub fn is_playing(&self) -> bool {
        self.state.lock().is_playing
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Mark the player as destroyed so subsequent mpv commands short-circuit
    /// instead of triggering tauri-plugin-mpv's internal unwrap() panic.
    pub fn mark_destroyed(&self) {
        self.mpv_alive.store(false, Ordering::SeqCst);
    }

    fn mpv_command(&self, command: serde_json::Value) -> Result<()> {
        if !self.mpv_alive.load(Ordering::SeqCst) {
            return Err(PlayerError::Op("mpv instance destroyed".into()));
        }
        let cmd = MpvCommand {
            command: command.as_array().cloned().unwrap_or_default(),
            request_id: None,
        };
        // tauri-plugin-mpv panics with `Option::unwrap()` in its command
        // dispatcher when the instance has been removed from its map (e.g.
        // its own CloseRequested handler destroyed mpv).  Catch that, mark
        // mpv as dead so subsequent calls short-circuit, and surface as Err.
        let app = self.app.clone();
        let result =
            std::panic::catch_unwind(AssertUnwindSafe(|| app.mpv().command(cmd, WINDOW_LABEL)));
        match result {
            Err(_) => {
                self.mpv_alive.store(false, Ordering::SeqCst);
                Err(PlayerError::Op(
                    "mpv command panicked (instance gone?)".into(),
                ))
            }
            Ok(Err(e)) => Err(PlayerError::Op(e.to_string())),
            Ok(Ok(_)) => Ok(()),
        }
    }

    fn read_property_f64(&self, property: &str) -> Result<f64> {
        if !self.mpv_alive.load(Ordering::SeqCst) {
            return Err(PlayerError::Op("mpv instance destroyed".into()));
        }
        let cmd = MpvCommand {
            command: vec![json!("get_property"), json!(property)],
            request_id: Some(1),
        };
        let app = self.app.clone();
        let response = match std::panic::catch_unwind(AssertUnwindSafe(|| {
            app.mpv().command(cmd, WINDOW_LABEL)
        })) {
            Err(_) => {
                self.mpv_alive.store(false, Ordering::SeqCst);
                return Err(PlayerError::Op(
                    "mpv command panicked (instance gone?)".into(),
                ));
            }
            Ok(r) => r.map_err(|e| PlayerError::Op(e.to_string()))?,
        };

        if response.error != "success" {
            return Err(PlayerError::Op(format!(
                "get_property {property} error: {}",
                response.error
            )));
        }

        response
            .data
            .as_ref()
            .and_then(|v| v.as_f64())
            .ok_or_else(|| PlayerError::Op(format!("get_property {property} returned non-f64")))
    }
}

impl<R: Runtime> Drop for Player<R> {
    fn drop(&mut self) {
        if let Err(e) = self.app.mpv().destroy(WINDOW_LABEL) {
            warn!("mpv destroy on drop: {e}");
        }
    }
}

// ---------------------------------------------------------------------------
// Pure position logic (extracted so it can be unit-tested without mpv)
// ---------------------------------------------------------------------------

/// Decide whether the emitter should treat the current poll as end-of-file.
///
/// EOF is true in either of two cases, both requiring a known duration:
///   1. `time-pos` is None — mpv unloaded the file / entered idle,
///   2. `time-pos` >= duration − `EOF_EPSILON_SEC`.
///
/// A None duration is never EOF: without a known length we cannot tell that
/// playback has ended, so we keep emitting positions.
/// Tolerance for treating `time-pos` as having reached `duration`: mpv's
/// position poll can lag the true end of file by a small margin.
const EOF_EPSILON_SEC: f64 = 0.2;

fn is_eof(pos: Option<f64>, duration: Option<f64>) -> bool {
    match (pos, duration) {
        (None, Some(_)) => true,
        (Some(p), Some(d)) if d > 0.0 && p >= d - EOF_EPSILON_SEC => true,
        _ => false,
    }
}

/// True while a seek-suppress window is still open: `deadline` is set and
/// `now` has not reached it yet.  A None deadline (or an expired one) means
/// suppression is over.
fn seek_suppressed(now: Instant, deadline: Option<Instant>) -> bool {
    matches!(deadline, Some(d) if now < d)
}

// ---------------------------------------------------------------------------
// Position emitter task
// ---------------------------------------------------------------------------

/// Spawns a Tokio task that emits `playback_position` every 100 ms while
/// something is playing.  Also detects EOF (position ≥ duration) and emits
/// `playback_finished` + `playback_stopped`.
pub fn spawn_position_emitter<R: Runtime + 'static>(player: Arc<Player<R>>, app: AppHandle<R>) {
    // tauri::async_runtime::spawn works inside the setup hook where the
    // bare tokio runtime context is not yet active.
    tauri::async_runtime::spawn(async move {
        let mut ticker = interval(Duration::from_millis(100));
        loop {
            ticker.tick().await;

            if !player.is_playing() {
                continue;
            }

            let entry_id = match player.current_entry_id() {
                Some(id) => id,
                None => continue,
            };

            let duration_sec = player.duration_sec();

            // EOF detection runs *before* the seek-suppress window so a file
            // that reaches its end inside the 300 ms window still triggers
            // `playback_finished` instead of leaving the UI stuck.
            let pos = player.position_sec();
            if is_eof(pos, duration_sec) {
                debug!("playback finished: entry_id={entry_id}");
                let _ = app.emit("playback_finished", json!({ "entry_id": entry_id }));
                let _ = app.emit("playback_stopped", json!({}));
                player.state.lock().is_playing = false;
                continue;
            }

            // Suppress emits while mpv is still catching up to a recent seek
            // target (see Player::seek for the rationale).  EOF above has
            // already been handled, so skipping ticks here is safe.
            let suppressed = {
                let mut s = player.state.lock();
                if seek_suppressed(Instant::now(), s.seek_suppress_until) {
                    true
                } else {
                    // Deadline passed: clear it so the check is cheap on
                    // subsequent ticks. Skip the write entirely when it's
                    // already None -- the common case once the window closes.
                    if s.seek_suppress_until.is_some() {
                        s.seek_suppress_until = None;
                    }
                    false
                }
            };
            if suppressed {
                continue;
            }

            let pos = match pos {
                Some(p) => p,
                None => continue,
            };

            // duration_sec() re-queries mpv when the cached value is None,
            // so sending it with every position tick auto-populates the
            // frontend slider as soon as mpv has parsed the file header.
            let _ = app.emit(
                "playback_position",
                json!({
                    "position_sec": pos,
                    "entry_id": entry_id,
                    "duration_sec": duration_sec,
                }),
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    /// Compile-time check: Player::new returns Result (does not panic at type
    /// level).  The body is never executed because we cannot build a real
    /// AppHandle in a unit test without a running Tauri app.
    fn _assert_new_returns_result<R: Runtime>(app: AppHandle<R>) -> Result<Player<R>> {
        Player::new(app)
    }

    /// Smoke: PlayerError variants format correctly.
    #[test_case(PlayerError::Init("test init".to_string()) => "mpv init failed: test init"; "init")]
    #[test_case(PlayerError::Op("test op".to_string()) => "mpv operation failed: test op"; "op")]
    #[test_case(PlayerError::FileNotFound("/tmp/x.wav".to_string()) => "file not found: /tmp/x.wav"; "file_not_found")]
    fn player_error_display(err: PlayerError) -> String {
        err.to_string()
    }

    // -- is_eof ------------------------------------------------------------

    /// `is_eof(pos, duration)` decides EOF from mpv's last position poll:
    /// - unknown duration → never EOF (cannot tell the file ended);
    /// - a zero-length duration is guarded off (`d > 0.0`);
    /// - otherwise EOF once `pos` is `None` (file unloaded) or within
    ///   `EOF_EPSILON_SEC` of the end.
    #[test_case(None, Some(10.0) => true; "none_pos_with_duration")]
    #[test_case(Some(9.8), Some(10.0) => true; "at_threshold")]
    #[test_case(Some(10.0), Some(10.0) => true; "at_duration")]
    #[test_case(Some(11.0), Some(10.0) => true; "past_duration")]
    #[test_case(Some(9.79), Some(10.0) => false; "just_before_threshold")]
    #[test_case(Some(0.0), Some(10.0) => false; "start_of_file")]
    #[test_case(None, None => false; "unknown_duration_no_pos")]
    #[test_case(Some(5.0), None => false; "unknown_duration_with_pos")]
    #[test_case(Some(0.0), Some(0.0) => false; "zero_duration")]
    fn is_eof_cases(pos: Option<f64>, duration: Option<f64>) -> bool {
        is_eof(pos, duration)
    }

    // -- seek_suppressed ---------------------------------------------------

    #[test]
    fn seek_suppressed_none_deadline_is_not_suppressed() {
        assert!(!seek_suppressed(Instant::now(), None));
    }

    #[test]
    fn seek_suppressed_before_deadline_is_suppressed() {
        let now = Instant::now();
        let deadline = now + Duration::from_millis(300);
        assert!(seek_suppressed(now, Some(deadline)));
    }

    #[test]
    fn seek_suppressed_at_deadline_is_not_suppressed() {
        // Boundary: `now < deadline` is strict, so now == deadline is over.
        let now = Instant::now();
        assert!(!seek_suppressed(now, Some(now)));
    }

    #[test]
    fn seek_suppressed_after_deadline_is_not_suppressed() {
        let deadline = Instant::now();
        let now = deadline + Duration::from_millis(1);
        assert!(!seek_suppressed(now, Some(deadline)));
    }
}
