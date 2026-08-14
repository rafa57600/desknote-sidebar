mod appbar;
mod commands;

use appbar::AppBarManager;
use commands::{
    exit_app, get_appbar_state, list_monitors, refresh_sidebar_position, register_with_window,
    set_sidebar_always_on_top, set_sidebar_edge, set_sidebar_monitor, set_sidebar_width,
};
use std::io;
use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(AppBarManager::new())
        .invoke_handler(tauri::generate_handler![
            get_appbar_state,
            set_sidebar_width,
            set_sidebar_edge,
            set_sidebar_always_on_top,
            set_sidebar_monitor,
            refresh_sidebar_position,
            list_monitors,
            exit_app,
        ])
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "main window not found"))?;

            register_with_window(app, &window).map_err(io::Error::other)?;

            // The native AppBar code also sets z-order and position. These Tauri settings
            // keep behavior predictable before/after AppBar transitions.
            window.set_resizable(true)?;
            window.set_shadow(false)?;
            window.set_skip_taskbar(true)?;
            window.show()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Tauri application");

    app.run(|app_handle, event| {
        if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
            if let Some(state) = app_handle.try_state::<AppBarManager>() {
                state.unregister();
            }
        }
    });
}
