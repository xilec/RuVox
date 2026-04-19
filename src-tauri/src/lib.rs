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
            let ttsd_dir = app
                .path()
                .resource_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join("ttsd");
            let ttsd_dir = if ttsd_dir.exists() {
                ttsd_dir
            } else {
                std::path::PathBuf::from("ttsd")
            };

            let tts = Arc::new(
                tts::TtsSubprocess::spawn(ttsd_dir)
                    .expect("не удалось запустить ttsd subprocess"),
            );

            // Warm up Silero model in background; emit model_loading → model_loaded/model_error.
            {
                let tts_clone = Arc::clone(&tts);
                let app_handle = app.handle().clone();
                tokio::spawn(async move {
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

                tokio::spawn(async move {
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
