#!/usr/bin/env bash
set -euo pipefail

SOCKS_PROXY="127.0.0.1:9050"

echo "=== 1. WireGuard interface check ==="
WG_IFACES="$(ip -brief link show type wireguard 2>/dev/null || true)"
if [[ -n "$WG_IFACES" ]]; then
  echo "[OK] Active WireGuard interface(s):"
  echo "$WG_IFACES"
else
  echo "[WARN] No active WireGuard interface detected."
fi

echo ""
echo "=== 2. Direct / VPN exit IP (what sites see WITHOUT Tor) ==="
DIRECT_IP="$(curl -s --noproxy '*' --max-time 8 https://api.ipify.org || echo "unreachable")"
echo "Direct/VPN IP: $DIRECT_IP"

echo ""
echo "=== 3. Tor SOCKS5 exit check (with remote DNS via socks5h) ==="
TOR_JSON="$(curl -s --max-time 15 --socks5-hostname "$SOCKS_PROXY" https://check.torproject.org/api/ip || true)"

if [[ -z "$TOR_JSON" ]]; then
  echo "[FAIL] Could not connect through Tor at $SOCKS_PROXY."
  echo "       Make sure ./scripts/start-tor.sh is running and has bootstrapped to 100%."
  exit 1
fi

echo "Tor API response: $TOR_JSON"

if echo "$TOR_JSON" | grep -q '"IsTor":true'; then
  echo "[OK] Traffic is exiting through Tor (hidden from your upstream VPN)."
else
  echo "[FAIL] Traffic is NOT exiting through Tor!"
  exit 1
fi

echo ""
echo "=== 4. GNOME System Proxy status & live GIO check ==="
if command -v gsettings >/dev/null 2>&1; then
  G_MODE="$(gsettings get org.gnome.system.proxy mode | tr -d "'")"
  G_SOCKS_HOST="$(gsettings get org.gnome.system.proxy.socks host | tr -d "'")"
  G_SOCKS_PORT="$(gsettings get org.gnome.system.proxy.socks port)"
  G_HTTP_HOST="$(gsettings get org.gnome.system.proxy.http host | tr -d "'")"
  G_HTTP_PORT="$(gsettings get org.gnome.system.proxy.http port)"
  echo "GNOME mode:  $G_MODE"
  echo "GNOME socks: ${G_SOCKS_HOST}:${G_SOCKS_PORT}"
  echo "GNOME http:  ${G_HTTP_HOST}:${G_HTTP_PORT}"

  if [[ "$G_MODE" == "manual" && "$G_SOCKS_HOST" == "127.0.0.1" && "$G_SOCKS_PORT" == "9050" ]]; then
    echo "[OK] GNOME System Proxy is pointed at Tor SOCKS5 (127.0.0.1:9050)."
    if command -v gio >/dev/null 2>&1; then
      GIO_JSON="$(timeout 15 gio cat https://check.torproject.org/api/ip 2>/dev/null || true)"
      echo "GNOME native (gio) response: ${GIO_JSON:-failed}"
      if echo "$GIO_JSON" | grep -q '"IsTor":true'; then
        echo "[OK] Verified end-to-end: GNOME apps & Chrome are exiting through Tor."
      else
        echo "[FAIL] GNOME proxy is ON, but GNOME native request did NOT exit through Tor!"
        exit 1
      fi
    fi
  else
    echo "[INFO] GNOME System Proxy is OFF (or not set to 127.0.0.1:9050)."
    echo "       Run ./scripts/gnome-proxy.sh on (or toggle it in the GUI) to route Chrome through Tor."
  fi
else
  echo "[INFO] gsettings not found (non-GNOME environment)."
fi
