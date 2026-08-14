use serde::Serialize;
use std::{
    ffi::c_void,
    mem::{size_of, size_of_val},
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering},
        Mutex,
    },
};
use tauri::WebviewWindow;
use windows::{
    core::w,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::{
            Dwm::{
                DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE,
                DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
            },
            Gdi::{
                GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO,
                MONITOR_DEFAULTTONEAREST,
            },
        },
        UI::{
            HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
            Shell::{
                DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass, APPBARDATA,
                ABE_LEFT, ABE_RIGHT, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS,
                ABM_WINDOWPOSCHANGED, ABN_FULLSCREENAPP, ABN_POSCHANGED, SHAppBarMessage,
            },
            WindowsAndMessaging::{
                RegisterWindowMessageW, SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST,
                SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, WM_DESTROY,
                WM_DISPLAYCHANGE, WM_DPICHANGED, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE,
                WM_SETTINGCHANGE, WM_SIZING, WM_WINDOWPOSCHANGED, WMSZ_LEFT, WMSZ_RIGHT,
            },
        },
    },
};

const SUBCLASS_ID: usize = 0x4453_4E54; // "DSNT"
const MIN_LOGICAL_WIDTH: u32 = 48;
const MIN_INTERACTIVE_LOGICAL_WIDTH: u32 = 280;
const MAX_LOGICAL_WIDTH: u32 = 720;

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

    fn as_shell_edge(self) -> u32 {
        match self {
            Self::Left => ABE_LEFT,
            Self::Right => ABE_RIGHT,
        }
    }

    fn as_atomic(self) -> u32 {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }

    fn from_atomic(value: u32) -> Self {
        if value == 0 { Self::Left } else { Self::Right }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AppBarState {
    pub supported: bool,
    pub registered: bool,
    /// Width in logical pixels (96-DPI units), not physical monitor pixels.
    pub width: u32,
    pub edge: Edge,
    pub always_on_top: bool,
}

struct SubclassContext {
    callback_message: u32,
    width: AtomicU32,
    edge: AtomicU32,
    always_on_top: AtomicBool,
    anchor_x: AtomicI32,
    anchor_y: AtomicI32,
    registered: AtomicBool,
    interactive_sizing: AtomicBool,
    in_layout: AtomicBool,
}

struct Registration {
    hwnd_value: isize,
    context_ptr: usize,
}

struct Inner {
    registration: Option<Registration>,
    width: u32,
    edge: Edge,
    always_on_top: bool,
}

pub struct AppBarManager {
    inner: Mutex<Inner>,
}

impl Default for AppBarManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBarManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                registration: None,
                width: 360,
                edge: Edge::Right,
                always_on_top: true,
            }),
        }
    }

    pub fn register(&self, window: &WebviewWindow) -> Result<AppBarState, String> {
        let mut inner = self.inner.lock().map_err(|_| "AppBar state lock poisoned".to_string())?;
        sync_inner_from_context(&mut inner);
        if inner.registration.is_some() {
            return Ok(state_from_inner(&inner));
        }

        let tauri_hwnd = window.hwnd().map_err(|error| error.to_string())?;
        let hwnd_value = tauri_hwnd.0 as isize;
        let hwnd = hwnd_from_value(hwnd_value);

        let callback_message = unsafe { RegisterWindowMessageW(w!("DeskNote.Sidebar.AppBar.Callback.v2")) };
        if callback_message == 0 {
            return Err("RegisterWindowMessageW failed".to_string());
        }

        let (anchor_x, anchor_y) = unsafe { monitor_center_for_window(hwnd)? };
        let context = Box::new(SubclassContext {
            callback_message,
            width: AtomicU32::new(inner.width),
            edge: AtomicU32::new(inner.edge.as_atomic()),
            always_on_top: AtomicBool::new(inner.always_on_top),
            anchor_x: AtomicI32::new(anchor_x),
            anchor_y: AtomicI32::new(anchor_y),
            registered: AtomicBool::new(false),
            interactive_sizing: AtomicBool::new(false),
            in_layout: AtomicBool::new(false),
        });
        let context_ptr = Box::into_raw(context) as usize;

        let register_result = unsafe {
            let mut data = appbar_data(hwnd);
            data.uCallbackMessage = callback_message;
            SHAppBarMessage(ABM_NEW, &mut data)
        };
        if register_result == 0 {
            unsafe { drop(Box::from_raw(context_ptr as *mut SubclassContext)); }
            return Err("Windows Shell rejected ABM_NEW".to_string());
        }

        let subclass_ok = unsafe {
            SetWindowSubclass(hwnd, Some(appbar_subclass_proc), SUBCLASS_ID, context_ptr).as_bool()
        };
        if !subclass_ok {
            unsafe {
                let mut data = appbar_data(hwnd);
                SHAppBarMessage(ABM_REMOVE, &mut data);
                drop(Box::from_raw(context_ptr as *mut SubclassContext));
            }
            return Err("SetWindowSubclass failed".to_string());
        }

        let context_ref = unsafe { &*(context_ptr as *const SubclassContext) };
        context_ref.registered.store(true, Ordering::SeqCst);

        unsafe {
            configure_native_chrome(hwnd);
        }

        if let Err(error) = unsafe { reposition(hwnd, context_ref) } {
            unsafe {
                context_ref.registered.store(false, Ordering::SeqCst);
                let mut data = appbar_data(hwnd);
                SHAppBarMessage(ABM_REMOVE, &mut data);
                let _ = RemoveWindowSubclass(hwnd, Some(appbar_subclass_proc), SUBCLASS_ID);
                drop(Box::from_raw(context_ptr as *mut SubclassContext));
            }
            return Err(error);
        }

        inner.registration = Some(Registration { hwnd_value, context_ptr });
        Ok(state_from_inner(&inner))
    }

    pub fn unregister(&self) {
        let Ok(mut inner) = self.inner.lock() else { return; };
        sync_inner_from_context(&mut inner);
        let Some(registration) = inner.registration.take() else { return; };

        unsafe {
            let hwnd = hwnd_from_value(registration.hwnd_value);
            let context_ptr = registration.context_ptr as *mut SubclassContext;
            if !context_ptr.is_null() {
                let context = &*context_ptr;
                if context.registered.swap(false, Ordering::SeqCst) {
                    let mut data = appbar_data(hwnd);
                    SHAppBarMessage(ABM_REMOVE, &mut data);
                }
                let _ = RemoveWindowSubclass(hwnd, Some(appbar_subclass_proc), SUBCLASS_ID);
                drop(Box::from_raw(context_ptr));
            }
        }
    }

    pub fn state(&self) -> AppBarState {
        let Ok(mut inner) = self.inner.lock() else {
            return AppBarState {
                supported: true,
                registered: false,
                width: 360,
                edge: Edge::Right,
                always_on_top: true,
            };
        };
        sync_inner_from_context(&mut inner);
        state_from_inner(&inner)
    }

    pub fn set_width(&self, width: u32) -> Result<AppBarState, String> {
        let mut inner = self.inner.lock().map_err(|_| "AppBar state lock poisoned".to_string())?;
        inner.width = width.clamp(MIN_LOGICAL_WIDTH, MAX_LOGICAL_WIDTH);
        if let Some(registration) = &inner.registration {
            let context = unsafe { &*(registration.context_ptr as *const SubclassContext) };
            context.width.store(inner.width, Ordering::SeqCst);
            unsafe { reposition(hwnd_from_value(registration.hwnd_value), context)?; }
        }
        Ok(state_from_inner(&inner))
    }

    pub fn set_edge(&self, edge: Edge) -> Result<AppBarState, String> {
        let mut inner = self.inner.lock().map_err(|_| "AppBar state lock poisoned".to_string())?;
        inner.edge = edge;
        if let Some(registration) = &inner.registration {
            let context = unsafe { &*(registration.context_ptr as *const SubclassContext) };
            context.edge.store(edge.as_atomic(), Ordering::SeqCst);
            unsafe { reposition(hwnd_from_value(registration.hwnd_value), context)?; }
        }
        Ok(state_from_inner(&inner))
    }

    pub fn set_always_on_top(&self, enabled: bool) -> Result<AppBarState, String> {
        let mut inner = self.inner.lock().map_err(|_| "AppBar state lock poisoned".to_string())?;
        inner.always_on_top = enabled;
        if let Some(registration) = &inner.registration {
            let context = unsafe { &*(registration.context_ptr as *const SubclassContext) };
            context.always_on_top.store(enabled, Ordering::SeqCst);
            unsafe { apply_z_order(hwnd_from_value(registration.hwnd_value), enabled)?; }
        }
        Ok(state_from_inner(&inner))
    }

    pub fn set_monitor_anchor(&self, x: i32, y: i32) -> Result<AppBarState, String> {
        let mut inner = self.inner.lock().map_err(|_| "AppBar state lock poisoned".to_string())?;
        if let Some(registration) = &inner.registration {
            let context = unsafe { &*(registration.context_ptr as *const SubclassContext) };
            context.anchor_x.store(x, Ordering::SeqCst);
            context.anchor_y.store(y, Ordering::SeqCst);
            unsafe { reposition(hwnd_from_value(registration.hwnd_value), context)?; }
            sync_inner_from_context(&mut inner);
        }
        Ok(state_from_inner(&inner))
    }

    pub fn refresh(&self) -> Result<AppBarState, String> {
        let mut inner = self.inner.lock().map_err(|_| "AppBar state lock poisoned".to_string())?;
        if let Some(registration) = &inner.registration {
            let context = unsafe { &*(registration.context_ptr as *const SubclassContext) };
            unsafe { reposition(hwnd_from_value(registration.hwnd_value), context)?; }
            sync_inner_from_context(&mut inner);
        }
        Ok(state_from_inner(&inner))
    }
}

fn sync_inner_from_context(inner: &mut Inner) {
    let Some(registration) = &inner.registration else { return; };
    if registration.context_ptr == 0 { return; }
    let context = unsafe { &*(registration.context_ptr as *const SubclassContext) };
    inner.width = context.width.load(Ordering::SeqCst);
    inner.edge = Edge::from_atomic(context.edge.load(Ordering::SeqCst));
    inner.always_on_top = context.always_on_top.load(Ordering::SeqCst);
}

fn state_from_inner(inner: &Inner) -> AppBarState {
    AppBarState {
        supported: true,
        registered: inner.registration.is_some(),
        width: inner.width,
        edge: inner.edge,
        always_on_top: inner.always_on_top,
    }
}

fn hwnd_from_value(value: isize) -> HWND {
    HWND(value as *mut c_void)
}

fn appbar_data(hwnd: HWND) -> APPBARDATA {
    let mut data = APPBARDATA::default();
    data.cbSize = size_of::<APPBARDATA>() as u32;
    data.hWnd = hwnd;
    data
}

unsafe fn configure_native_chrome(hwnd: HWND) {
    // Windows 11 can add rounded corners and a one-pixel DWM border even to
    // undecorated windows. An edge-docked AppBar should be visually flush.
    let corners = DWMWCP_DONOTROUND;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_WINDOW_CORNER_PREFERENCE,
        &corners as *const _ as *const c_void,
        size_of_val(&corners) as u32,
    );

    let border_color = DWMWA_COLOR_NONE;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_BORDER_COLOR,
        &border_color as *const _ as *const c_void,
        size_of_val(&border_color) as u32,
    );
}

unsafe fn monitor_center_for_window(hwnd: HWND) -> Result<(i32, i32), String> {
    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut info = MONITORINFO::default();
    info.cbSize = size_of::<MONITORINFO>() as u32;
    if !GetMonitorInfoW(monitor, &mut info).as_bool() {
        return Err("GetMonitorInfoW failed".to_string());
    }
    Ok((
        info.rcMonitor.left + (info.rcMonitor.right - info.rcMonitor.left) / 2,
        info.rcMonitor.top + (info.rcMonitor.bottom - info.rcMonitor.top) / 2,
    ))
}

unsafe fn monitor_metrics_for_context(context: &SubclassContext) -> Result<(RECT, u32), String> {
    let point = POINT {
        x: context.anchor_x.load(Ordering::SeqCst),
        y: context.anchor_y.load(Ordering::SeqCst),
    };
    let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
    let mut info = MONITORINFO::default();
    info.cbSize = size_of::<MONITORINFO>() as u32;
    if !GetMonitorInfoW(monitor, &mut info).as_bool() {
        return Err("GetMonitorInfoW failed".to_string());
    }

    let mut dpi_x = 96u32;
    let mut dpi_y = 96u32;
    if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_err() || dpi_x == 0 {
        dpi_x = 96;
    }

    Ok((info.rcMonitor, dpi_x))
}

fn logical_to_physical(logical: u32, dpi: u32) -> i32 {
    (((logical as u64) * (dpi as u64) + 48) / 96) as i32
}

fn physical_to_logical(physical: i32, dpi: u32) -> u32 {
    if dpi == 0 {
        return physical.max(1) as u32;
    }
    (((physical.max(1) as u64) * 96 + (dpi as u64 / 2)) / dpi as u64) as u32
}

fn clamp_physical_width(
    requested: i32,
    monitor_rect: RECT,
    dpi: u32,
    interactive: bool,
) -> i32 {
    let min_logical = if interactive {
        MIN_INTERACTIVE_LOGICAL_WIDTH
    } else {
        MIN_LOGICAL_WIDTH
    };
    let min_physical = logical_to_physical(min_logical, dpi).max(1);
    let configured_max = logical_to_physical(MAX_LOGICAL_WIDTH, dpi).max(min_physical);
    let monitor_width = (monitor_rect.right - monitor_rect.left).max(1);
    let monitor_max = ((monitor_width as i64 * 3) / 4) as i32;
    let max_physical = configured_max.min(monitor_max.max(min_physical));
    requested.clamp(min_physical, max_physical)
}

unsafe fn reserve_rect(
    hwnd: HWND,
    context: &SubclassContext,
    requested_physical_width: i32,
    interactive: bool,
) -> Result<(RECT, u32), String> {
    let (monitor_rect, dpi) = monitor_metrics_for_context(context)?;
    let physical_width = clamp_physical_width(requested_physical_width, monitor_rect, dpi, interactive);
    let edge = Edge::from_atomic(context.edge.load(Ordering::SeqCst));

    let mut data = appbar_data(hwnd);
    data.uEdge = edge.as_shell_edge();
    data.rc = monitor_rect;

    SHAppBarMessage(ABM_QUERYPOS, &mut data);

    match edge {
        Edge::Left => data.rc.right = data.rc.left + physical_width,
        Edge::Right => data.rc.left = data.rc.right - physical_width,
    }

    SHAppBarMessage(ABM_SETPOS, &mut data);
    Ok((data.rc, dpi))
}

unsafe fn reposition(hwnd: HWND, context: &SubclassContext) -> Result<(), String> {
    if !context.registered.load(Ordering::SeqCst) {
        return Ok(());
    }

    if context.in_layout.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let result = (|| {
        let (_, dpi) = monitor_metrics_for_context(context)?;
        let logical_width = context
            .width
            .load(Ordering::SeqCst)
            .clamp(MIN_LOGICAL_WIDTH, MAX_LOGICAL_WIDTH);
        let requested_physical = logical_to_physical(logical_width, dpi);
        let (rect, approved_dpi) = reserve_rect(hwnd, context, requested_physical, false)?;

        let width = (rect.right - rect.left).max(1);
        let height = (rect.bottom - rect.top).max(1);
        context.width.store(
            physical_to_logical(width, approved_dpi).clamp(MIN_LOGICAL_WIDTH, MAX_LOGICAL_WIDTH),
            Ordering::SeqCst,
        );

        let topmost = context.always_on_top.load(Ordering::SeqCst);
        let insert_after = if topmost { Some(HWND_TOPMOST) } else { Some(HWND_NOTOPMOST) };

        SetWindowPos(
            hwnd,
            insert_after,
            rect.left,
            rect.top,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .map_err(|error| format!("SetWindowPos failed: {error}"))?;

        Ok(())
    })();

    context.in_layout.store(false, Ordering::SeqCst);
    result
}

unsafe fn handle_native_sizing(
    hwnd: HWND,
    context: &SubclassContext,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Result<(), String> {
    if lparam.0 == 0 || !context.registered.load(Ordering::SeqCst) {
        return Ok(());
    }

    context.interactive_sizing.store(true, Ordering::SeqCst);

    if context.in_layout.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let result = (|| {
        let rect_ptr = lparam.0 as *mut RECT;
        let incoming = *rect_ptr;
        let edge = Edge::from_atomic(context.edge.load(Ordering::SeqCst));
        let sizing_edge = wparam.0 as u32;
        let expected_sizing_edge = match edge {
            Edge::Left => WMSZ_RIGHT,
            Edge::Right => WMSZ_LEFT,
        };

        let (_, dpi) = monitor_metrics_for_context(context)?;
        let current_logical = context
            .width
            .load(Ordering::SeqCst)
            .clamp(MIN_LOGICAL_WIDTH, MAX_LOGICAL_WIDTH);

        // Only the divider edge is allowed to resize the AppBar. Tauri's native
        // undecorated-resize helper may detect the other edges too; those are
        // intentionally locked so the bar always stays flush with its monitor.
        let requested_physical = if sizing_edge == expected_sizing_edge {
            (incoming.right - incoming.left).max(1)
        } else {
            logical_to_physical(current_logical, dpi)
        };

        let (approved, approved_dpi) = reserve_rect(hwnd, context, requested_physical, true)?;
        *rect_ptr = approved;

        let approved_width = (approved.right - approved.left).max(1);
        context.width.store(
            physical_to_logical(approved_width, approved_dpi)
                .clamp(MIN_INTERACTIVE_LOGICAL_WIDTH, MAX_LOGICAL_WIDTH),
            Ordering::SeqCst,
        );
        Ok(())
    })();

    context.in_layout.store(false, Ordering::SeqCst);
    result
}

unsafe fn apply_z_order(hwnd: HWND, topmost: bool) -> Result<(), String> {
    let insert_after = if topmost { Some(HWND_TOPMOST) } else { Some(HWND_NOTOPMOST) };
    SetWindowPos(
        hwnd,
        insert_after,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    )
    .map_err(|error| format!("SetWindowPos(z-order) failed: {error}"))
}

unsafe extern "system" fn appbar_subclass_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    ref_data: usize,
) -> LRESULT {
    if ref_data == 0 {
        return DefSubclassProc(hwnd, message, wparam, lparam);
    }

    let context = &*(ref_data as *const SubclassContext);

    if message == context.callback_message {
        match wparam.0 as u32 {
            ABN_POSCHANGED => {
                if !context.interactive_sizing.load(Ordering::SeqCst)
                    && !context.in_layout.load(Ordering::SeqCst)
                {
                    let _ = reposition(hwnd, context);
                }
            }
            ABN_FULLSCREENAPP => {
                // Keep the configured topmost policy. Exclusive fullscreen apps
                // can still take ownership of the display by design.
                let _ = apply_z_order(hwnd, context.always_on_top.load(Ordering::SeqCst));
            }
            _ => {}
        }
        return LRESULT(0);
    }

    match message {
        WM_ENTERSIZEMOVE => {
            context.interactive_sizing.store(true, Ordering::SeqCst);
        }
        WM_SIZING => {
            let _ = handle_native_sizing(hwnd, context, wparam, lparam);
            return LRESULT(1);
        }
        WM_EXITSIZEMOVE => {
            context.interactive_sizing.store(false, Ordering::SeqCst);
            let _ = reposition(hwnd, context);
        }
        WM_DISPLAYCHANGE | WM_DPICHANGED | WM_SETTINGCHANGE => {
            if !context.interactive_sizing.load(Ordering::SeqCst) {
                let _ = reposition(hwnd, context);
            }
        }
        WM_WINDOWPOSCHANGED => {
            if context.registered.load(Ordering::SeqCst) {
                let mut data = appbar_data(hwnd);
                SHAppBarMessage(ABM_WINDOWPOSCHANGED, &mut data);
            }
        }
        WM_DESTROY => {
            if context.registered.swap(false, Ordering::SeqCst) {
                let mut data = appbar_data(hwnd);
                SHAppBarMessage(ABM_REMOVE, &mut data);
            }
        }
        _ => {}
    }

    DefSubclassProc(hwnd, message, wparam, lparam)
}
