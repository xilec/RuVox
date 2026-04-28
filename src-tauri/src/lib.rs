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
use tauri::{Emitter, Manager, RunEvent};
use tauri_plugin_mpv::MpvExt;

use commands::*;
use pipeline::TTSPipeline;
use player::Player;
use state::AppState;
use storage::service::StorageService;
use tray::TrayCmd;

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

pub fn run() {
    reap_orphan_mpv();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_mpv::init())
        .setup(|app| {
            tray::init(app.handle())?;

            // WindowConfig in tauri.conf.json has no `icon` field, and the
            // bundle-level icons only get wired up by `cargo tauri build`.
            // For dev (and as a hardening for release), set the window icon
            // explicitly so X11 / app switchers / task bars pick it up.
            // Linux GTK CSD title bars typically don't render this icon
            // inside the title bar itself — that is a GNOME/KDE chrome
            // decision, not a Tauri limitation.
            if let Some(window) = app.get_webview_window("main") {
                let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/128x128.png"))?;
                let _ = window.set_icon(icon);
            }

            let player = Arc::new(Player::new(app.handle().clone())?);
            player::spawn_position_emitter(player.clone(), app.handle().clone());

            let storage = Arc::new(StorageService::new().expect("failed to open storage"));

            // One-shot migration: any pre-Opus `.wav` audio in history gets
            // transcoded to `.opus` in the background. Runs blocking on a
            // dedicated thread so app startup is not delayed; per-entry
            // errors are logged inside the migration routine.
            {
                let storage_for_migration = Arc::clone(&storage);
                tauri::async_runtime::spawn(async move {
                    let stats = tokio::task::spawn_blocking(move || {
                        storage_for_migration.migrate_wav_audio_to_opus()
                    })
                    .await;
                    match stats {
                        Ok(s) if s.considered == 0 => {
                            tracing::debug!("audio migration: nothing to do");
                        }
                        Ok(s) => {
                            tracing::info!(
                                "audio migration: considered={}, migrated={}, skipped_missing={}, failed={}",
                                s.considered, s.migrated, s.skipped_missing, s.failed
                            );
                        }
                        Err(e) => {
                            tracing::error!("audio migration task panicked: {e}");
                        }
                    }
                });
            }

            // Spawn ttsd subprocess.
            // In production: bundled next to the binary (resource_dir/ttsd).
            // In `cargo tauri dev`: cwd is src-tauri/, so the project ttsd lives
            // at ../ttsd; fall back to ./ttsd for ad-hoc runs from the repo root.
            let ttsd_dir = {
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
            };

            // tokio::process::Command::spawn requires an active tokio runtime
            // context; setup hook runs synchronously, so enter Tauri's runtime
            // explicitly via block_on (the inner spawn returns instantly).
            let tts = Arc::new(
                tauri::async_runtime::block_on(async move { tts::TtsSubprocess::spawn(ttsd_dir) })
                    .expect("failed to spawn ttsd subprocess"),
            );

            // Warm up Silero model in background; emit model_loading → model_loaded/model_error.
            {
                let tts_clone = Arc::clone(&tts);
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let _ = app_handle.emit("model_loading", json!({}));
                    match tts_clone.warmup().await {
                        Ok(()) => {
                            tracing::info!("ttsd warmup ok");
                            let _ = app_handle.emit("model_loaded", json!({}));
                        }
                        Err(e) => {
                            tracing::error!("ttsd warmup failed: {e}");
                            let _ =
                                app_handle.emit("model_error", json!({ "message": e.to_string() }));
                        }
                    }
                });
            }

            let pipeline = Arc::new(Mutex::new(TTSPipeline::new()));

            // Create a channel for tray menu commands (read_now / read_later).
            let (tray_tx, mut tray_rx) = tokio::sync::mpsc::channel::<TrayCmd>(16);

            // Spawn the tray command handler loop.
            {
                let storage_clone = Arc::clone(&storage);
                let tts_clone = Arc::clone(&tts);
                let player_clone = Arc::clone(&player);
                let pipeline_clone = Arc::clone(&pipeline);
                let app_handle = app.handle().clone();

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

                        let entry = match storage_clone.add_entry(text) {
                            Ok(e) => e,
                            Err(e) => {
                                tracing::error!("tray: failed to add entry: {e}");
                                continue;
                            }
                        };

                        let entry_id = entry.id;
                        let _ = app_handle.emit("entry_updated", json!({ "entry": entry }));

                        spawn_synthesis_pub(
                            app_handle.clone(),
                            Arc::clone(&storage_clone),
                            Arc::clone(&tts_clone),
                            Arc::clone(&player_clone),
                            Arc::clone(&pipeline_clone),
                            entry_id,
                            cmd.play_when_ready,
                        );
                    }
                });
            }

            let app_state = AppState {
                storage,
                tts,
                player,
                pipeline,
                tray_cmd_tx: Some(tray_tx),
                user_quit: Arc::new(AtomicBool::new(false)),
            };
            app.manage(app_state);

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
            get_timestamps,
            clear_cache,
            get_cache_stats,
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
