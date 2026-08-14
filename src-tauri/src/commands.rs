use crate::appbar::{AppBarManager, AppBarState, Edge};
use serde::Serialize;
use tauri::{AppHandle, Manager, State, WebviewWindow};

#[derive(Debug, Clone, Serialize)]
pub struct MonitorInfo {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub is_current: bool,
}

#[tauri::command]
pub fn get_appbar_state(state: State<'_, AppBarManager>) -> AppBarState {
    state.state()
}

#[tauri::command]
pub fn set_sidebar_width(
    state: State<'_, AppBarManager>,
    width: u32,
) -> Result<AppBarState, String> {
    state.set_width(width)
}

#[tauri::command]
pub fn set_sidebar_edge(
    state: State<'_, AppBarManager>,
    edge: String,
) -> Result<AppBarState, String> {
    state.set_edge(Edge::parse(&edge)?)
}

#[tauri::command]
pub fn set_sidebar_always_on_top(
    state: State<'_, AppBarManager>,
    window: WebviewWindow,
    enabled: bool,
) -> Result<AppBarState, String> {
    window
        .set_always_on_top(enabled)
        .map_err(|error| error.to_string())?;
    state.set_always_on_top(enabled)
}

#[tauri::command]
pub fn set_sidebar_monitor(
    state: State<'_, AppBarManager>,
    x: i32,
    y: i32,
) -> Result<AppBarState, String> {
    state.set_monitor_anchor(x, y)
}

#[tauri::command]
pub fn refresh_sidebar_position(
    state: State<'_, AppBarManager>,
) -> Result<AppBarState, String> {
    state.refresh()
}

#[tauri::command]
pub fn list_monitors(window: WebviewWindow) -> Result<Vec<MonitorInfo>, String> {
    let current = window.current_monitor().map_err(|error| error.to_string())?;
    let current_position = current.as_ref().map(|m| *m.position());
    let monitors = window.available_monitors().map_err(|error| error.to_string())?;

    Ok(monitors
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| {
            let position = *monitor.position();
            let size = *monitor.size();
            let name = monitor
                .name()
                .cloned()
                .unwrap_or_else(|| format!("Monitor {}", index + 1));
            let id = format!(
                "{}|{}:{}|{}x{}",
                name, position.x, position.y, size.width, size.height
            );
            MonitorInfo {
                id,
                name,
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
                scale_factor: monitor.scale_factor(),
                is_current: current_position
                    .map(|current| current.x == position.x && current.y == position.y)
                    .unwrap_or(false),
            }
        })
        .collect())
}

#[tauri::command]
pub fn exit_app(app: AppHandle, state: State<'_, AppBarManager>) {
    state.unregister();
    app.exit(0);
}

pub fn register_with_window(
    app: &tauri::App,
    window: &WebviewWindow,
) -> Result<AppBarState, String> {
    let state = app.state::<AppBarManager>();
    state.register(window)
}
