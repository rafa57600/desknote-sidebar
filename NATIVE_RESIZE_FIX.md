# Native resize / flush-edge revision (v0.2.0)

The v0.1 resize implementation used JavaScript pointer movement plus repeated Tauri IPC calls. v0.2 replaces that path with the native Tauri/Windows sizing loop.

## What changed

- `src/App.tsx`: the divider calls `getCurrentWindow().startResizeDragging("West" | "East")` once.
- `windows_impl.rs`: `WM_SIZING` owns the live rectangle, clamps width, calls `ABM_QUERYPOS` / `ABM_SETPOS`, and updates the stored logical width.
- `windows_impl.rs`: `WM_ENTERSIZEMOVE` / `WM_EXITSIZEMOVE` suppress conflicting AppBar re-layout during the system drag.
- `tauri.conf.json`: native resizing enabled, Tauri shadow disabled.
- `windows_impl.rs`: Windows 11 square-corner preference and DWM border suppression are applied when supported.
- `styles.css`: outer CSS borders removed; the divider line is an inset visual only, so it consumes no desktop pixels.
- `windows_impl.rs`: selected-monitor DPI is used for width conversion.
- Both `RemoveWindowSubclass` warnings are fixed with explicit `let _ = ...`.

## Test on Windows

```powershell
npm install
npm run tauri dev
```

Check these cases:

1. Maximize Explorer/Chrome beside the sidebar: there should be no visible unused strip between the maximized app and DeskNote.
2. Drag only the inner divider edge: resizing should follow the Windows pointer smoothly and the other maximized app should track the reserved work area.
3. Release the mouse: the chosen width should persist.
4. Switch 100%/125%/150% DPI monitors and use **Refresh monitors** if needed.
5. Move the Windows taskbar or change display resolution: DeskNote should recalculate its AppBar rectangle.
