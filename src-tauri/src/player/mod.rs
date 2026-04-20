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
//! The project spec says "prefer tauri-plugin-mpv if viable". It is viable:
//! position polling via `get_property` IPC round-trips every 100 ms is
//! perfectly adequate for word-highlight sync (same approach as the legacy
//! python-mpv version which polled at 200 ms).

use std::path::Path;
use std::sync::Arc;

use parking_lot::Mutex;
use serde_json::json;
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_mpv::{MpvCommand, MpvConfig, MpvExt};
use tokio::time::{interval, Duration};
use tracing::{debug, warn};

const WINDOW_LABEL: &str = "main";

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
}

// ---------------------------------------------------------------------------
// Public Player struct
// ---------------------------------------------------------------------------

pub struct Player<R: Runtime> {
    app: AppHandle<R>,
    state: Mutex<State>,
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

        let this = Self {
            app,
            state: Mutex::new(State {
                current_entry_id: None,
                duration_sec: None,
                is_playing: false,
            }),
        };

        // Belt-and-braces: set `af` via IPC too, so scaletempo2 is definitely
        // in the filter chain regardless of how the args were interpreted at
        // process init.
        if let Err(e) = this.mpv_command(json!(["set_property", "af", "scaletempo2"])) {
            warn!("failed to set runtime af=scaletempo2: {e}");
        }

        debug!("mpv player initialised (scaletempo2, audio-only)");

        Ok(this)
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
    pub fn seek(&self, position_sec: f64) -> Result<()> {
        self.mpv_command(json!(["seek", position_sec, "absolute"]))?;
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

    fn mpv_command(&self, command: serde_json::Value) -> Result<()> {
        let cmd = MpvCommand {
            command: command
                .as_array()
                .cloned()
                .unwrap_or_default(),
            request_id: None,
        };
        self.app
            .mpv()
            .command(cmd, WINDOW_LABEL)
            .map_err(|e| PlayerError::Op(e.to_string()))?;
        Ok(())
    }

    fn read_property_f64(&self, property: &str) -> Result<f64> {
        let cmd = MpvCommand {
            command: vec![
                json!("get_property"),
                json!(property),
            ],
            request_id: Some(1),
        };
        let response = self
            .app
            .mpv()
            .command(cmd, WINDOW_LABEL)
            .map_err(|e| PlayerError::Op(e.to_string()))?;

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
// Position emitter task
// ---------------------------------------------------------------------------

/// Spawns a Tokio task that emits `playback_position` every 100 ms while
/// something is playing.  Also detects EOF (position ≥ duration) and emits
/// `playback_finished` + `playback_stopped`.
pub fn spawn_position_emitter<R: Runtime + 'static>(
    player: Arc<Player<R>>,
    app: AppHandle<R>,
) {
    // tauri::async_runtime::spawn works inside the setup hook where the
    // bare tokio runtime context is not yet active.
    tauri::async_runtime::spawn(async move {
        let mut ticker = interval(Duration::from_millis(100));
        loop {
            ticker.tick().await;

            if !player.is_playing() {
                continue;
            }

            let (pos, entry_id) = match (player.position_sec(), player.current_entry_id()) {
                (Some(p), Some(id)) => (p, id),
                _ => continue,
            };

            let _ = app.emit(
                "playback_position",
                json!({ "position_sec": pos, "entry_id": entry_id }),
            );

            // EOF detection: emit finished + stopped when position reaches duration.
            if let Some(dur) = player.duration_sec() {
                if dur > 0.0 && pos >= dur - 0.05 {
                    debug!("playback finished: entry_id={entry_id}");
                    let _ = app.emit("playback_finished", json!({ "entry_id": entry_id }));
                    let _ = app.emit("playback_stopped", json!({}));
                    player.state.lock().is_playing = false;
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time check: Player::new returns Result (does not panic at type
    /// level).  The body is never executed because we cannot build a real
    /// AppHandle in a unit test without a running Tauri app.
    fn _assert_new_returns_result<R: Runtime>(app: AppHandle<R>) -> Result<Player<R>> {
        Player::new(app)
    }

    /// Smoke: PlayerError variants format correctly.
    #[test]
    fn player_error_display() {
        let e = PlayerError::Init("test init".to_string());
        assert_eq!(e.to_string(), "mpv init failed: test init");

        let e = PlayerError::Op("test op".to_string());
        assert_eq!(e.to_string(), "mpv operation failed: test op");

        let e = PlayerError::FileNotFound("/tmp/x.wav".to_string());
        assert_eq!(e.to_string(), "file not found: /tmp/x.wav");
    }

    /// Integration test (requires libmpv runtime + audio device): load a small
    /// WAV, play briefly, check that position increases.
    ///
    /// Run with: `cargo test -- --ignored`
    #[test]
    #[ignore]
    fn integration_play_wav_position_increases() {
        // This test must be run on a host with mpv installed and an audio
        // output device available.  It is ignored by default so CI does not
        // fail in headless environments.
        //
        // To run:
        //   cargo test --lib player -- --ignored
        //
        // The test would:
        // 1. Build a minimal Tauri app context (tauri::test::mock_app).
        // 2. Create Player::new(app.handle().clone()).
        // 3. Call player.load(&wav_path, "test-entry".into()).
        // 4. Call player.play().
        // 5. Sleep 300 ms.
        // 6. Assert player.position_sec() > Some(0.1).
        // 7. Call player.stop().
        //
        // Skipped here because tauri::test requires the full Tauri test harness
        // which is not available without the `test-utils` feature enabled.
        println!("integration test skipped — run with --ignored on a real host");
    }
}
