#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PID_FILE="$ROOT_DIR/.runtime/tor.pid"

if [[ ! -f "$PID_FILE" ]]; then
  echo "[*] No PID file found ($PID_FILE). Tor daemon does not appear to be running."
  exit 0
fi

PID="$(cat "$PID_FILE")"
if kill -0 "$PID" 2>/dev/null; then
  echo "[*] Stopping Tor (PID $PID)..."
  kill "$PID"
  rm -f "$PID_FILE"
  echo "[*] Stopped."
else
  echo "[*] Process $PID is not running. Cleaning up stale PID file."
  rm -f "$PID_FILE"
fi
