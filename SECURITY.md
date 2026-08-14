# Security Policy

## Supported versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | ✅ Yes             |
| < 0.2   | ❌ No              |

Only the latest release receives security fixes.

## Reporting a vulnerability

Please **do not open a public issue** for security vulnerabilities. Report them
privately instead:

- **Preferred:** [GitHub Security Advisories](https://github.com/rafa57600/desknote-sidebar/security/advisories/new)
  — the "Report a vulnerability" button.

You will receive a response within **7 days**. If the issue is confirmed, a fix
is released as soon as a patched version is available, and the vulnerability is
disclosed after the fix ships.

### What to include

- Affected version(s) and platform
- A description of the vulnerability and its impact
- Steps to reproduce (minimal)
- Any proof-of-concept code (if applicable)

## Security notes for this project

- The app loads **no remote content**; the WebView only serves local files and
  the restricted CSP in `src-tauri/tauri.conf.json`.
- The window exposes only the custom commands listed in `commands.rs` and gated
  by `capabilities/default.json`.
- Notes and settings live in the local WebView profile and are **not
  encrypted** — treat the app as a plain-text notepad until end-to-end storage
  encryption is added.
- If you self-build and distribute, keep in mind that binaries can be modified;
  use the official installers and verify checksums where available.
