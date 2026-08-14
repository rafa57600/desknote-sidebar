use serde::Serialize;
use std::sync::Mutex;
use tauri::WebviewWindow;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    Left,
    Right,
}

impl Edge {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            _ => Err("edge must be 'left' or 'right'".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AppBarState {
    pub supported: bool,
    pub registered: bool,
    pub width: u32,
    pub edge: Edge,
    pub always_on_top: bool,
}

pub struct AppBarManager {
    state: Mutex<AppBarState>,
}

impl AppBarManager {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(AppBarState {
                supported: false,
                registered: false,
                width: 360,
                edge: Edge::Right,
                always_on_top: true,
            }),
        }
    }

    pub fn register(&self, _window: &WebviewWindow) -> Result<AppBarState, String> {
        Ok(self.state())
    }
    pub fn unregister(&self) {}
    pub fn state(&self) -> AppBarState {
        self.state.lock().map(|s| s.clone()).unwrap_or(AppBarState {
            supported: false,
            registered: false,
            width: 360,
            edge: Edge::Right,
            always_on_top: true,
        })
    }
    pub fn set_width(&self, width: u32) -> Result<AppBarState, String> {
        if let Ok(mut s) = self.state.lock() {
            s.width = width.clamp(48, 720);
        }
        Ok(self.state())
    }
    pub fn set_edge(&self, edge: Edge) -> Result<AppBarState, String> {
        if let Ok(mut s) = self.state.lock() {
            s.edge = edge;
        }
        Ok(self.state())
    }
    pub fn set_always_on_top(&self, enabled: bool) -> Result<AppBarState, String> {
        if let Ok(mut s) = self.state.lock() {
            s.always_on_top = enabled;
        }
        Ok(self.state())
    }
    pub fn set_monitor_anchor(&self, _x: i32, _y: i32) -> Result<AppBarState, String> {
        Ok(self.state())
    }
    pub fn refresh(&self) -> Result<AppBarState, String> {
        Ok(self.state())
    }
}
