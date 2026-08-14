# DeskNote Sidebar — Architecture

This document describes the component boundaries, the IPC bridge, and the
AppBar lifecycle. It targets contributors; see [README.md](../README.md) for
user-facing documentation.

## Component overview

```text
┌─────────────────────────── WebView ───────────────────────────┐
│  src/ (React + TypeScript)                                    │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────────────┐  │
│  │ App.tsx     │  │ storage.ts  │  │ native.ts             │  │
│  │ UI state,   │  │ notes &     │  │ typed wrappers over   │  │
│  │ rendering   │  │ settings    │  │ Tauri invoke()        │  │
│  └─────────────┘  └─────────────┘  └───────────┬───────────┘  │
│                              localStorage (persisted)         │
└───────────────────────────────────────────────┬───────────────┘
                                                │ IPC (invoke)
┌───────────────────────────────────────────────▼───────────────┐
│ src-tauri/ (Rust)                                              │
│  ┌─────────────────────┐    ┌───────────────────────────────┐  │
│  │ commands.rs         │    │ appbar/                       │  │
│  │ stateless IPC entry │───▶│  AppBarManager (managed state)│  │
│  │ points              │    │  ├─ windows_impl.rs (Win32)   │  │
│  └─────────────────────┘    │  └─ unsupported.rs (fallback) │  │
│        lib.rs: builder, setup, exit cleanup                  │  │
│        tauri.conf.json, capabilities/default.json            │  │
└───────────────────────────────────────────────────────────────┘
                        │ SHAppBarMessage / SetWindowSubclass / SetWindowPos
                        ▼
              Windows Shell (taskbar work area)
```

## Layers and rules

1. **UI (`src/`)** — React renders app state. It never calls Win32 directly.
2. **Storage (`src/storage.ts`)** — notes and settings persist in the WebView's
   `localStorage`. There is intentionally no remote sync.
3. **IPC bridge (`src/native.ts` + `commands.rs`)** — every command appears in
   both places, typed end-to-end. The command list in `lib.rs` is the
   allowlist the WebView can call; anything not listed is rejected.
4. **AppBar manager (`appbar/`)** — the only module allowed to touch the
   Windows Shell. `windows_impl.rs` is `#[cfg(windows)]`; `unsupported.rs`
   provides a no-op preview so the project compiles on non-Windows hosts.
5. **Capabilities (`capabilities/default.json`)** — the Tauri 2 permission
   gate for the `main` window; combined with the CSP in `tauri.conf.json`, no
   remote content or unlisted commands are reachable.

## The AppBar lifecycle

State lives in a single `AppBarManager` (registered with `app.manage()` in
`lib.rs`) so both IPC handlers and the native subclass share one source of
truth.

### Startup (`lib.rs::setup` → `register_with_window`)

1. Resolve the `main` `WebviewWindow` and its native `HWND`.
2. Register a private window message with `RegisterWindowMessageW`.
3. Install the subclass (`SetWindowSubclass`) — the callback is the single
   place where all Shell events converge.
4. Register with the Shell: `SHAppBarMessage(ABM_NEW, ...)`.
5. Locate the anchor monitor (stored by the user in Settings), read its
   effective DPI via `GetDpiForMonitor(MDT_EFFECTIVE_DPI)`, and convert the
   logical sidebar width to physical pixels.
6. Negotiate the rectangle: `ABM_QUERYPOS` (Windows accounts for the taskbar
   and other AppBars) → `ABM_SETPOS` (reserve it) → `SetWindowPos` (place the
   Tauri window inside it).
7. Remove the visual seam: `DwmSetWindowAttribute` for square corners /
   no DWM border, window shadow off.

### While running

- **Resize** — the divider calls Tauri's native resize-drag once; Windows runs
  the modal sizing loop and the subclass handles `WM_SIZING`, constraining the
  rectangle to the AppBar edge and re-reserving it. This replaces the v0.1
  approach of streaming `set_sidebar_width` IPC calls from JS.
- **`ABN_POSCHANGED`** — the taskbar moved or another AppBar registered;
  recompute the reserved rectangle.
- **`WM_DISPLAYCHANGE` / `WM_DPICHANGED` / `WM_SETTINGCHANGE`** — re-anchor to
  the nearest valid monitor and recalculate DPI-derived width.
- **IPC commands** (`set_sidebar_width`, `set_sidebar_edge`,
  `set_sidebar_monitor`, `set_sidebar_always_on_top`) — mutate manager state
  and immediately re-run the reservation flow; `refresh_sidebar_position`
  forces a full recalculation (used by "Refresh monitors" in Settings).

### Shutdown

`exit_app` calls `state.unregister()` (which sends `ABM_REMOVE`) and exits;
`lib.rs` also unregisters on any `RunEvent::ExitRequested`/`Exit`, so the work
area reservation is always released — even on taskbar kill / crash of the
WebView process.

## Threading model

- The Win32 subclass callback runs on the window's UI thread.
- Tauri commands run on Tauri's IPC/async context.
- `AppBarManager` uses interior mutability (`Mutex`-guarded fields) so both
  sides mutate state safely; the reservation functions are designed to be
  re-entrant (called from commands and from the subclass).

## Build matrix and CI

| Workflow | Trigger | What it does |
| --- | --- | --- |
| `windows.yml` | push to `main`, PRs | Full `tauri build` on `windows-latest` |
| `checks.yml` | push to `main`, PRs | `tsc`/Vite build (ubuntu) + `cargo fmt`/`clippy -D warnings` (windows) |
| `release.yml` | tag `v*` | Production build, SHA-256 checksums, draft GitHub Release with MSI/NSIS |
| Dependabot | weekly / monthly | `npm`, `cargo`, `github-actions` updates (grouped) |

Rust checks run on **Windows** deliberately — the AppBar implementation is
windows-only, so a Linux runner would lint only the no-op fallback.

## Known limits

- AppBars cannot coexist with exclusive fullscreen apps (see
  [README — fullscreen limitation](../README.md#fullscreen-limitation)).
- Notes are unencrypted local data; no sync exists by design.
- The bundle identifier (`com.example.desknote.sidebar`) is a placeholder —
  replace it before distributing, and use your own publisher metadata.
