pub mod audio;
pub mod commands;
pub mod paths;
pub mod pipeline;
pub mod player;
pub mod state;
pub mod storage;
pub mod tray;
pub mod tts;

#[cfg(test)]
mod test_support;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, RunEvent, Runtime};
use tauri_plugin_mpv::MpvExt;

use commands::*;
use pipeline::TTSPipeline;
use player::{Player, PlayerBackend};
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
///
/// Unix-only: on Windows the plugin uses named pipes instead of /tmp
/// sockets and there is no /proc — the code would not even compile
/// (`libc::kill`).  A crashed mpv child there exits with its parent or
/// lingers harmlessly until reboot; a named-pipe reaper is a non-goal.
#[cfg(unix)]
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
/// - `engine = "piper"` → in-process [`tts::PiperEngine`].
/// - `engine = "silero"` *and* probe says Silero is available →
///   [`tts::TtsSupervisor`] over `uv run python -m ttsd`. Spawn failure
///   (race between probe and spawn) silently falls back to Piper.
/// - `engine = "silero"` but probe says unavailable → silent migration to
///   Piper; the user's `engine` value on disk is left untouched so they
///   can roll back by installing the Silero stack.
/// - `engine = "silero_native"` (default) *and* the model bundle is on disk →
///   in-process [`tts::SileroNativeEngine`]. A missing bundle falls back
///   to Piper the same way; the config value is preserved.
///
/// Voice models live at `<voices_root>/piper/<voice>/…` — see
/// `tts::piper::catalog`. The Silero Native model bundle lives next to
/// them at `<voices_root>/silero-native/` and is fetched on demand via
/// `download_silero_native_bundle`. The voices root itself is resolved
/// per-OS by [`crate::paths::voices_root`].
/// Returns the active engine layer plus the runtime paths and emitter the
/// rest of the app needs (Phase 4 download command, Phase 3 probe).
type EngineWiring = (
    Arc<tts::EngineSwitcher>,
    std::path::PathBuf, // ttsd_dir
    std::path::PathBuf, // piper_voices_dir
    std::path::PathBuf, // silero_native_bundle_dir
    tts::supervisor::Emitter,
);

fn build_engine<R: Runtime>(
    app: &AppHandle<R>,
    storage: &StorageService,
) -> Result<EngineWiring, SetupError> {
    let data_dir = crate::paths::voices_root().ok_or("no per-user data dir for voices")?;
    let voices_dir = data_dir.join("piper");
    let silero_native_bundle_dir = data_dir.join("silero-native");

    let app_handle_for_emitter = app.clone();
    let emitter: tts::supervisor::Emitter = Arc::new(move |event_name, payload| {
        let _ = app_handle_for_emitter.emit(event_name, payload);
    });

    let config = storage.load_config().unwrap_or_default();
    let ttsd_dir = resolve_ttsd_dir(app);

    let want_silero = config.engine == "silero";
    // Targeted probe: the full availability::probe would also stat the
    // silero-native bundle, whose result is not used on this path.
    let silero_available = want_silero && tts::availability::probe_silero(&ttsd_dir).available;
    // Stat-only gate (manifest parses, every listed file present with the
    // recorded size) — the engine's warmup runs the full sha256 verification
    // before opening ONNX sessions.
    let want_silero_native = config.engine == "silero_native"
        && tts::availability::probe_silero_native(&silero_native_bundle_dir).available;

    let (initial_engine, initial_kind, initial_voice) = if want_silero_native {
        let engine: Arc<dyn tts::TtsEngine> = Arc::new(tts::SileroNativeEngine::new(
            silero_native_bundle_dir.clone(),
            Arc::clone(&emitter),
        ));
        (engine, tts::EngineKind::SileroNative, None)
    } else if silero_available {
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
        if config.engine == "silero_native" {
            tracing::info!(
                "Silero Native requested in config but the bundle is missing or incomplete; serving Piper this run"
            );
        }
        build_piper_initial(&voices_dir, &config.piper_voice, &emitter)
    };

    let switcher = Arc::new(tts::EngineSwitcher::new(
        initial_engine,
        initial_kind,
        initial_voice,
        voices_dir.clone(),
        ttsd_dir.clone(),
        silero_native_bundle_dir.clone(),
        Arc::clone(&emitter),
    ));
    Ok((
        switcher,
        ttsd_dir,
        voices_dir,
        silero_native_bundle_dir,
        emitter,
    ))
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
    player: Arc<dyn PlayerBackend>,
    pipeline: Arc<Mutex<TTSPipeline>>,
    synthesis_tasks: Arc<Mutex<HashMap<storage::schema::EntryId, tokio::task::AbortHandle>>>,
    synthesize_entered: Arc<Mutex<HashSet<storage::schema::EntryId>>>,
    app: AppHandle<R>,
) -> tokio::sync::mpsc::Sender<TrayCmd> {
    let (tray_tx, mut tray_rx) = tokio::sync::mpsc::channel::<TrayCmd>(16);

    tauri::async_runtime::spawn(async move {
        let deps = SynthesisDeps {
            app: app.clone(),
            storage: Arc::clone(&storage),
            tts: Arc::clone(&tts),
            piper_voices_dir,
            emitter,
            player,
            pipeline,
            synthesis_tasks,
            synthesize_entered,
        };
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

            spawn_synthesis(deps.clone(), entry_id, cmd.play_when_ready);
        }
    });

    tray_tx
}

/// The single registration point for all Tauri commands.
///
/// Extracted from `run()` so the test harness (`test_support`) registers the
/// identical command set on its `MockRuntime` app — no drift between the
/// production handler list and what tests exercise.
pub(crate) fn invoke_handler<R: Runtime>()
-> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        add_clipboard_entry,
        add_text_entry,
        preview_normalize,
        get_entries,
        get_entry,
        delete_entry,
        delete_audio,
        regenerate_entry,
        set_entry_format,
        cancel_synthesis,
        play_entry,
        pause_playback,
        resume_playback,
        stop_playback,
        shutdown_player_for_update,
        seek_to,
        set_speed,
        set_volume,
        get_config,
        update_config,
        get_available_engines,
        download_piper_voice,
        download_silero_native_bundle,
        get_timestamps,
        clear_cache,
        get_cache_stats,
        get_cache_dir,
        get_log_dir,
    ]
}

/// Platform-specific startup environment. Windows: point piper-rs at the
/// bundled `espeak-ng-data/` directory (espeak-rs expects
/// `PIPER_ESPEAKNG_DATA_DIRECTORY` to name the directory *containing*
/// `espeak-ng-data`). MUST be called at the top of `main`, before Tokio or
/// Tauri spawn any threads — `std::env::set_var` is `unsafe` in edition
/// 2024 because it races with concurrent environment readers.
///
/// Linux gets the variable from the nix wrapper instead (flake.nix
/// preFixup); dev runs on Windows without the bundled dir fall back to
/// Piper's built-in search (degraded Russian stress, still functional).
#[cfg(windows)]
pub fn init_platform_env() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(exe_dir) = exe.parent() else { return };
    if exe_dir.join("espeak-ng-data").exists() {
        // SAFETY: called from main before any threads exist.
        unsafe { std::env::set_var("PIPER_ESPEAKNG_DATA_DIRECTORY", exe_dir) };
    }
}

/// No-op on platforms whose environment is provided by the packaging
/// (nix wrapper sets `PIPER_ESPEAKNG_DATA_DIRECTORY` on Linux).
#[cfg(not(windows))]
pub fn init_platform_env() {}

/// Diagnostic logging: LogDir always, Stdout in debug builds (#202).
/// `RUST_LOG` overrides the level (level names only, default `info`).
fn logging_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

    let level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|v| v.parse::<log::LevelFilter>().ok())
        .unwrap_or(log::LevelFilter::Info);

    let mut targets = vec![Target::new(TargetKind::LogDir { file_name: None })];
    if cfg!(debug_assertions) {
        targets.push(Target::new(TargetKind::Stdout));
    }

    tauri_plugin_log::Builder::new()
        .targets(targets)
        .rotation_strategy(RotationStrategy::KeepSome(4))
        .max_file_size(5 * 1024 * 1024)
        .level(level)
        .build()
}

pub fn run() {
    #[cfg(unix)]
    reap_orphan_mpv();
    tauri::Builder::default()
        // Registered first so the other plugins' init records are captured.
        .plugin(logging_plugin())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_mpv::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            tray::init(app.handle())?;
            install_window_icon(app.handle())?;

            let player: Arc<dyn PlayerBackend> = Arc::new(Player::new(app.handle().clone())?);
            player::spawn_position_emitter(Arc::clone(&player), app.handle().clone());

            let storage = Arc::new(StorageService::new().expect("failed to open storage"));
            spawn_audio_migration_and_cleanup(Arc::clone(&storage));

            let (engine_switcher, ttsd_dir, piper_voices_dir, silero_native_bundle_dir, emitter) =
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
            let synthesis_tasks = Arc::new(Mutex::new(HashMap::new()));
            let synthesize_entered = Arc::new(Mutex::new(HashSet::new()));
            let tray_tx = spawn_tray_handler(
                Arc::clone(&storage),
                Arc::clone(&tts),
                piper_voices_dir.clone(),
                Arc::clone(&emitter),
                Arc::clone(&player),
                Arc::clone(&pipeline),
                Arc::clone(&synthesis_tasks),
                Arc::clone(&synthesize_entered),
                app.handle().clone(),
            );

            app.manage(AppState {
                storage,
                tts,
                engine_switcher,
                ttsd_dir,
                piper_voices_dir,
                silero_native_bundle_dir,
                emitter,
                player,
                pipeline,
                tray_cmd_tx: Some(tray_tx),
                user_quit: Arc::new(AtomicBool::new(false)),
                synthesis_tasks,
                synthesize_entered,
            });

            Ok(())
        })
        .invoke_handler(invoke_handler())
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
