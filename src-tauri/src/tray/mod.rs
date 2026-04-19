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
            // TODO(B4): replace emits with direct `add_clipboard_entry` invocations once
            // Tauri commands are implemented.
            "read_now" => {
                let _ = app.emit("tray_read_now", ());
            }
            "read_later" => {
                let _ = app.emit("tray_read_later", ());
            }
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

fn load_tray_icon<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<Image<'static>> {
    // Embed PNG at compile time — no resource_dir dependency.
    Image::from_bytes(include_bytes!("../../icons/tray.png"))
}
