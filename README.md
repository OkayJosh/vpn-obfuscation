# VPN Obfuscator — Tor + obfs4 over WireGuard & GNOME Proxy

A lightweight Linux toolkit and native **Tauri v2** desktop controller that routes your traffic through **Tor + `obfs4` bridges** *inside* an existing WireGuard VPN — hiding your destinations, DNS queries, and Tor usage from third-party, shared, or untrusted VPN servers without needing a second VPS.

## How It Works

```text
Your Apps / Browser (Chrome, Firefox, CLI)
   │  (SOCKS5h 127.0.0.1:9050 or HTTP 127.0.0.1:8118)
   ▼
Local Tor Client + obfs4proxy
   │  (3-layer Tor encryption wrapped in obfs4 obfuscation)
   ▼
Upstream WireGuard VPN (e.g. wg0)
   │  (VPN only sees obfuscated TCP noise to an unlisted bridge IP)
   ▼
obfs4 Bridge ──► Tor Network ──► Destination
```

### What your upstream VPN can and cannot see
- **Cannot see**: which websites or IPs you visit, your DNS queries (when using remote DNS / `socks5h://`), TLS SNI hostnames, or traffic contents.
- **Cannot easily fingerprint**: Tor handshakes (scrambled by `obfs4` pluggable transport to an unlisted bridge).
- **Can still see**: that your device is sending scrambled TCP packets to a single bridge IP, along with packet timing and volume.

---

## Features
- **Tauri v2 Desktop App** (`app/`):
  - One-click **Start / Stop Tor** with live bootstrap progress (`0% → 100%`)
  - **GNOME System Proxy** toggle (`gsettings`) to route Chrome and GNOME apps via `SOCKS5 127.0.0.1:9050` (and optional `HTTP/HTTPS 127.0.0.1:8118`), with automatic proxy disable when stopping Tor so you never get stranded offline
  - Built-in **IP & Leak Verification** comparing your direct/VPN IP vs. Tor exit IP
  - Live **WireGuard interface** status monitor
  - Built-in **`bridges.txt` editor** with **Save & Restart Tor**
  - **Live Tor Logs** viewer and one-click **Isolated Chromium** launcher
- **CLI Scripts** (`scripts/`):
  - `install.sh` — Installs `tor`, `obfs4`, `torsocks`, and `proxychains-ng` on Fedora
  - `start-tor.sh` / `stop-tor.sh` — Runs Tor as an unprivileged local user (foreground or `--daemon`, with `obfs4` bridges or `--no-bridges`)
  - `gnome-proxy.sh` — Toggle GNOME system proxy from the terminal (`on`, `on --http`, `off`, `status`)
  - `verify.sh` — Checks WireGuard status and verifies `IsTor: true`
  - `run-via-tor.sh` — Wrapper for CLI tools (`socks5h://`), `torsocks`, or isolated Chromium (`--chromium`)

---

## Quickstart

### 1. Install dependencies
On **Fedora**:
```bash
./scripts/install.sh
```
On **Debian / Ubuntu**:
```bash
sudo apt install -y tor obfs4proxy torsocks proxychains4
```

### 2. Add fresh `obfs4` bridges
```bash
cp bridges.txt.example bridges.txt
```
Get 2–3 fresh `obfs4` bridge lines from any of:
- Web: <https://bridges.torproject.org/bridges?transport=obfs4>
- Email: send `get transport obfs4` to `bridges@torproject.org` (from Gmail or Riseup)
- Telegram: message `@GetBridgesBot` with `/bridges`

Paste the lines into `bridges.txt` (it is git-ignored so your private bridges are never committed).

### 3. Build & launch the Desktop GUI (Tauri v2)
Requires Rust (`cargo`), Node.js (`npm`), and GTK3 / WebKit2GTK (`webkit2gtk4.1-devel` on Fedora or `libwebkit2gtk-4.1-dev` on Debian/Ubuntu):
```bash
cd app
npm install
npm run tauri build -- --no-bundle
cd ..
./scripts/launch-gui.sh
```
Or run in development mode:
```bash
cd app && npm run tauri dev
```

### 4. Or use the CLI directly
```bash
# Start Tor with obfs4 bridges in background
./scripts/start-tor.sh --daemon

# Verify WireGuard + Tor exit IP
./scripts/verify.sh

# Route Chrome & GNOME apps through Tor
./scripts/gnome-proxy.sh on

# Or run individual commands / isolated Chromium
./scripts/run-via-tor.sh curl https://check.torproject.org/api/ip
./scripts/run-via-tor.sh --chromium https://check.torproject.org

# Stop Tor & restore direct/VPN routing
./scripts/gnome-proxy.sh off
./scripts/stop-tor.sh
```

---

## Security Notes
1. **Always use `socks5h://` (not `socks5://`)** in CLI tools so DNS queries resolve through Tor instead of leaking to your upstream VPN's DNS server.
2. **Only apps using `127.0.0.1:9050` or `127.0.0.1:8118`** (or GNOME System Proxy) are routed through Tor — background system services outside the proxy still exit directly through your VPN.
3. **Tor only tunnels TCP** (not UDP/ICMP).
4. **Never commit `bridges.txt`, `.tor-data/`, or `.runtime/`** — all three are listed in `.gitignore`.

## License
[MIT](LICENSE)
