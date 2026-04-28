use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};

use crate::state::AppState;

pub fn init<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    // Order rationale: libayatana-appindicator (Linux/KDE/GNOME) opens the
    // menu on every tray-icon click — there are no raw click events.  Put
    // "Открыть окно" at the top so a quick click + first-item-pick is the
    // shortest path back to the window.
    let show = MenuItem::with_id(app, "show", "Открыть окно", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let add = MenuItem::with_id(app, "add", "Добавить", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show, &sep1, &add, &sep2, &quit])?;

    let _tray = TrayIconBuilder::with_id("main")
        .tooltip("RuVox")
        .icon(load_tray_icon(app)?)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            // Invoke synthesis directly via AppState to avoid a webview round-trip.
            // The closure's `app` is `AppHandle<R>` but at runtime R == Wry.
            "add" => invoke_add_clipboard_entry(app, true),
            "show" => show_main_window(app),
            "quit" => {
                if let Some(state) = app.try_state::<AppState>() {
                    state
                        .user_quit
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // libayatana-appindicator (used on KDE) often does not propagate
            // DoubleClick events, so a single left click on the tray icon is
            // also wired to show the main window.  Filter on `Up` to fire
            // exactly once per click.
            let should_show = matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            );
            if should_show {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_skip_taskbar(false);
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
    // tauri-plugin-mpv destroys mpv whenever the main window's
    // CloseRequested fires (its own RunEvent handler), so the subprocess is
    // gone after a tray-on-close cycle.  Re-init lazily on every show so
    // playback works after the user reopens the window.
    if let Some(state) = app.try_state::<AppState>() {
        if let Err(e) = state.player.ensure_mpv_alive() {
            tracing::error!("ensure_mpv_alive failed: {e}");
        }
    }
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
        let event = if play_when_ready {
            "tray_read_now"
        } else {
            "tray_read_later"
        };
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
