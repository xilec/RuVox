use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use parking_lot::Mutex;
use tokio::task::AbortHandle;

use crate::dictionary::DictionaryStore;
use crate::pipeline::TTSPipeline;
use crate::player::PlayerBackend;
use crate::storage::schema::EntryId;
use crate::storage::service::StorageService;
use crate::tray::TrayCmd;
use crate::tts::supervisor::Emitter;
use crate::tts::{EngineSwitcher, TtsEngine};

/// Application-wide state held in `tauri::State<AppState>`.
///
/// Runtime-agnostic by design (no `AppHandle` / `Player<R>` generics), so the
/// same state can be registered with `app.manage()` both in the production
/// `Wry` app and in the `MockRuntime` test harness.
pub struct AppState {
    pub storage: Arc<StorageService>,
    /// TTS engine — actually an [`EngineSwitcher`], exposed as a trait object
    /// so the synthesis pipeline stays engine-agnostic. Use `engine_switcher`
    /// when the caller needs to swap the underlying engine.
    pub tts: Arc<dyn TtsEngine>,
    /// Typed handle to the same switcher held in `tts`. Used by
    /// `update_config` to apply engine / voice changes at runtime.
    pub engine_switcher: Arc<EngineSwitcher>,
    /// Resolved on-disk location of the optional `ttsd/` Python package.
    /// Consumed by `get_available_engines` to probe Silero's environment
    /// without re-discovering the path on every call.
    pub ttsd_dir: PathBuf,
    /// Root directory for Piper voice files (one subdir per voice id).
    /// Used by `download_piper_voice` and the auto-download fallback in
    /// `synthesize_audio` so they don't need to re-derive the path.
    pub piper_voices_dir: PathBuf,
    /// Directory holding the Silero Native model bundle (`manifest.json` +
    /// ONNX/dict files). Consumed by `get_available_engines` (availability
    /// probe) and `download_silero_native_bundle`.
    pub silero_native_bundle_dir: PathBuf,
    /// Frontend emitter shared with the engine layer. Held here so the
    /// download path can reuse it without rebuilding the closure.
    pub emitter: Emitter,
    /// Audio player behind an object-safe trait so tests can substitute a
    /// fake (no mpv subprocess / window). Production holds `Player<Wry>`.
    pub player: Arc<dyn PlayerBackend>,
    pub pipeline: Arc<Mutex<TTSPipeline>>,
    /// Owns the user dictionary file (config root). Commands layer reads it
    /// for save/import/export; the loaded dictionary itself lives in
    /// `pipeline` (single home for the active map).
    pub dictionary_store: Arc<DictionaryStore>,
    /// Sender for tray menu commands (read_now / read_later).
    /// `None` before the background loop is started in `setup()`.
    pub tray_cmd_tx: Option<tokio::sync::mpsc::Sender<TrayCmd>>,
    /// Set to `true` when the user picks "Выход" in the tray menu.  Lets the
    /// runtime distinguish a real quit from a window-close that should keep
    /// the app running in the tray.
    pub user_quit: Arc<AtomicBool>,
    /// Abort handles of the spawned synthesis tasks, keyed by entry.
    /// Populated by `spawn_synthesis` and removed when the task finishes
    /// (any outcome) or when `cancel_synthesis` aborts it.
    pub synthesis_tasks: Arc<Mutex<HashMap<EntryId, AbortHandle>>>,
    /// Entries currently inside the `tts.synthesize()` await. Lets
    /// `cancel_synthesis` decide whether the ttsd subprocess must be killed
    /// (only entries that reached the TTS stage keep ttsd busy).
    pub synthesize_entered: Arc<Mutex<HashSet<EntryId>>>,
}
