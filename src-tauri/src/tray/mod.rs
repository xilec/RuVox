use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};

pub fn init<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let play = MenuItem::with_id(app, "play", "Воспроизвести", false, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Пауза", false, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let read_now = MenuItem::with_id(app, "read_now", "Читать сразу", true, None::<&str>)?;
    let read_later = MenuItem::with_id(app, "read_later", "Читать отложенно", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let settings = MenuItem::with_id(app, "settings", "Настройки...", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "Открыть окно", true, None::<&str>)?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &play,
            &pause,
            &sep1,
            &read_now,
            &read_later,
            &sep2,
            &settings,
            &show,
            &sep3,
            &quit,
        ],
    )?;

    let _tray = TrayIconBuilder::with_id("main")
        .tooltip("RuVox")
        .icon(load_tray_icon(app)?)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            // Invoke synthesis directly via AppState to avoid a webview round-trip.
            // The closure's `app` is `AppHandle<R>` but at runtime R == Wry.
            "read_now" => invoke_add_clipboard_entry(app, true),
            "read_later" => invoke_add_clipboard_entry(app, false),
            "settings" => {
                let _ = app.emit("tray_open_settings", ());
            }
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "play" => {
                let _ = app.emit("tray_play", ());
            }
            "pause" => {
                let _ = app.emit("tray_pause", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// Directly invoke add_clipboard_entry logic via AppState.
///
/// The menu event handler receives `AppHandle<R>` where `R` is the runtime type
/// used at startup (always `Wry` in production). We use `try_state::<AppState>`
/// which is registered with the concrete `Wry` runtime.
fn invoke_add_clipboard_entry<R: Runtime>(app: &AppHandle<R>, play_when_ready: bool) {
    use crate::state::AppState;

    let state = match app.try_state::<AppState>() {
        Some(s) => s,
        None => {
            tracing::warn!("tray: AppState not yet ready");
            return;
        }
    };

    // Retrieve the Wry-typed AppHandle stored in AppState's player.
    // We obtain it by casting through a channel: spawn a task on the Tokio runtime
    // and pass the player's handle (which is already Wry-typed) from the managed state.
    //
    // Since AppState stores `Player<Wry>`, the player's `app` field is `AppHandle<Wry>`.
    // We need an `AppHandle<Wry>` for spawn_synthesis_pub. The player already holds one,
    // but there's no public accessor for it. As a pragmatic solution we use a dedicated
    // tray command channel stored in AppState.
    //
    // For now, use the tray-owned AppHandle and clone it to Wry via the command sender
    // stored in AppState (added below). This is a placeholder until the tray_tx channel
    // is wired; fall back to emitting a tray event that the frontend forwards.
    if let Some(sender) = state.tray_cmd_tx.as_ref() {
        let _ = sender.try_send(TrayCmd { play_when_ready });
    } else {
        // No channel yet — emit an event for the frontend to pick up.
        let event = if play_when_ready { "tray_read_now" } else { "tray_read_later" };
        let _ = app.emit(event, ());
    }
}

/// Message sent from the tray menu handler to the main synthesis loop.
pub struct TrayCmd {
    pub play_when_ready: bool,
}

fn load_tray_icon<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<Image<'static>> {
    Image::from_bytes(include_bytes!("../../icons/tray.png"))
}
