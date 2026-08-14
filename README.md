# DeskNote Sidebar (Tauri 2 + Windows AppBar)

A production-oriented Windows desktop sidebar built with **Tauri 2**, **React/TypeScript**, **Rust**, and the native Windows **Application Desktop Toolbar (AppBar)** API.

Unlike a normal `alwaysOnTop` window, DeskNote registers with the Windows Shell and reserves part of the monitor work area. Maximized applications therefore use the remaining desktop area instead of opening behind the sidebar.

## Features

- Real Windows Shell AppBar registration (`ABM_NEW` / `ABM_REMOVE`)
- Position negotiation (`ABM_QUERYPOS` / `ABM_SETPOS`)
- Responds to `ABN_POSCHANGED`
- Responds to display, DPI, settings, and native sizing-loop changes
- Left or right docking
- Multi-monitor selection
- DPI-aware logical sidebar width
- Optional topmost z-order
- Native Windows drag-to-resize divider (`startResizeDragging` + `WM_SIZING`)
- Collapsed 56 px rail
- Persistent notes using WebView local storage
- Persistent UI settings
- Explicit reservation recalculation control
- Cleans up the AppBar reservation on exit
- Windows CI workflow included
- Non-Windows preview mode compiles without the native AppBar behavior


## v0.2 native-resize fixes

This revision specifically addresses the two issues reported after the first build:

- The old React pointer loop no longer sends a stream of `set_sidebar_width` IPC calls while dragging. The divider now starts Windows/Tauri's native resize operation once, and the Rust Win32 subclass handles `WM_SIZING`.
- Window shadow, outer CSS borders, Windows 11 corner rounding, and the DWM border are disabled/removed where supported to eliminate the visible seam around the dock.
- The AppBar width is recalculated from the selected monitor's effective DPI rather than assuming the current window DPI before a monitor move.
- `RemoveWindowSubclass` return values are explicitly ignored with `let _ = ...`, removing the two compiler warnings from v0.1.

## Project structure

```text
desknote-sidebar/
├─ src/                         React/TypeScript UI
│  ├─ App.tsx
│  ├─ main.tsx
│  ├─ native.ts                Tauri command wrappers
│  ├─ storage.ts               Persistent notes/settings
│  └─ styles.css
├─ src-tauri/
│  ├─ src/
│  │  ├─ appbar/
│  │  │  ├─ windows_impl.rs    Native Windows AppBar implementation
│  │  │  ├─ unsupported.rs     Non-Windows preview implementation
│  │  │  └─ mod.rs
│  │  ├─ commands.rs           Tauri IPC commands
│  │  ├─ lib.rs                App setup + exit cleanup
│  │  └─ main.rs
│  ├─ capabilities/default.json
│  ├─ icons/
│  ├─ Cargo.toml
│  └─ tauri.conf.json
└─ .github/workflows/windows.yml
```

## Requirements on Windows

Install:

1. Node.js 20 or newer
2. Rust stable with the MSVC toolchain
3. Microsoft C++ Build Tools / Visual Studio Build Tools with **Desktop development with C++**
4. WebView2 Runtime (normally already present on current Windows 10/11)

For Rust, the usual setup is:

```powershell
rustup default stable-msvc
```

## Run in development

From the project root:

```powershell
npm install
npm run tauri dev
```

The app starts docked to the right side at 360 logical pixels. Open or maximize another desktop application to verify that Windows reserves the sidebar area.

## Build the Windows installer

```powershell
npm install
npm run tauri build
```

Tauri writes release artifacts under:

```text
src-tauri/target/release/bundle/
```

## How the native AppBar works

`src-tauri/src/appbar/windows_impl.rs` does the following:

1. Gets the Tauri window's native `HWND`.
2. Registers a private callback message with `RegisterWindowMessageW`.
3. Registers the window as an AppBar using `SHAppBarMessage(ABM_NEW, ...)`.
4. Installs a safe Win32 subclass callback using `SetWindowSubclass`.
5. Locates the selected monitor.
6. Reads effective DPI for the selected monitor and converts logical width to physical pixels.
7. Calls `ABM_QUERYPOS` so Windows can account for the taskbar and other AppBars.
8. Calls `ABM_SETPOS` to reserve the resulting rectangle.
9. Calls `SetWindowPos` to place the Tauri window in that exact rectangle.
10. Uses Tauri’s native resize-drag command only to enter the Windows sizing loop; `WM_SIZING` then constrains and reserves the AppBar rectangle while the pointer moves.
11. Responds to Shell `ABN_POSCHANGED` notifications by recalculating the position.
12. Recalculates on `WM_DISPLAYCHANGE`, `WM_DPICHANGED`, and `WM_SETTINGCHANGE`.
13. Removes Windows 11 rounded corners/DWM border where supported and disables the Tauri window shadow so the dock is flush.
14. Sends `ABM_REMOVE` during normal exit or window destruction.

## Important fullscreen limitation

A Windows AppBar controls the normal desktop **work area**, so normal maximized windows respect it. Browser-style fullscreen often still allows a topmost AppBar to remain visible, but an exclusive fullscreen application or game can take ownership of the entire display. No normal desktop AppBar can guarantee that it remains beside every exclusive fullscreen application.

## Multi-monitor behavior

The UI lists monitors exposed by Tauri and stores the chosen monitor. The native AppBar stores an anchor point on that monitor and uses `MonitorFromPoint(..., MONITOR_DEFAULTTONEAREST)` when it recalculates its position.

If the physical monitor topology changes significantly (for example, monitors are unplugged and Windows reorders coordinates), use **Settings → Refresh monitors** and reselect the desired monitor. The native `WM_DISPLAYCHANGE` handler also recalculates against the nearest valid monitor.

## Security notes

- The window only exposes the custom Tauri commands defined in `commands.rs`.
- The Tauri capability uses `core:default` for the `main` window.
- A restrictive CSP is configured in `tauri.conf.json`.
- No remote web content is loaded.
- Notes and settings stay in the local WebView profile unless you add synchronization yourself.

## Production hardening you may still want

The project contains the native desktop behavior and app lifecycle needed for a solid production base. Before distributing a commercial application, consider adding:

- Code signing for Windows installers
- Crash reporting/telemetry according to your privacy requirements
- Encrypted/synchronized note storage if notes are sensitive
- Automatic updates using Tauri's updater plugin
- Accessibility/keyboard-navigation audit
- Automated Windows VM tests across common DPI and multi-monitor layouts
- Your own application identifier, publisher metadata, icons, and legal notices

## Native API references used

- Windows `SHAppBarMessage`
- `APPBARDATA`
- `ABM_NEW`, `ABM_REMOVE`, `ABM_QUERYPOS`, `ABM_SETPOS`, `ABM_WINDOWPOSCHANGED`
- `ABN_POSCHANGED`, `ABN_FULLSCREENAPP`
- `SetWindowSubclass` / `DefSubclassProc`
- `MonitorFromWindow`, `MonitorFromPoint`, `GetMonitorInfoW`
- `GetDpiForMonitor` / `MDT_EFFECTIVE_DPI`
- `WM_SIZING`, `WM_ENTERSIZEMOVE`, `WM_EXITSIZEMOVE`
- `DwmSetWindowAttribute` for square corners / no DWM border
- Tauri 2 `WebviewWindow::hwnd()` and monitor APIs

## License

MIT. Replace the copyright/publisher information before shipping.
