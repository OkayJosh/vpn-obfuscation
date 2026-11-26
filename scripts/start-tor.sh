#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_DIR="$ROOT_DIR/.tor-data"
RUNTIME_DIR="$ROOT_DIR/.runtime"
TEMPLATE="$ROOT_DIR/torrc.template"
BRIDGES_FILE="$ROOT_DIR/bridges.txt"
TORRC_OUT="$RUNTIME_DIR/torrc"
PID_FILE="$RUNTIME_DIR/tor.pid"
LOG_FILE="$RUNTIME_DIR/tor.log"

USE_BRIDGES=1
DAEMON_MODE=0

for arg in "$@"; do
  case "$arg" in
    --no-bridges)
      USE_BRIDGES=0
      ;;
    --daemon|-d)
      DAEMON_MODE=1
      ;;
    --help|-h)
      echo "Usage: $0 [--daemon|-d] [--no-bridges]"
      echo ""
      echo "Options:"
      echo "  --daemon, -d   Run Tor in the background (logs to .runtime/tor.log)"
      echo "  --no-bridges   Connect to Tor directly without obfs4 bridges (testing only;"
      echo "                 your VPN will still not see your destinations, but CAN tell you use Tor)"
      exit 0
      ;;
    *)
      echo "[ERROR] Unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

if ! command -v tor >/dev/null 2>&1; then
  echo "[ERROR] 'tor' is not installed. Run ./scripts/install.sh first." >&2
  exit 1
fi

mkdir -p "$DATA_DIR" "$RUNTIME_DIR"
chmod 700 "$DATA_DIR"

BRIDGE_BLOCK=""
if [[ "$USE_BRIDGES" -eq 1 ]]; then
  OBFS4_BIN="$(command -v obfs4proxy || command -v lyrebird || true)"
  if [[ -z "$OBFS4_BIN" ]]; then
    echo "[ERROR] 'obfs4proxy' (or 'lyrebird') is not installed. Run ./scripts/install.sh first." >&2
    exit 1
  fi

  if [[ ! -f "$BRIDGES_FILE" ]]; then
    echo "[ERROR] $BRIDGES_FILE not found." >&2
    echo ""
    echo "Create it from the example and add 2-3 fresh obfs4 bridges:"
    echo "  cp bridges.txt.example bridges.txt"
    echo "  # Get bridges at: https://bridges.torproject.org/bridges?transport=obfs4"
    echo ""
    echo "Or test plain Tor right now with: $0 --no-bridges"
    exit 1
  fi

  BRIDGE_LINES=()
  while IFS= read -r raw_line || [[ -n "$raw_line" ]]; do
    line="$(echo "$raw_line" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
    [[ -z "$line" || "$line" == \#* ]] && continue
    if [[ "$line" == Bridge\ * ]]; then
      BRIDGE_LINES+=("$line")
    elif [[ "$line" == obfs4\ * ]]; then
      BRIDGE_LINES+=("Bridge $line")
    else
      echo "[WARN] Ignoring unrecognized line in bridges.txt: $line" >&2
    fi
  done < "$BRIDGES_FILE"

  if [[ "${#BRIDGE_LINES[@]}" -eq 0 ]]; then
    echo "[ERROR] No valid obfs4 bridge lines found in $BRIDGES_FILE." >&2
    echo "Get fresh bridges from: https://bridges.torproject.org/bridges?transport=obfs4" >&2
    exit 1
  fi

  BRIDGE_BLOCK="UseBridges 1"$'\n'"ClientTransportPlugin obfs4 exec $OBFS4_BIN"
  for b in "${BRIDGE_LINES[@]}"; do
    BRIDGE_BLOCK+=$'\n'"$b"
  done
  echo "[*] Configured ${#BRIDGE_LINES[@]} obfs4 bridge(s) using $OBFS4_BIN"
else
  BRIDGE_BLOCK="# Running without bridges (--no-bridges)"
  echo "[WARN] Running without obfs4 bridges: your VPN cannot see your destinations, but can see you are connecting to Tor."
fi

# Generate runtime torrc safely
python3 - "$TEMPLATE" "$TORRC_OUT" "$DATA_DIR" "$BRIDGE_BLOCK" <<'PYEOF'
import sys
template_path, out_path, data_dir, bridge_block = sys.argv[1:5]
with open(template_path, "r", encoding="utf-8") as f:
    content = f.read()
content = content.replace("__DATA_DIR__", data_dir)
content = content.replace("__BRIDGE_CONFIG__", bridge_block)
with open(out_path, "w", encoding="utf-8") as f:
    f.write(content)
PYEOF

if [[ "$DAEMON_MODE" -eq 1 ]]; then
  if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    echo "[*] Tor is already running (PID $(cat "$PID_FILE"))."
    exit 0
  fi
  echo "[*] Starting Tor in background..."
  tor -f "$TORRC_OUT" --RunAsDaemon 1 --PidFile "$PID_FILE" --Log "notice file $LOG_FILE"
  echo "[*] Tor started (PID $(cat "$PID_FILE"))."
  echo "    SOCKS5 proxy: 127.0.0.1:9050"
  echo "    HTTP proxy:   127.0.0.1:8118"
  echo "    Logs:         $LOG_FILE"
  echo "    Wait for 'Bootstrapped 100%' in logs, then run ./scripts/verify.sh"
else
  echo "[*] Starting Tor in foreground (Ctrl+C to stop)..."
  echo "    SOCKS5 proxy: 127.0.0.1:9050"
  echo "    HTTP proxy:   127.0.0.1:8118"
  exec tor -f "$TORRC_OUT"
fi
