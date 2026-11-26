#!/usr/bin/env bash
set -euo pipefail

if [[ $# -eq 0 || "$1" == "--help" || "$1" == "-h" ]]; then
  echo "Usage:"
  echo "  $0 <command> [args...]     Run a CLI command with Tor proxy env vars (socks5h + HTTP)"
  echo "  $0 --torsocks <cmd> [...]  Force arbitrary TCP binary through Tor via torsocks"
  echo "  $0 --chromium [url]        Launch Chromium/Chrome with isolated Tor SOCKS5 proxy + remote DNS"
  echo ""
  echo "Examples:"
  echo "  $0 curl https://check.torproject.org/api/ip"
  echo "  $0 --torsocks git clone https://example.com/repo.git"
  echo "  $0 --chromium https://check.torproject.org"
  exit 0
fi

case "$1" in
  --torsocks)
    shift
    if ! command -v torsocks >/dev/null 2>&1; then
      echo "[ERROR] torsocks is not installed. Run ./scripts/install.sh first." >&2
      exit 1
    fi
    exec torsocks "$@"
    ;;
  --chromium)
    shift
    BROWSER="$(command -v chromium || command -v chromium-browser || command -v google-chrome || true)"
    if [[ -z "$BROWSER" ]]; then
      echo "[ERROR] Neither chromium nor google-chrome found on PATH." >&2
      exit 1
    fi
    TMP_PROFILE="$(mktemp -d /tmp/tor-chromium-XXXXXX)"
    echo "[*] Launching $BROWSER with Tor SOCKS5 proxy (remote DNS, isolated profile)..."
    echo "    Profile will be cleaned up on exit: $TMP_PROFILE"
    trap 'rm -rf "$TMP_PROFILE"' EXIT
    # --host-resolver-rules prevents local DNS leaks to the upstream VPN
    "$BROWSER" \
      --user-data-dir="$TMP_PROFILE" \
      --proxy-server="socks5://127.0.0.1:9050" \
      --host-resolver-rules="MAP * ~NOTFOUND , EXCLUDE 127.0.0.1" \
      --incognito \
      "$@"
    ;;
  *)
    # socks5h:// ensures DNS is resolved through Tor, NOT locally via the upstream VPN
    export ALL_PROXY="socks5h://127.0.0.1:9050"
    export all_proxy="socks5h://127.0.0.1:9050"
    export HTTP_PROXY="http://127.0.0.1:8118"
    export http_proxy="http://127.0.0.1:8118"
    export HTTPS_PROXY="http://127.0.0.1:8118"
    export https_proxy="http://127.0.0.1:8118"
    exec "$@"
    ;;
esac
