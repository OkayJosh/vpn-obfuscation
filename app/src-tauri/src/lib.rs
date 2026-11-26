use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct GnomeProxyStatus {
    pub mode: String,
    pub socks_host: String,
    pub socks_port: i32,
    pub http_host: String,
    pub http_port: i32,
    pub https_host: String,
    pub https_port: i32,
    pub enabled_for_tor: bool,
    pub includes_http: bool,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemStatus {
    pub workspace_root: String,
    pub vpn_active: bool,
    pub vpn_interfaces: Vec<String>,
    pub tor_running: bool,
    pub tor_pid: Option<u32>,
    pub tor_bootstrap: u8,
    pub tor_status_line: String,
    pub tor_using_bridges: bool,
    pub configured_bridges: usize,
    pub gnome_proxy: GnomeProxyStatus,
    pub tor_installed: bool,
    pub obfs4_installed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifyResult {
    pub direct_ip: String,
    pub tor_ip: Option<String>,
    pub is_tor: bool,
    pub error: Option<String>,
    pub gnome_proxy_enabled: bool,
    pub gnome_ip: Option<String>,
    pub gnome_is_tor: bool,
    pub gnome_error: Option<String>,
    pub http_tunnel_ok: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct TorCheckApi {
    #[serde(rename = "IsTor")]
    is_tor: bool,
    #[serde(rename = "IP")]
    ip: String,
}

fn find_workspace_root() -> PathBuf {
    if let Ok(env_root) = std::env::var("VPN_OBFUSCATION_ROOT") {
        let p = PathBuf::from(env_root);
        if p.join("torrc.template").exists() {
            return p;
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        let mut cur = Some(cwd.as_path());
        while let Some(dir) = cur {
            candidates.push(dir.to_path_buf());
            cur = dir.parent();
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent();
        while let Some(dir) = cur {
            candidates.push(dir.to_path_buf());
            cur = dir.parent();
        }
    }

    for cand in candidates {
        if cand.join("torrc.template").exists() && cand.join("scripts/start-tor.sh").exists() {
            return cand;
        }
    }

    // Fallback to compile-time parent of app/src-tauri
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(root) = manifest_dir.parent().and_then(|p| p.parent()) {
        if root.join("torrc.template").exists() {
            return root.to_path_buf();
        }
    }

    PathBuf::from(".")
}

fn is_pid_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

fn parse_wireguard_interfaces(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            trimmed.split_whitespace().next().map(|s| s.to_string())
        })
        .collect()
}

fn parse_tor_bootstrap(log_content: &str) -> (u8, String) {
    let mut progress: u8 = 0;
    let mut last_line = String::new();

    for line in log_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(idx) = trimmed.find("Bootstrapped ") {
            let after = &trimmed[idx + "Bootstrapped ".len()..];
            if let Some(pct_idx) = after.find('%') {
                if let Ok(val) = after[..pct_idx].trim().parse::<u8>() {
                    progress = val;
                    last_line = trimmed[idx..].to_string();
                }
            }
        }
    }

    if last_line.is_empty() {
        if let Some(last) = log_content.lines().rev().find(|l| !l.trim().is_empty()) {
            last_line = last.trim().to_string();
        }
    }

    (progress, last_line)
}

fn count_configured_bridges(content: &str) -> usize {
    content
        .lines()
        .filter(|line| {
            let t = line.trim();
            !t.is_empty() && !t.starts_with('#') && (t.starts_with("obfs4 ") || t.starts_with("Bridge obfs4 "))
        })
        .count()
}

fn strip_gsettings_quotes(raw: &str) -> String {
    raw.trim().trim_matches('\'').to_string()
}

async fn cmd_exists(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

async fn gsettings_get(schema: &str, key: &str) -> Result<String, String> {
    let out = Command::new("gsettings")
        .args(["get", schema, key])
        .output()
        .await
        .map_err(|e| format!("Failed to run gsettings: {e}"))?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(strip_gsettings_quotes(&String::from_utf8_lossy(&out.stdout)))
}

async fn gsettings_set(schema: &str, key: &str, value: &str) -> Result<(), String> {
    let out = Command::new("gsettings")
        .args(["set", schema, key, value])
        .output()
        .await
        .map_err(|e| format!("Failed to run gsettings: {e}"))?;

    if !out.status.success() {
        return Err(format!(
            "gsettings set {schema} {key} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

async fn read_gnome_proxy_status() -> GnomeProxyStatus {
    let available = cmd_exists("gsettings").await;
    if !available {
        return GnomeProxyStatus {
            mode: "unavailable".to_string(),
            socks_host: String::new(),
            socks_port: 0,
            http_host: String::new(),
            http_port: 0,
            https_host: String::new(),
            https_port: 0,
            enabled_for_tor: false,
            includes_http: false,
            available: false,
        };
    }

    let mode = gsettings_get("org.gnome.system.proxy", "mode")
        .await
        .unwrap_or_else(|_| "none".to_string());
    let socks_host = gsettings_get("org.gnome.system.proxy.socks", "host")
        .await
        .unwrap_or_default();
    let socks_port = gsettings_get("org.gnome.system.proxy.socks", "port")
        .await
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let http_host = gsettings_get("org.gnome.system.proxy.http", "host")
        .await
        .unwrap_or_default();
    let http_port = gsettings_get("org.gnome.system.proxy.http", "port")
        .await
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let https_host = gsettings_get("org.gnome.system.proxy.https", "host")
        .await
        .unwrap_or_default();
    let https_port = gsettings_get("org.gnome.system.proxy.https", "port")
        .await
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);

    let enabled_for_tor = mode == "manual" && socks_host == "127.0.0.1" && socks_port == 9050;
    let includes_http = enabled_for_tor
        && http_host == "127.0.0.1"
        && http_port == 8118
        && https_host == "127.0.0.1"
        && https_port == 8118;

    GnomeProxyStatus {
        mode,
        socks_host,
        socks_port,
        http_host,
        http_port,
        https_host,
        https_port,
        enabled_for_tor,
        includes_http,
        available: true,
    }
}

#[tauri::command]
async fn get_status() -> Result<SystemStatus, String> {
    let root = find_workspace_root();

    // 1. WireGuard interfaces
    let wg_out = Command::new("ip")
        .args(["-brief", "link", "show", "type", "wireguard"])
        .output()
        .await
        .ok();
    let vpn_interfaces = wg_out
        .map(|o| parse_wireguard_interfaces(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default();
    let vpn_active = !vpn_interfaces.is_empty();

    // 2. Tor PID + status
    let pid_path = root.join(".runtime/tor.pid");
    let mut tor_pid: Option<u32> = None;
    let mut tor_running = false;
    if let Ok(pid_str) = fs::read_to_string(&pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            if is_pid_alive(pid) {
                tor_pid = Some(pid);
                tor_running = true;
            }
        }
    }

    // 3. Tor log bootstrap progress
    let log_path = root.join(".runtime/tor.log");
    let (tor_bootstrap, tor_status_line) = if tor_running {
        match fs::read_to_string(&log_path) {
            Ok(content) => parse_tor_bootstrap(&content),
            Err(_) => (0, "Starting Tor...".to_string()),
        }
    } else {
        (0, "Stopped".to_string())
    };

    // 4. Bridges info
    let torrc_path = root.join(".runtime/torrc");
    let tor_using_bridges = tor_running
        && fs::read_to_string(&torrc_path)
            .map(|c| c.contains("UseBridges 1"))
            .unwrap_or(false);

    let bridges_path = root.join("bridges.txt");
    let configured_bridges = fs::read_to_string(&bridges_path)
        .map(|c| count_configured_bridges(&c))
        .unwrap_or(0);

    // 5. GNOME proxy & binaries
    let gnome_proxy = read_gnome_proxy_status().await;
    let tor_installed = cmd_exists("tor").await;
    let obfs4_installed = cmd_exists("obfs4proxy").await || cmd_exists("lyrebird").await;

    Ok(SystemStatus {
        workspace_root: root.display().to_string(),
        vpn_active,
        vpn_interfaces,
        tor_running,
        tor_pid,
        tor_bootstrap,
        tor_status_line,
        tor_using_bridges,
        configured_bridges,
        gnome_proxy,
        tor_installed,
        obfs4_installed,
    })
}

#[tauri::command]
async fn start_tor(use_bridges: bool) -> Result<String, String> {
    let root = find_workspace_root();
    let script = root.join("scripts/start-tor.sh");
    if !script.exists() {
        return Err(format!("Missing script: {}", script.display()));
    }

    // Clear old log before starting so bootstrap progress is accurate
    let log_path = root.join(".runtime/tor.log");
    let _ = fs::remove_file(&log_path);

    let mut args = vec!["--daemon"];
    if !use_bridges {
        args.push("--no-bridges");
    }

    let out = Command::new(&script)
        .args(&args)
        .current_dir(&root)
        .output()
        .await
        .map_err(|e| format!("Failed to execute start-tor.sh: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

    if !out.status.success() {
        return Err(if !stderr.is_empty() { stderr } else { stdout });
    }
    Ok(stdout)
}

#[tauri::command]
async fn stop_tor(disable_gnome_proxy: bool) -> Result<String, String> {
    let root = find_workspace_root();
    let script = root.join("scripts/stop-tor.sh");
    if !script.exists() {
        return Err(format!("Missing script: {}", script.display()));
    }

    if disable_gnome_proxy {
        let proxy = read_gnome_proxy_status().await;
        if proxy.enabled_for_tor {
            let _ = set_gnome_proxy(false, false).await;
        }
    }

    let out = Command::new(&script)
        .current_dir(&root)
        .output()
        .await
        .map_err(|e| format!("Failed to execute stop-tor.sh: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

    if !out.status.success() {
        return Err(if !stderr.is_empty() { stderr } else { stdout });
    }
    Ok(stdout)
}

#[tauri::command]
async fn set_gnome_proxy(enabled: bool, include_http: bool) -> Result<GnomeProxyStatus, String> {
    if !cmd_exists("gsettings").await {
        return Err("gsettings is not installed (GNOME desktop required).".to_string());
    }

    if enabled {
        gsettings_set("org.gnome.system.proxy", "use-same-proxy", "false").await?;
        gsettings_set("org.gnome.system.proxy.socks", "host", "'127.0.0.1'").await?;
        gsettings_set("org.gnome.system.proxy.socks", "port", "9050").await?;

        if include_http {
            gsettings_set("org.gnome.system.proxy.http", "host", "'127.0.0.1'").await?;
            gsettings_set("org.gnome.system.proxy.http", "port", "8118").await?;
            gsettings_set("org.gnome.system.proxy.https", "host", "'127.0.0.1'").await?;
            gsettings_set("org.gnome.system.proxy.https", "port", "8118").await?;
        } else {
            gsettings_set("org.gnome.system.proxy.http", "host", "''").await?;
            gsettings_set("org.gnome.system.proxy.http", "port", "0").await?;
            gsettings_set("org.gnome.system.proxy.https", "host", "''").await?;
            gsettings_set("org.gnome.system.proxy.https", "port", "0").await?;
        }

        gsettings_set(
            "org.gnome.system.proxy",
            "ignore-hosts",
            "['localhost', '127.0.0.0/8', '::1']",
        )
        .await?;
        gsettings_set("org.gnome.system.proxy", "mode", "'manual'").await?;
    } else {
        gsettings_set("org.gnome.system.proxy", "mode", "'none'").await?;
    }

    Ok(read_gnome_proxy_status().await)
}

fn parse_tor_check_json(body: &str) -> Result<(bool, String), String> {
    serde_json::from_str::<TorCheckApi>(body.trim())
        .map(|p| (p.is_tor, p.ip))
        .map_err(|e| format!("Unexpected Tor check response: {e}"))
}

#[tauri::command]
async fn verify_connection() -> Result<VerifyResult, String> {
    let gnome_proxy = read_gnome_proxy_status().await;

    let direct_fut = Command::new("curl")
        .args(["-s", "--noproxy", "*", "--max-time", "8", "https://api.ipify.org"])
        .output();

    let tor_fut = Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "15",
            "--socks5-hostname",
            "127.0.0.1:9050",
            "https://check.torproject.org/api/ip",
        ])
        .output();

    // Native GNOME/GLib check: `gio cat` reads org.gnome.system.proxy directly without CLI proxy flags
    let gnome_fut = async {
        if !cmd_exists("gio").await {
            return Err("gio not installed".to_string());
        }
        let out = Command::new("timeout")
            .args(["15", "gio", "cat", "https://check.torproject.org/api/ip"])
            .output()
            .await
            .map_err(|e| format!("Failed to run gio: {e}"))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if err.is_empty() {
                "GNOME gio request timed out or failed".to_string()
            } else {
                err
            });
        }
        let body = String::from_utf8_lossy(&out.stdout).to_string();
        parse_tor_check_json(&body)
    };

    // Optional HTTP CONNECT tunnel check when GNOME HTTP/HTTPS proxy (:8118) is enabled
    let http_fut = async {
        if !gnome_proxy.includes_http {
            return None;
        }
        let out = Command::new("curl")
            .args([
                "-s",
                "--max-time",
                "15",
                "--proxy",
                "http://127.0.0.1:8118",
                "https://check.torproject.org/api/ip",
            ])
            .output()
            .await
            .ok()?;
        if !out.status.success() {
            return Some(false);
        }
        let body = String::from_utf8_lossy(&out.stdout);
        Some(parse_tor_check_json(&body).map(|(is_tor, _)| is_tor).unwrap_or(false))
    };

    let (direct_res, tor_res, gnome_res, http_tunnel_ok) =
        tokio::join!(direct_fut, tor_fut, gnome_fut, http_fut);

    let direct_ip = direct_res
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unreachable".to_string());

    let (tor_ip, is_tor, error) = match tor_res {
        Ok(out) if out.status.success() => {
            let body = String::from_utf8_lossy(&out.stdout);
            match parse_tor_check_json(&body) {
                Ok((is_tor, ip)) => (Some(ip), is_tor, None),
                Err(e) => (None, false, Some(e)),
            }
        }
        Ok(out) => {
            let err_msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            (
                None,
                false,
                Some(if err_msg.is_empty() {
                    "Could not reach Tor SOCKS5 at 127.0.0.1:9050".to_string()
                } else {
                    err_msg
                }),
            )
        }
        Err(e) => (None, false, Some(format!("Failed to run curl: {e}"))),
    };

    let (gnome_ip, gnome_is_tor, gnome_error) = match gnome_res {
        Ok((g_tor, g_ip)) => (Some(g_ip), g_tor, None),
        Err(e) => (None, false, Some(e)),
    };

    Ok(VerifyResult {
        direct_ip,
        tor_ip,
        is_tor,
        error,
        gnome_proxy_enabled: gnome_proxy.enabled_for_tor,
        gnome_ip,
        gnome_is_tor,
        gnome_error,
        http_tunnel_ok,
    })
}

#[tauri::command]
async fn get_bridges() -> Result<String, String> {
    let root = find_workspace_root();
    let bridges_path = root.join("bridges.txt");
    if bridges_path.exists() {
        fs::read_to_string(&bridges_path).map_err(|e| format!("Failed to read bridges.txt: {e}"))
    } else {
        let example_path = root.join("bridges.txt.example");
        Ok(fs::read_to_string(&example_path).unwrap_or_default())
    }
}

#[tauri::command]
async fn save_bridges(content: String) -> Result<usize, String> {
    let root = find_workspace_root();
    let bridges_path = root.join("bridges.txt");
    fs::write(&bridges_path, &content).map_err(|e| format!("Failed to write bridges.txt: {e}"))?;
    Ok(count_configured_bridges(&content))
}

#[tauri::command]
async fn get_tor_logs(lines: Option<usize>) -> Result<String, String> {
    let root = find_workspace_root();
    let log_path = root.join(".runtime/tor.log");
    let max_lines = lines.unwrap_or(80);
    match fs::read_to_string(&log_path) {
        Ok(content) => {
            let all: Vec<&str> = content.lines().collect();
            let start = all.len().saturating_sub(max_lines);
            Ok(all[start..].join("\n"))
        }
        Err(_) => Ok(String::from("No Tor logs yet. Start Tor to see live output.")),
    }
}

#[tauri::command]
async fn launch_tor_chromium() -> Result<String, String> {
    let root = find_workspace_root();
    let script = root.join("scripts/run-via-tor.sh");
    if !script.exists() {
        return Err(format!("Missing script: {}", script.display()));
    }

    Command::new(&script)
        .args(["--chromium", "https://check.torproject.org"])
        .current_dir(&root)
        .spawn()
        .map_err(|e| format!("Failed to launch Chromium: {e}"))?;

    Ok("Launched isolated Chromium via Tor SOCKS5".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_status,
            start_tor,
            stop_tor,
            set_gnome_proxy,
            verify_connection,
            get_bridges,
            save_bridges,
            get_tor_logs,
            launch_tor_chromium
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wireguard_interfaces_cleanly() {
        let sample = "work-vpn         UNKNOWN        <POINTOPOINT,NOARP,UP,LOWER_UP>\nwg0              UNKNOWN        <POINTOPOINT,NOARP,UP,LOWER_UP>\n";
        assert_eq!(parse_wireguard_interfaces(sample), vec!["work-vpn", "wg0"]);
        assert!(parse_wireguard_interfaces("").is_empty());
    }

    #[test]
    fn parses_tor_bootstrap_progress() {
        let log = "\
Nov 26 17:49:56.000 [notice] Bootstrapped 1% (conn_pt): Connecting to pluggable transport
Nov 26 17:49:57.000 [notice] Bootstrapped 75% (enough_dirinfo): Loaded enough directory info to build circuits
Nov 26 17:49:59.000 [notice] Bootstrapped 100% (done): Done
";
        let (pct, line) = parse_tor_bootstrap(log);
        assert_eq!(pct, 100);
        assert!(line.contains("Bootstrapped 100%"));
    }

    #[test]
    fn counts_only_active_obfs4_bridges() {
        let txt = "\
# comment
# obfs4 1.2.3.4:443 AAAA cert=xxx iat-mode=0
obfs4 198.51.100.10:8214 1B6C cert=yyy iat-mode=0
Bridge obfs4 203.0.113.20:9001 BBBB cert=zzz iat-mode=0
";
        assert_eq!(count_configured_bridges(txt), 2);
    }

    #[test]
    fn strips_gsettings_single_quotes() {
        assert_eq!(strip_gsettings_quotes("'manual'\n"), "manual");
        assert_eq!(strip_gsettings_quotes("9050\n"), "9050");
    }

    #[test]
    fn parses_tor_check_api_json() {
        let ok = parse_tor_check_json(r#"{"IsTor":true,"IP":"204.8.96.182"}"#).unwrap();
        assert_eq!(ok, (true, "204.8.96.182".to_string()));
        let direct = parse_tor_check_json(r#"{"IsTor":false,"IP":"198.51.100.5"}"#).unwrap();
        assert_eq!(direct, (false, "198.51.100.5".to_string()));
        assert!(parse_tor_check_json("not json").is_err());
    }
}
