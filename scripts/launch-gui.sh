#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE_BIN="$ROOT_DIR/app/src-tauri/target/release/vpn-obfuscation-gui"

export VPN_OBFUSCATION_ROOT="$ROOT_DIR"

if [[ -x "$RELEASE_BIN" ]]; then
  exec "$RELEASE_BIN" "$@"
else
  echo "[*] Release binary not built yet; starting in Tauri dev mode..."
  cd "$ROOT_DIR/app"
  exec npm run tauri dev
fi
