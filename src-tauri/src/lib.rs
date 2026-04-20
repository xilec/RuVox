pub mod commands;
pub mod pipeline;
pub mod player;
pub mod state;
pub mod storage;
pub mod tray;
pub mod tts;

use std::sync::Arc;

use parking_lot::Mutex;
use serde_json::json;
use tauri::{Emitter, Manager};

use commands::*;
use pipeline::TTSPipeline;
use player::Player;
use state::AppState;
use storage::service::StorageService;
use tray::TrayCmd;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_mpv::init())
        .setup(|app| {
            tray::init(app.handle())?;

            let player = Arc::new(Player::new(app.handle().clone())?);
            player::spawn_position_emitter(player.clone(), app.handle().clone());

            let storage = Arc::new(
                StorageService::new()
                    .expect("не удалось открыть хранилище"),
            );

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
                tauri::async_runtime::block_on(async move {
                    tts::TtsSubprocess::spawn(ttsd_dir)
                })
                .expect("не удалось запустить ttsd subprocess"),
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
                            let _ = app_handle.emit("model_error", json!({ "message": e.to_string() }));
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
            };
            app.manage(app_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            add_clipboard_entry,
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
            update_entry_edited_text,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
