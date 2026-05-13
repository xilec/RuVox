pub mod audio;
pub mod commands;
pub mod pipeline;
pub mod player;
pub mod state;
pub mod storage;
pub mod tray;
pub mod tts;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, RunEvent, Runtime};
use tauri_plugin_mpv::MpvExt;

use commands::*;
use pipeline::TTSPipeline;
use player::Player;
use state::AppState;
use storage::service::StorageService;
use tray::TrayCmd;

type SetupError = Box<dyn std::error::Error>;

/// Kill orphan mpv processes left over from a previous crash of this binary.
///
/// tauri-plugin-mpv creates a UNIX socket at
/// `/tmp/tauri_plugin_mpv_socket_<parent_pid>_<window_label>`.  If the
/// parent_pid no longer exists, the corresponding mpv is an orphan that
/// survived a crash/SIGKILL.  Find those mpv PIDs via `/proc/<pid>/cmdline`
/// search and send SIGTERM.
fn reap_orphan_mpv() {
    let Ok(entries) = std::fs::read_dir("/tmp") else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(s) = name.to_str() else { continue };
        if !s.starts_with("tauri_plugin_mpv_socket_") {
            continue;
        }
        let parts: Vec<&str> = s.split('_').collect();
        let Some(parent_pid_str) = parts.get(4) else {
            continue;
        };
        let Ok(parent_pid) = parent_pid_str.parse::<u32>() else {
            continue;
        };
        if std::path::Path::new(&format!("/proc/{parent_pid}")).exists() {
            continue;
        }
        // Parent dead → find mpv with this IPC socket arg and kill it.
        if let Ok(procs) = std::fs::read_dir("/proc") {
            for p in procs.flatten() {
                let Ok(pid) = p.file_name().to_string_lossy().parse::<u32>() else {
                    continue;
                };
                let Ok(cmdline) = std::fs::read_to_string(format!("/proc/{pid}/cmdline")) else {
                    continue;
                };
                if cmdline.contains(&format!("tauri_plugin_mpv_socket_{parent_pid}_")) {
                    tracing::warn!("reaping orphan mpv pid={pid} (parent {parent_pid} dead)");
                    unsafe {
                        libc::kill(pid as i32, libc::SIGTERM);
                    }
                }
            }
        }
        let _ = std::fs::remove_file(entry.path());
    }
}

/// Inject the bundled 128×128 PNG as the WebView window icon.
///
/// `WindowConfig` in `tauri.conf.json` has no `icon` field, and bundle-level
/// icons only get wired up by `cargo tauri build`. For dev (and as hardening
/// for release), set the window icon explicitly so X11 / app switchers / task
/// bars pick it up. Linux GTK CSD title bars typically don't render this
/// icon inside the title bar itself — that is a GNOME/KDE chrome decision,
/// not a Tauri limitation.
fn install_window_icon<R: Runtime>(app: &AppHandle<R>) -> Result<(), SetupError> {
    if let Some(window) = app.get_webview_window("main") {
        let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/128x128.png"))?;
        let _ = window.set_icon(icon);
    }
    Ok(())
}

/// One-shot WAV→Opus migration followed by a startup cache cleanup, both
/// running on Tauri's async runtime so app startup is not delayed.
///
/// Order matters: migration finishes before the orphan sweep walks the audio
/// directory, so freshly-renamed `.opus` files are already linked to their
/// entries by the time the sweep runs.
fn spawn_audio_migration_and_cleanup(storage: Arc<StorageService>) {
    tauri::async_runtime::spawn(async move {
        let storage_for_cleanup = Arc::clone(&storage);
        let stats = tokio::task::spawn_blocking(move || storage.migrate_wav_audio_to_opus()).await;
        match stats {
            Ok(s) if s.considered == 0 => {
                tracing::debug!("audio migration: nothing to do");
            }
            Ok(s) => {
                tracing::info!(
                    "audio migration: considered={}, migrated={}, skipped_missing={}, failed={}",
                    s.considered,
                    s.migrated,
                    s.skipped_missing,
                    s.failed
                );
            }
            Err(e) => {
                tracing::error!("audio migration task panicked: {e}");
            }
        }

        let cleanup_result = tokio::task::spawn_blocking(move || {
            let cfg = storage_for_cleanup.load_config().unwrap_or_default();
            let target_bytes = (cfg.max_cache_size_mb as u64) * 1024 * 1024;
            storage_for_cleanup
                .run_startup_cleanup(target_bytes)
                .map_err(|e| e.to_string())
        })
        .await;
        match cleanup_result {
            Ok(Ok(s)) => {
                tracing::info!(
                    "startup cleanup: orphans={}, evicted_files={}, freed={} bytes",
                    s.sweep.deleted_files,
                    s.evict.deleted_files,
                    s.sweep.freed_bytes + s.evict.freed_bytes,
                );
            }
            Ok(Err(e)) => {
                tracing::warn!("startup cleanup failed: {e}");
            }
            Err(e) => {
                tracing::error!("startup cleanup task panicked: {e}");
            }
        }
    });
}

/// Resolve the on-disk `ttsd/` directory used by Silero.
///
/// In production the bundle ships ttsd next to the binary (resource_dir/ttsd).
/// In `cargo tauri dev` cwd is `src-tauri/`, so the project ttsd lives at
/// `../ttsd`; fall back to `./ttsd` for ad-hoc runs from the repo root.
/// The path is consumed by [`tts::EngineSwitcher::apply_config`] only when
/// the user actually picks Silero — the directory is not required to exist
/// at startup.
fn resolve_ttsd_dir<R: Runtime>(app: &AppHandle<R>) -> std::path::PathBuf {
    let res_dir = app
        .path()
        .resource_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("ttsd");
    if res_dir.exists() {
        res_dir
    } else if std::path::Path::new("../ttsd/pyproject.toml").exists() {
        std::path::PathBuf::from("../ttsd")
    } else {
        std::path::PathBuf::from("ttsd")
    }
}

/// Build the TTS engine layer for the running app.
///
/// Returns the [`tts::EngineSwitcher`] (which owns the active engine) and
/// the resolved `ttsd/` directory so the rest of the app can re-probe
/// availability later via `get_available_engines`.
///
/// The initial engine is chosen from the persisted config and the Silero
/// availability probe (Phase 3 of #42):
/// - `engine = "piper"` (default) → in-process [`tts::PiperEngine`].
/// - `engine = "silero"` *and* probe says Silero is available →
///   [`tts::TtsSupervisor`] over `uv run python -m ttsd`. Spawn failure
///   (race between probe and spawn) silently falls back to Piper.
/// - `engine = "silero"` but probe says unavailable → silent migration to
///   Piper; the user's `engine` value on disk is left untouched so they
///   can roll back by installing the Silero stack.
///
/// Voice models live at `<data_local_dir>/ruvox/voices/piper/<voice>/…`
/// — see `tts::piper::catalog`. Phase 4 of #42 adds on-demand download;
/// for now, if the default voice isn't on disk, warmup emits `model_error`
/// and the first synthesize returns `voice_not_installed`.
/// Returns the active engine layer plus the runtime paths and emitter the
/// rest of the app needs (Phase 4 download command, Phase 3 probe).
type EngineWiring = (
    Arc<tts::EngineSwitcher>,
    std::path::PathBuf, // ttsd_dir
    std::path::PathBuf, // piper_voices_dir
    tts::supervisor::Emitter,
);

fn build_engine<R: Runtime>(
    app: &AppHandle<R>,
    storage: &StorageService,
) -> Result<EngineWiring, SetupError> {
    let voices_dir = dirs::data_local_dir()
        .ok_or("dirs::data_local_dir() returned None")?
        .join("ruvox")
        .join("voices")
        .join("piper");

    let app_handle_for_emitter = app.clone();
    let emitter: tts::supervisor::Emitter = Arc::new(move |event_name, payload| {
        let _ = app_handle_for_emitter.emit(event_name, payload);
    });

    let config = storage.load_config().unwrap_or_default();
    let ttsd_dir = resolve_ttsd_dir(app);

    let want_silero = config.engine == "silero";
    let silero_available = want_silero && tts::availability::probe(&ttsd_dir).silero.available;

    let (initial_engine, initial_kind, initial_voice) = if silero_available {
        match try_build_silero(&ttsd_dir, Arc::clone(&emitter)) {
            Ok(sup) => {
                let engine: Arc<dyn tts::TtsEngine> = sup;
                (engine, tts::EngineKind::Silero, None)
            }
            Err(e) => {
                tracing::warn!("Silero probe passed but spawn failed ({e}); falling back to Piper");
                build_piper_initial(&voices_dir, &config.piper_voice, &emitter)
            }
        }
    } else {
        if want_silero {
            tracing::info!("Silero requested in config but unavailable; serving Piper this run");
        }
        build_piper_initial(&voices_dir, &config.piper_voice, &emitter)
    };

    let switcher = Arc::new(tts::EngineSwitcher::new(
        initial_engine,
        initial_kind,
        initial_voice,
        voices_dir.clone(),
        ttsd_dir.clone(),
        Arc::clone(&emitter),
    ));
    Ok((switcher, ttsd_dir, voices_dir, emitter))
}

fn build_piper_initial(
    voices_dir: &std::path::Path,
    voice: &str,
    emitter: &tts::supervisor::Emitter,
) -> (Arc<dyn tts::TtsEngine>, tts::EngineKind, Option<String>) {
    let engine: Arc<dyn tts::TtsEngine> = Arc::new(tts::PiperEngine::new(
        voices_dir.to_path_buf(),
        voice.to_string(),
        Arc::clone(emitter),
    ));
    (engine, tts::EngineKind::Piper, Some(voice.to_string()))
}

fn try_build_silero(
    ttsd_dir: &std::path::Path,
    emitter: tts::supervisor::Emitter,
) -> Result<Arc<tts::TtsSupervisor>, tts::TtsError> {
    let ttsd_dir = ttsd_dir.to_path_buf();
    let factory: tts::supervisor::CommandFactory = Arc::new(move || {
        let mut cmd = tokio::process::Command::new("uv");
        cmd.args(["run", "python", "-m", "ttsd"])
            .current_dir(&ttsd_dir);
        cmd
    });
    // tokio::process::Command::spawn requires an active tokio runtime
    // context; the Tauri setup hook runs synchronously, so enter the
    // runtime explicitly via block_on (the inner spawn returns instantly).
    let supervisor =
        tauri::async_runtime::block_on(async move { tts::TtsSupervisor::spawn(factory, emitter) })?;
    Ok(Arc::new(supervisor))
}

/// Spawn the tray-command handler loop and return the channel sender.
///
/// The tray emits commands for "read clipboard now" / "queue clipboard"; this
/// loop reads the system clipboard on a blocking thread, creates a history
/// entry, and kicks off background synthesis.
#[allow(clippy::too_many_arguments)]
fn spawn_tray_handler<R: Runtime + 'static>(
    storage: Arc<StorageService>,
    tts: Arc<dyn tts::TtsEngine>,
    piper_voices_dir: std::path::PathBuf,
    emitter: tts::supervisor::Emitter,
    player: Arc<Player<R>>,
    pipeline: Arc<Mutex<TTSPipeline>>,
    app: AppHandle<R>,
) -> tokio::sync::mpsc::Sender<TrayCmd> {
    let (tray_tx, mut tray_rx) = tokio::sync::mpsc::channel::<TrayCmd>(16);

    tauri::async_runtime::spawn(async move {
        while let Some(cmd) = tray_rx.recv().await {
            // Read clipboard on a blocking thread (required on Linux).
            let text_result = tokio::task::spawn_blocking(|| {
                let mut board = arboard::Clipboard::new()?;
                board.get_text()
            })
            .await;

            let text = match text_result {
                Ok(Ok(t)) if !t.trim().is_empty() => t,
                Ok(Ok(_)) => {
                    tracing::warn!("tray: clipboard is empty");
                    continue;
                }
                Ok(Err(e)) => {
                    tracing::error!("tray: clipboard read failed: {e}");
                    continue;
                }
                Err(e) => {
                    tracing::error!("tray: clipboard task panicked: {e}");
                    continue;
                }
            };

            let entry = match storage.add_entry(text) {
                Ok(e) => e,
                Err(e) => {
                    tracing::error!("tray: failed to add entry: {e}");
                    continue;
                }
            };

            let entry_id = entry.id;
            let _ = app.emit("entry_updated", json!({ "entry": entry }));

            spawn_synthesis_pub(
                app.clone(),
                Arc::clone(&storage),
                Arc::clone(&tts),
                piper_voices_dir.clone(),
                Arc::clone(&emitter),
                Arc::clone(&player),
                Arc::clone(&pipeline),
                entry_id,
                cmd.play_when_ready,
            );
        }
    });

    tray_tx
}

pub fn run() {
    reap_orphan_mpv();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_mpv::init())
        .setup(|app| {
            tray::init(app.handle())?;
            install_window_icon(app.handle())?;

            let player = Arc::new(Player::new(app.handle().clone())?);
            player::spawn_position_emitter(player.clone(), app.handle().clone());

            let storage = Arc::new(StorageService::new().expect("failed to open storage"));
            spawn_audio_migration_and_cleanup(Arc::clone(&storage));

            let (engine_switcher, ttsd_dir, piper_voices_dir, emitter) =
                build_engine(app.handle(), &storage)?;
            let tts: Arc<dyn tts::TtsEngine> = engine_switcher.clone();
            // Warm up the model in background. The engine owns the
            // model_loading → model_loaded/model_error emit sequence so the
            // initial warmup and post-respawn warmup share one code path.
            {
                let tts_clone = Arc::clone(&tts);
                tauri::async_runtime::spawn(async move {
                    tts_clone.spawn_initial_warmup().await;
                });
            }

            let pipeline = Arc::new(Mutex::new(TTSPipeline::new()));
            let tray_tx = spawn_tray_handler(
                Arc::clone(&storage),
                Arc::clone(&tts),
                piper_voices_dir.clone(),
                Arc::clone(&emitter),
                Arc::clone(&player),
                Arc::clone(&pipeline),
                app.handle().clone(),
            );

            app.manage(AppState {
                storage,
                tts,
                engine_switcher,
                ttsd_dir,
                piper_voices_dir,
                emitter,
                player,
                pipeline,
                tray_cmd_tx: Some(tray_tx),
                user_quit: Arc::new(AtomicBool::new(false)),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            add_clipboard_entry,
            add_text_entry,
            preview_normalize,
            get_entries,
            get_entry,
            delete_entry,
            delete_audio,
            regenerate_entry,
            cancel_synthesis,
            play_entry,
            pause_playback,
            resume_playback,
            stop_playback,
            seek_to,
            set_speed,
            set_volume,
            get_config,
            update_config,
            get_available_engines,
            download_piper_voice,
            get_timestamps,
            clear_cache,
            get_cache_stats,
            get_cache_dir,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            // Window-close intercept: hide the main window instead of
            // quitting so the app keeps running in the system tray.  Skipped
            // when the tray's "Выход" item set user_quit — then we let the
            // window close so app.exit(0) can finish.
            RunEvent::WindowEvent {
                label,
                event: tauri::WindowEvent::CloseRequested { api, .. },
                ..
            } if label == player::WINDOW_LABEL => {
                let user_quit = app_handle
                    .try_state::<AppState>()
                    .map(|s| s.user_quit.load(Ordering::SeqCst))
                    .unwrap_or(false);
                if !user_quit {
                    api.prevent_close();
                    if let Some(w) = app_handle.get_webview_window(&label) {
                        let _ = w.set_skip_taskbar(true);
                        let _ = w.hide();
                    }
                }
            }
            // ExitRequested fires when Tauri thinks the last window is gone
            // (e.g. user used a window-manager close that we didn't catch
            // via WindowEvent).  Block the implicit exit so the app keeps
            // running in the tray; allow it through only when the tray's
            // "Выход" set user_quit.
            RunEvent::ExitRequested { api, .. } => {
                let user_quit = app_handle
                    .try_state::<AppState>()
                    .map(|s| s.user_quit.load(Ordering::SeqCst))
                    .unwrap_or(false);
                if !user_quit {
                    api.prevent_exit();
                }
            }
            RunEvent::Exit => {
                // Mark Player as destroyed *before* calling mpv().destroy() so
                // any in-flight command (position-emitter tick, tray callback)
                // short-circuits rather than tripping the plugin's internal
                // unwrap on an already-removed instance.
                if let Some(state) = app_handle.try_state::<AppState>() {
                    state.player.mark_destroyed();
                }
                if let Err(e) = app_handle.mpv().destroy(player::WINDOW_LABEL) {
                    tracing::warn!("mpv destroy on exit failed: {e}");
                }
            }
            _ => {}
        });
}
