use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::pipeline::TTSPipeline;
use crate::player::Player;
use crate::storage::service::StorageService;
use crate::tray::TrayCmd;
use crate::tts::TtsEngine;

/// Application-wide state held in `tauri::State<AppState>`.
///
/// Uses the concrete `tauri::Wry` runtime so that `AppState` is non-generic
/// and can be registered with `app.manage()` without ambiguity in
/// `tauri::generate_handler!`.
pub struct AppState {
    pub storage: Arc<StorageService>,
    /// TTS engine. Today this is always [`crate::tts::PiperEngine`]; #43 will
    /// pick between Piper and Silero at startup based on the user's config
    /// and an availability probe.
    pub tts: Arc<dyn TtsEngine>,
    pub player: Arc<Player<tauri::Wry>>,
    pub pipeline: Arc<Mutex<TTSPipeline>>,
    /// Sender for tray menu commands (read_now / read_later).
    /// `None` before the background loop is started in `setup()`.
    pub tray_cmd_tx: Option<tokio::sync::mpsc::Sender<TrayCmd>>,
    /// Set to `true` when the user picks "Выход" in the tray menu.  Lets the
    /// runtime distinguish a real quit from a window-close that should keep
    /// the app running in the tray.
    pub user_quit: Arc<AtomicBool>,
}
