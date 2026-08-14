# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial public release infrastructure: community health files, issue/PR
  templates, Dependabot, lint checks, and a tag-triggered release pipeline.

### Security

- Fixed a use-after-free race in the Win32 AppBar teardown path: the
  subclass context is now freed from `WM_NCDESTROY` on the window-owner
  thread instead of from a Tauri IPC/run-event thread.
- `RemoveWindowSubclass` is no longer called from foreign threads (Win32
  thread-affinity contract).
- Narrowed the window capabilities to least privilege: replaced the
  `core:default` umbrella with only the three permissions the app uses
  (resize dragging, event listen/unlisten).
- Replaced the placeholder `com.example.desknote.sidebar` bundle identifier
  with `com.rafa57600.desknote`.

## [0.2.0] - 2026-08-14

### Changed

- Native resize is now driven by Windows' own sizing loop: the divider starts
  a single Tauri/Win32 resize operation and the Rust subclass handles
  `WM_SIZING`, replacing the v0.1 JavaScript pointer loop that streamed
  `set_sidebar_width` IPC calls while dragging.
- AppBar width is recalculated from the selected monitor's effective DPI
  instead of assuming the current window DPI before a monitor move.
- Removed the visible seam around the dock: window shadow, outer CSS borders,
  Windows 11 corner rounding, and the DWM border are disabled where supported.

### Fixed

- Two `RemoveWindowSubclass` compiler warnings from v0.1 removed by explicitly
  ignoring the return value.

## [0.1.0] - Initial release

- Windows Shell AppBar registration (`ABM_NEW` / `ABM_REMOVE`), work-area
  reservation, and `ABN_POSCHANGED` handling.
- Left/right docking, multi-monitor selection, DPI-aware width.
- Collapsible 56 px rail; persistent notes and settings in WebView local
  storage.
- Non-Windows preview mode that compiles without native AppBar behavior.

[Unreleased]: https://github.com/rafa57600/desknote-sidebar/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/rafa57600/desknote-sidebar/releases/tag/v0.2.0
