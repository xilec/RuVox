pub mod pipeline;
pub mod player;
pub mod storage;
pub mod tray;
pub mod tts;

use std::sync::Arc;

use player::Player;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_mpv::init())
        .setup(|app| {
            tray::init(app.handle())?;
            let player = Arc::new(Player::new(app.handle().clone())?);
            player::spawn_position_emitter(player.clone(), app.handle().clone());
            app.manage(player);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
