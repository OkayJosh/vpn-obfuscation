#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LAUNCHER="$ROOT_DIR/scripts/launch-gui.sh"
ICON_SRC="$ROOT_DIR/app/src-tauri/icons/icon.png"

ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
APP_DIR="$HOME/.local/share/applications"
ICON_DEST="$ICON_DIR/vpn-obfuscator.png"
DESKTOP_FILE="$APP_DIR/vpn-obfuscator.desktop"

if [[ ! -x "$LAUNCHER" ]]; then
  chmod +x "$LAUNCHER"
fi

if [[ ! -f "$ICON_SRC" ]]; then
  echo "[ERROR] Icon not found at $ICON_SRC" >&2
  exit 1
fi

mkdir -p "$ICON_DIR" "$APP_DIR"
cp "$ICON_SRC" "$ICON_DEST"

cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=VPN Obfuscator
GenericName=Tor + obfs4 & GNOME Proxy Controller
Comment=Route traffic through Tor + obfs4 bridges over WireGuard and manage GNOME System Proxy
Exec=$LAUNCHER
Icon=$ICON_DEST
Terminal=false
Categories=Network;Security;Utility;
Keywords=VPN;Tor;obfs4;Proxy;Privacy;WireGuard;GNOME;
StartupNotify=true
StartupWMClass=vpn-obfuscation-gui
EOF

chmod 644 "$DESKTOP_FILE"

if command -v desktop-file-validate >/dev/null 2>&1; then
  desktop-file-validate "$DESKTOP_FILE"
  echo "[OK] Validated $DESKTOP_FILE"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APP_DIR" 2>/dev/null || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

echo "[OK] Installed desktop shortcut: $DESKTOP_FILE"
echo "[OK] Installed app icon:         $ICON_DEST"
echo ""
echo "Open Fedora's 'Show Apps' (Super key) and search for 'VPN Obfuscator'!"
echo "You can right-click it there and choose 'Pin to Dash' for one-click access."
