$ErrorActionPreference = "Stop"
if (-not (Test-Path "node_modules")) {
  npm install
}
npm run tauri dev
