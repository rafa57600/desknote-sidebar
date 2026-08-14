# Contributing to DeskNote Sidebar

Thanks for your interest in contributing! DeskNote Sidebar is a native Windows
desktop app built with [Tauri 2](https://tauri.app), Rust, and React/TypeScript.
This guide explains how to set up a dev environment, how the code is organized,
and what we expect from pull requests.

## Getting started

### Prerequisites (Windows)

- [Node.js](https://nodejs.org) 20 or newer
- [Rust](https://rustup.rs) stable with the MSVC toolchain:
  ```powershell
  rustup default stable-msvc
  ```
- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
  with **Desktop development with C++** (required for `windows-sys` and friends)
- [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
  (normally preinstalled on current Windows)

### Development setup

```powershell
git clone https://github.com/rafa57600/desknote-sidebar.git
cd desknote-sidebar
npm install
npm run tauri dev
```

The app starts docked to the right side at 360 logical pixels.

### Useful commands

| Command                     | Purpose                                   |
| --------------------------- | ----------------------------------------- |
| `npm run dev`               | Vite dev server (frontend only)           |
| `npm run build`             | TypeScript check + frontend production build |
| `npm run tauri dev`         | Full app in dev mode (hot reload)         |
| `npm run tauri build`       | Production bundle (MSI/NSIS installer)    |
| `cargo fmt --check`         | Rust formatting check (in `src-tauri/`)   |
| `cargo clippy -- -D warnings` | Rust lint check (in `src-tauri/`)       |

## Project layout

```text
desknote-sidebar/
├─ src/                     React/TypeScript UI
│  ├─ App.tsx               Main UI component
│  ├─ native.ts             Tauri command wrappers (typed IPC bridge)
│  ├─ storage.ts            Persistent notes/settings (WebView local storage)
│  └─ styles.css            Global styles
├─ src-tauri/
│  ├─ src/
│  │  ├─ appbar/
│  │  │  ├─ windows_impl.rs    Native Windows AppBar implementation
│  │  │  ├─ unsupported.rs     Non-Windows fallback (preview builds)
│  │  │  └─ mod.rs
│  │  ├─ commands.rs           Tauri IPC command definitions
│  │  ├─ lib.rs                App setup + exit cleanup
│  │  └─ main.rs
│  ├─ capabilities/default.json
│  ├─ icons/
│  ├─ Cargo.toml
│  └─ tauri.conf.json
└─ .github/workflows/       CI + release pipelines
```

The important boundary: **UI state lives in `src/`**, and everything that talks
to the Windows Shell lives in `src-tauri/src/appbar/windows_impl.rs`. The
frontend never touches the AppBar API directly — it goes through the typed
commands in `src/native.ts` and `commands.rs`.

## Making changes

1. Create a branch: `git checkout -b fix/describe-the-fix`
2. Make your change. Keep it focused — one concern per PR.
3. For UI changes, verify the behavior by running `npm run tauri dev`.
4. Run the checks below before opening a PR.

## Pull request checklist

- [ ] `npm run build` passes (TypeScript)
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] The app was run manually and the changed behavior verified
- [ ] README or docs updated if user-visible behavior changed
- [ ] No new secrets, absolute paths, or machine-specific values added

## Coding style

- **Rust:** follow `rustfmt` and `clippy`; prefer safe, explicit code over
  cleverness. Win32 FFI stays contained in the `appbar` module.
- **TypeScript:** strict mode is on; keep the IPC bridge in `src/native.ts`
  typed end-to-end (a command appears in `commands.rs` and as a typed wrapper).
- **CSS:** use the existing custom-property tokens in `styles.css`; don't
  hardcode colors.

## Reporting bugs / requesting features

Open an issue — there are templates for [bug reports](.github/ISSUE_TEMPLATE/bug_report.yml)
and [feature requests](.github/ISSUE_TEMPLATE/feature_request.yml). For
security vulnerabilities, use the private [security advisory](https://github.com/rafa57600/desknote-sidebar/security/advisories/new)
flow instead of a public issue (see [SECURITY.md](SECURITY.md)).

## License

By contributing you agree that your contributions are licensed under the
[MIT License](LICENSE) that covers this project.
