// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Must run before ruvox_tauri_lib::run() starts Tokio/Tauri threads —
    // see the fn's docs.
    ruvox_tauri_lib::init_platform_env();
    ruvox_tauri_lib::run()
}
