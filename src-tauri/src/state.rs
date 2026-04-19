use std::sync::Arc;

use parking_lot::Mutex;

use crate::pipeline::TTSPipeline;
use crate::player::Player;
use crate::storage::service::StorageService;
use crate::tray::TrayCmd;
use crate::tts::TtsSubprocess;

/// Application-wide state held in `tauri::State<AppState>`.
///
/// Uses the concrete `tauri::Wry` runtime so that `AppState` is non-generic
/// and can be registered with `app.manage()` without ambiguity in
/// `tauri::generate_handler!`.
pub struct AppState {
    pub storage: Arc<StorageService>,
    pub tts: Arc<TtsSubprocess>,
    pub player: Arc<Player<tauri::Wry>>,
    pub pipeline: Arc<Mutex<TTSPipeline>>,
    /// Sender for tray menu commands (read_now / read_later).
    /// `None` before the background loop is started in `setup()`.
    pub tray_cmd_tx: Option<tokio::sync::mpsc::Sender<TrayCmd>>,
}
