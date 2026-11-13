//! Audio playback for the dictionary editor's pronunciation preview.
//!
//! The preview must not use the webview's HTML media stack (issue #349): a
//! WebKitGTK build without a GStreamer audio sink aborts the whole web
//! process inside `createAudioSink()`, and no JavaScript can catch that abort.
//! Playback therefore runs in a dedicated mpv instance driven by the same
//! plugin as narration, keyed by [`PREVIEW_WINDOW_LABEL`] so it never touches
//! the narration player's loaded entry, position or `playback_*` events.
//!
//! The instance is created by the first [`MpvPreviewAudio::play`] and then
//! reused: `--idle=yes` keeps mpv alive between clips. Re-initializing a
//! killed instance is what leaves the plugin's IPC socket behind, so
//! [`MpvPreviewAudio::stop`] only halts playback and deletes the WAV; the
//! process is destroyed by [`MpvPreviewAudio::shutdown`] (app exit and
//! [`Drop`]). The editor stops playback when the reported duration elapses.

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

    /// Stop playback and delete the WAV. Idempotent: a missing instance or
    /// file is not an error. The mpv process stays alive for the next clip.
    fn stop(&self) -> Result<(), PreviewAudioError>;

    /// Destroy the preview instance. Called on app exit and from [`Drop`]; a
    /// later `play` starts a fresh process.
    fn shutdown(&self) -> Result<(), PreviewAudioError> {
        Ok(())
    }
}

/// Production preview player: one idle mpv process shared by all clips.
pub struct MpvPreviewAudio {
    app: AppHandle<Wry>,
    /// Directory for the temporary WAVs (`<app cache>/preview`).
    dir: PathBuf,
    /// Serializes play/stop and holds the file of the preview that may be
    /// playing right now.
    current: Mutex<Option<PathBuf>>,
}

/// Owns the temporary WAV of the preview that may be playing right now.
///
/// Split from the mpv control so the cleanup paths are covered by unit tests
/// that need no process, display or audio device.
struct PreviewFiles<'a> {
    dir: &'a Path,
}

impl<'a> PreviewFiles<'a> {
    fn new(dir: &'a Path) -> Self {
        Self { dir }
    }

    /// Write `wav` under a fresh UUID name; `create_dir_all` makes the first
    /// preview self-sufficient.
    fn write(&self, wav: &[u8]) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(self.dir)?;
        let path = self.dir.join(format!("{}.wav", uuid::Uuid::new_v4()));
        std::fs::write(&path, wav)?;
        Ok(path)
    }

    /// Remove `path`, ignoring a file that is already gone.
    fn delete(path: &Path) {
        if let Err(e) = std::fs::remove_file(path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!("preview: failed to delete {}: {e}", path.display());
        }
    }
}
impl MpvPreviewAudio {
    pub fn new(app: AppHandle<Wry>, dir: PathBuf) -> Self {
        Self {
            app,
            dir,
            current: Mutex::new(None),
        }
    }

    /// Start the instance if needed and hand it the file. A failed start is
    /// retried once after dropping the plugin's IPC socket: a killed mpv
    /// leaves the socket file behind, and a fresh process then cannot bind it
    /// (`connect` → `ECONNREFUSED`), which would fail every later preview.
    fn start(&self, path: &Path) -> Result<(), PreviewAudioError> {
        let first = self.init_and_load(path);
        if first.is_ok() {
            return first;
        }
        tracing::warn!("preview: mpv start failed, resetting its IPC socket: {first:?}");
        self.reset_ipc();
        self.init_and_load(path)
    }

    fn init_and_load(&self, path: &Path) -> Result<(), PreviewAudioError> {
        let resource_dir = self.app.path().resource_dir().ok();
        let config = MpvConfig {
            path: resolve_mpv_path(resource_dir.as_deref())
                .to_string_lossy()
                .into_owned(),
            // Audio-only, no speed filter: the preview is a short clip at the
            // recorded rate. Idle keeps the process between clips.
            args: vec![
                "--no-video".to_string(),
                "--no-ytdl".to_string(),
                "--idle=yes".to_string(),
            ],
            observed_properties: vec![],
            ipc_timeout_ms: 2000,
            show_mpv_output: false,
        };
        // The plugin locks its instance map with `unwrap` and can panic while
        // holding it; catching keeps the app and the narration player alive.
        let init = std::panic::catch_unwind(AssertUnwindSafe(|| {
            self.app.mpv().init(config, PREVIEW_WINDOW_LABEL)
        }));
        match init {
            Err(_) => return Err(PreviewAudioError::Init("mpv init panicked".into())),
            Ok(Err(e)) => return Err(PreviewAudioError::Init(e.to_string())),
            Ok(Ok(_)) => {}
        }

        self.command(json!(["loadfile", path]))?;
        self.command(json!(["set_property", "pause", false]))?;
        Ok(())
    }

    /// Drop the instance and the stale IPC socket it may have left behind.
    fn reset_ipc(&self) {
        let _ = self.destroy();
        #[cfg(unix)]
        {
            // Mirrors tauri-plugin-mpv 0.5.2 `ipc::get_ipc_pipe` (not
            // re-exported): the plugin names the socket after the main process
            // and the instance label.
            let socket = format!(
                "/tmp/tauri_plugin_mpv_socket_{}_{}",
                std::process::id(),
                PREVIEW_WINDOW_LABEL
            );
            if let Err(e) = std::fs::remove_file(&socket)
                && e.kind() != std::io::ErrorKind::NotFound
            {
                tracing::warn!("preview: failed to remove stale mpv socket {socket}: {e}");
            }
        }
    }
    /// Whether the plugin holds a live-looking instance for the preview label.
    ///
    /// `Mpv::command` unwraps that map, and a missing entry panics while the
    /// plugin's mutex is locked: the poison would then break narration too.
    /// A poisoned map (an earlier panic elsewhere in the plugin) is reported as
    /// absent rather than panicking again.
    fn has_instance(&self) -> bool {
        self.app
            .mpv()
            .instances
            .lock()
            .map(|instances| instances.contains_key(PREVIEW_WINDOW_LABEL))
            .unwrap_or(false)
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
        // `kill_mpv_process` also unwraps the plugin's instance map: a poisoned
        // map would turn an exit cleanup into a second panic.
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            self.app.mpv().destroy(PREVIEW_WINDOW_LABEL)
        }));
        match result {
            Err(_) => Err(PreviewAudioError::Command("mpv destroy panicked".into())),
            Ok(Err(e)) => Err(PreviewAudioError::Command(e.to_string())),
            Ok(Ok(_)) => Ok(()),
        }
    }
}

impl PreviewAudioBackend for MpvPreviewAudio {
    fn play(&self, wav: &[u8]) -> Result<(), PreviewAudioError> {
        let mut current = self.current.lock();

        // Replace: stop the clip that may still be playing and delete its
        // file. The process is kept — re-initializing it is what the plugin's
        // stale IPC socket breaks (`start`).
        if let Some(previous) = current.take() {
            // Guard the command: the plugin's dispatcher unwraps its instance
            // map, and a panic there poisons the plugin's global mutex, which
            // would break narration too.
            if self.has_instance()
                && let Err(e) = self.command(json!(["stop"]))
            {
                tracing::debug!("preview: stopping the previous clip failed: {e}");
            }
            PreviewFiles::delete(&previous);
        }

        let path = PreviewFiles::new(&self.dir).write(wav)?;

        match self.start(&path) {
            Ok(()) => {
                *current = Some(path);
                Ok(())
            }
            Err(e) => {
                PreviewFiles::delete(&path);
                Err(e)
            }
        }
    }

    fn stop(&self) -> Result<(), PreviewAudioError> {
        let mut current = self.current.lock();
        let path = current.take();
        // Keep the process: `stop` halts the clip but leaves mpv idle. Never
        // send the command to an absent instance: the plugin's dispatcher
        // unwraps the map and a panic there poisons the mutex used by the
        // narration player as well.
        if path.is_some()
            && self.has_instance()
            && let Err(e) = self.command(json!(["stop"]))
        {
            tracing::debug!("preview: stop command failed: {e}");
        }
        if let Some(path) = path {
            PreviewFiles::delete(&path);
        }
        Ok(())
    }

    fn shutdown(&self) -> Result<(), PreviewAudioError> {
        let mut current = self.current.lock();
        if let Some(path) = current.take() {
            PreviewFiles::delete(&path);
        }
        self.reset_ipc();
        Ok(())
    }
}

impl Drop for MpvPreviewAudio {
    fn drop(&mut self) {
        let _ = self.shutdown();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_files_writes_and_deletes_a_temporary_wav() {
        let root = tempfile::TempDir::new().unwrap();
        let dir = root.path().join("preview");
        let files = PreviewFiles::new(&dir);

        let path = files.write(b"RIFF....WAVE").unwrap();

        assert_eq!(path.parent().unwrap(), dir);
        assert_eq!(std::fs::read(&path).unwrap(), b"RIFF....WAVE");
        PreviewFiles::delete(&path);
        assert!(!path.exists());
        // Idempotent: the sweep or a double stop may delete it first.
        PreviewFiles::delete(&path);
    }

    #[test]
    fn preview_files_reports_an_unusable_cache_directory() {
        let root = tempfile::TempDir::new().unwrap();
        let blocker = root.path().join("preview");
        std::fs::write(&blocker, b"not a directory").unwrap();

        let err = PreviewFiles::new(&blocker).write(b"RIFF").unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    }
}
