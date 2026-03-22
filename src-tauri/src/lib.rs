mod api;

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    AppHandle, Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

static LOCKED: AtomicBool = AtomicBool::new(true);

#[tauri::command]
fn start_drag(window: tauri::WebviewWindow) {
    let _ = window.start_dragging();
}

fn apply_lock_state(app_handle: &AppHandle, locked: bool) {
    LOCKED.store(locked, Ordering::SeqCst);

    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.set_ignore_cursor_events(locked);
        let _ = window.set_resizable(!locked);
    }

    if let Some(tray) = app_handle.tray_by_id("main-tray") {
        let label = if locked { "Unlock" } else { "Lock" };
        let toggle_i = MenuItem::with_id(app_handle, "toggle", label, true, None::<&str>).unwrap();
        let quit_i = MenuItem::with_id(app_handle, "quit", "Quit", true, None::<&str>).unwrap();
        let menu = Menu::with_items(app_handle, &[&toggle_i, &quit_i]).unwrap();
        let _ = tray.set_menu(Some(menu));
    }

    let _ = app_handle.emit("lock-changed", locked);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let _ = window.set_ignore_cursor_events(true);

            let toggle_i = MenuItem::with_id(app, "toggle", "Unlock", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle_i, &quit_i])?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app_handle: &AppHandle, event| {
                    match event.id.as_ref() {
                        "toggle" => {
                            let currently_locked = LOCKED.load(Ordering::SeqCst);
                            apply_lock_state(app_handle, !currently_locked);
                        }
                        "quit" => {
                            app_handle.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            let app_handle_clone = app.handle().clone();
            std::thread::spawn(move || {
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(api::start_server(app_handle_clone));
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![start_drag])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
