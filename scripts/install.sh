#!/usr/bin/env bash
set -euo pipefail

echo "[*] Installing Tor, obfs4proxy, torsocks, and proxychains-ng on Fedora..."
sudo dnf install -y tor obfs4 torsocks proxychains-ng

echo ""
echo "[*] Verifying installed binaries:"
for bin in tor obfs4proxy torsocks proxychains4; do
  if command -v "$bin" >/dev/null 2>&1; then
    echo "  [OK] $bin -> $(command -v "$bin")"
  else
    echo "  [WARN] $bin not found on PATH"
  fi
done

echo ""
echo "[*] Done. Next steps:"
echo "  1. Copy bridges.txt.example to bridges.txt and paste 2-3 obfs4 bridges"
echo "     (get them from https://bridges.torproject.org/bridges?transport=obfs4)"
echo "  2. Run: ./scripts/start-tor.sh"
echo "  3. Verify: ./scripts/verify.sh"
