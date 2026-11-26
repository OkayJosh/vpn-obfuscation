#!/usr/bin/env bash
set -euo pipefail

case "${1:-status}" in
  on)
    INCLUDE_HTTP=0
    [[ "${2:-}" == "--http" ]] && INCLUDE_HTTP=1
    gsettings set org.gnome.system.proxy use-same-proxy false
    gsettings set org.gnome.system.proxy.socks host '127.0.0.1'
    gsettings set org.gnome.system.proxy.socks port 9050
    if [[ "$INCLUDE_HTTP" -eq 1 ]]; then
      gsettings set org.gnome.system.proxy.http host '127.0.0.1'
      gsettings set org.gnome.system.proxy.http port 8118
      gsettings set org.gnome.system.proxy.https host '127.0.0.1'
      gsettings set org.gnome.system.proxy.https port 8118
    else
      gsettings set org.gnome.system.proxy.http host ''
      gsettings set org.gnome.system.proxy.http port 0
      gsettings set org.gnome.system.proxy.https host ''
      gsettings set org.gnome.system.proxy.https port 0
    fi
    gsettings set org.gnome.system.proxy ignore-hosts "['localhost', '127.0.0.0/8', '::1']"
    gsettings set org.gnome.system.proxy mode 'manual'
    echo "[OK] GNOME proxy enabled -> SOCKS5 127.0.0.1:9050"
    ;;
  off)
    gsettings set org.gnome.system.proxy mode 'none'
    echo "[OK] GNOME proxy disabled (mode: none)"
    ;;
  status)
    echo "mode:  $(gsettings get org.gnome.system.proxy mode)"
    echo "socks: $(gsettings get org.gnome.system.proxy.socks host):$(gsettings get org.gnome.system.proxy.socks port)"
    echo "http:  $(gsettings get org.gnome.system.proxy.http host):$(gsettings get org.gnome.system.proxy.http port)"
    echo "https: $(gsettings get org.gnome.system.proxy.https host):$(gsettings get org.gnome.system.proxy.https port)"
    ;;
  *)
    echo "Usage: $0 {on [--http]|off|status}" >&2
    exit 1
    ;;
esac
