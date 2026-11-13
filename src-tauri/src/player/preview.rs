//! Audio playback for the dictionary editor's pronunciation preview.
//!
//! The preview must not use the webview's HTML media stack (issue #349): a
//! WebKitGTK build without a GStreamer audio sink aborts the whole web
//! process inside `createAudioSink()`, and no JavaScript can catch that abort.
//! Playback therefore runs in a dedicated mpv instance driven by the same
//! plugin as narration, keyed by [`PREVIEW_WINDOW_LABEL`] so it never touches
//! the narration player's loaded entry, position or `playback_*` events.
//!
//! The instance lives only while a preview plays: it is created by
//! [`MpvPreviewAudio::play`] and destroyed by [`MpvPreviewAudio::stop`], the
//! editor's close path, [`Drop`] and the app's exit handler. `--idle=yes`
//! keeps it alive after the WAV ends so the stop path always kills a live
//! process instead of racing an exit; the editor stops it when the reported
//! duration elapses.

use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde_json::json;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_mpv::{MpvCommand, MpvConfig, MpvExt};

#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use super::{PREVIEW_WINDOW_LABEL, resolve_mpv_path};

#[derive(Debug, thiserror::Error)]
pub enum PreviewAudioError {
    #[error("failed to write preview audio: {0}")]
    Write(#[from] std::io::Error),
    #[error("mpv init failed: {0}")]
    Init(String),
    #[error("mpv command failed: {0}")]
    Command(String),
}

/// Object-safe seam over the preview player, mirroring [`super::PlayerBackend`]
/// so command tests can run without mpv, a display or a real process.
pub trait PreviewAudioBackend: Send + Sync {
    /// Write `wav` into the preview cache and start playing it. A preview
    /// that is still playing is stopped (and its file deleted) first.
    fn play(&self, wav: &[u8]) -> Result<(), PreviewAudioError>;

    /// Stop and destroy the preview instance and delete its WAV. Idempotent:
    /// a missing instance or file is not an error.
    fn stop(&self) -> Result<(), PreviewAudioError>;
}

/// Production preview player: one short-lived mpv process per playback.
pub struct MpvPreviewAudio {
    app: AppHandle<Wry>,
    /// Directory for the temporary WAVs (`<app cache>/preview`).
    dir: PathBuf,
    /// Serializes play/stop and holds the file of the preview that may be
    /// playing right now.
    current: Mutex<Option<PathBuf>>,
}

impl MpvPreviewAudio {
    pub fn new(app: AppHandle<Wry>, dir: PathBuf) -> Self {
        Self {
            app,
            dir,
            current: Mutex::new(None),
        }
    }

    /// Start the mpv instance and hand it the file. Split from [`Self::play`]
    /// so the caller can clean up the file on every failure path.
    fn start(&self, path: &Path) -> Result<(), PreviewAudioError> {
        let resource_dir = self.app.path().resource_dir().ok();
        let config = MpvConfig {
            path: resolve_mpv_path(resource_dir.as_deref())
                .to_string_lossy()
                .into_owned(),
            // Audio-only, no speed filter: the preview is a short clip at the
            // recorded rate. Idle keeps the process alive after EOF so stop
            // always kills a live child instead of racing an exit.
            args: vec![
                "--no-video".to_string(),
                "--no-ytdl".to_string(),
                "--idle=yes".to_string(),
            ],
            observed_properties: vec![],
            ipc_timeout_ms: 2000,
            show_mpv_output: false,
        };
        self.app
            .mpv()
            .init(config, PREVIEW_WINDOW_LABEL)
            .map_err(|e| PreviewAudioError::Init(e.to_string()))?;

        self.command(json!(["loadfile", path]))?;
        self.command(json!(["set_property", "pause", false]))?;
        Ok(())
    }

    fn command(&self, command: serde_json::Value) -> Result<(), PreviewAudioError> {
        let cmd = MpvCommand {
            command: command.as_array().cloned().unwrap_or_default(),
            request_id: None,
        };
        // The plugin's dispatcher unwraps its instance map; a panic means the
        // instance is gone. Our own play/stop are serialized by `current`, so
        // this is a backstop rather than an expected path.
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            self.app.mpv().command(cmd, PREVIEW_WINDOW_LABEL)
        }));
        match result {
            Err(_) => Err(PreviewAudioError::Command(
                "mpv command panicked (instance gone?)".into(),
            )),
            Ok(Err(e)) => Err(PreviewAudioError::Command(e.to_string())),
            Ok(Ok(_)) => Ok(()),
        }
    }

    /// Kill the instance, if any. A plugin error is reported but never blocks
    /// deleting the WAV: the process may already be gone.
    fn destroy(&self) -> Result<(), PreviewAudioError> {
        self.app
            .mpv()
            .destroy(PREVIEW_WINDOW_LABEL)
            .map_err(|e| PreviewAudioError::Command(e.to_string()))
    }

    fn delete_file(path: &Path) {
        if let Err(e) = std::fs::remove_file(path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!("preview: failed to delete {}: {e}", path.display());
        }
    }
}

impl PreviewAudioBackend for MpvPreviewAudio {
    fn play(&self, wav: &[u8]) -> Result<(), PreviewAudioError> {
        let mut current = self.current.lock();

        // The plugin inserts into its instance map by label: a live instance
        // would be leaked by a second init, so stop the old preview first.
        if current.take().is_some()
            && let Err(e) = self.destroy()
        {
            tracing::warn!("preview: failed to stop the previous instance: {e}");
        }

        std::fs::create_dir_all(&self.dir)?;
        let path = self.dir.join(format!("{}.wav", uuid::Uuid::new_v4()));
        std::fs::write(&path, wav)?;

        match self.start(&path) {
            Ok(()) => {
                *current = Some(path);
                Ok(())
            }
            Err(e) => {
                Self::delete_file(&path);
                let _ = self.destroy();
                Err(e)
            }
        }
    }

    fn stop(&self) -> Result<(), PreviewAudioError> {
        let mut current = self.current.lock();
        let path = current.take();
        let destroy = self.destroy();
        if let Some(path) = path {
            Self::delete_file(&path);
        }
        destroy
    }
}

impl Drop for MpvPreviewAudio {
    fn drop(&mut self) {
        let _ = self.destroy();
    }
}

/// Test double: records every payload handed to the player and every stop.
#[cfg(test)]
pub struct FakePreviewAudio {
    plays: Mutex<Vec<Vec<u8>>>,
    stops: AtomicUsize,
    fail_next_play: AtomicBool,
}

#[cfg(test)]
impl Default for FakePreviewAudio {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl FakePreviewAudio {
    pub fn new() -> Self {
        Self {
            plays: Mutex::new(Vec::new()),
            stops: AtomicUsize::new(0),
            fail_next_play: AtomicBool::new(false),
        }
    }

    /// Make the next `play` fail, exercising the command's error mapping.
    pub fn fail_next_play(&self) {
        self.fail_next_play.store(true, Ordering::SeqCst);
    }

    pub fn played(&self) -> Vec<Vec<u8>> {
        self.plays.lock().clone()
    }

    pub fn stop_count(&self) -> usize {
        self.stops.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
impl PreviewAudioBackend for FakePreviewAudio {
    fn play(&self, wav: &[u8]) -> Result<(), PreviewAudioError> {
        if self.fail_next_play.swap(false, Ordering::SeqCst) {
            return Err(PreviewAudioError::Command("no mpv".into()));
        }
        self.plays.lock().push(wav.to_vec());
        Ok(())
    }

    fn stop(&self) -> Result<(), PreviewAudioError> {
        self.stops.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
