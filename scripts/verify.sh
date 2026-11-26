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
DIRECT_IP="$(curl -s --max-time 8 https://api.ipify.org || echo "unreachable")"
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
