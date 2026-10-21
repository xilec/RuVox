use tauri::{
    App, AppHandle, Emitter, Manager, Runtime,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::state::AppState;

/// Tray icon id: used both when building the icon and when looking it up to
/// follow the hide-on-close setting.
const TRAY_ID: &str = "main";

/// Register the tray-menu handlers once for the whole application lifetime.
///
/// Tauri keeps menu-event handlers in a global list that is never pruned, so
/// registering them per icon would make every tray-menu click fire once per
/// icon ever created (each "Добавить" would add and narrate the clipboard
/// entry several times). The icon itself is created and removed by
/// [`set_enabled`], the handlers stay.
pub fn register_menu_events<R: Runtime>(app: &App<R>) {
    app.on_menu_event(|app, event| match event.id.as_ref() {
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
    });
}

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

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        // Include the version so bug reports can quote it from the tray.
        .tooltip(format!("RuVox v{}", app.package_info().version))
        .icon(load_tray_icon(app)?)
        .menu(&menu)
        .show_menu_on_left_click(false)
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

/// Create or remove the tray icon to match `UIConfig.hide_to_tray_on_close`.
///
/// With hide-on-close off the app quits together with its window, so an icon
/// would have nothing to restore; it must also leave the notification area
/// rather than stay visible. The icon is built at startup only when the
/// persisted setting is enabled, and toggling later adds or removes it.
///
/// Creating the icon can fail (no system tray, read-only runtime dir); the
/// error is returned so the caller can keep the persisted setting in sync
/// with the reality.
pub fn set_enabled<R: Runtime>(app: &AppHandle<R>, enabled: bool) -> tauri::Result<()> {
    match (enabled, app.tray_by_id(TRAY_ID).is_some()) {
        (true, false) => init(app),
        (false, true) => {
            if let Some(tray) = app.remove_tray_by_id(TRAY_ID) {
                // Dropping a TrayIcon touches the GTK/macOS backend, which is
                // main-thread-only; Tauri marshals its tray methods but cannot
                // do so for `Drop`, hence the explicit hop. The scheduling
                // failure case means the event loop is already gone (app
                // exit), where nothing else can be done with the handle.
                if let Err(e) = app.run_on_main_thread(move || drop(tray)) {
                    tracing::warn!("tray: failed to schedule icon removal: {e}");
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Whether the tray icon currently exists.
///
/// The close gate uses it: a window may only be hidden into the tray while
/// there is a tray icon to restore it from.
pub fn is_present<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.tray_by_id(TRAY_ID).is_some()
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

    // The menu handler cannot call the `add_clipboard_entry` Tauri command
    // directly (commands need a webview invoke context), so it forwards the
    // request through the tray command channel stored in AppState. The
    // background loop started in `setup()` performs the clipboard read and
    // kicks off synthesis.
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

#[cfg(test)]
mod tests {
    use super::{is_present, set_enabled};
    use crate::test_support::build_test_app;

    /// Without the setting there is no tray icon at all, and turning the
    /// setting off again is a no-op.
    #[test]
    fn no_icon_while_the_setting_is_off() {
        let test_app = build_test_app();
        let handle = test_app.app.handle().clone();
        assert!(!is_present(&handle));

        set_enabled(&handle, false).expect("disabling the tray is a no-op");
        assert!(!is_present(&handle));
    }

    /// Enabling the tray either creates the icon or reports the failure — it
    /// never leaves the app claiming the tray exists while it does not.
    #[test]
    fn enabling_reports_whether_the_icon_exists() {
        let test_app = build_test_app();
        let handle = test_app.app.handle().clone();
        match set_enabled(&handle, true) {
            Ok(()) => assert!(is_present(&handle)),
            Err(_) => assert!(!is_present(&handle)),
        }
    }
}
