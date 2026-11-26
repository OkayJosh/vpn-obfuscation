import { invoke } from "@tauri-apps/api/core";

if (!("switch" in HTMLInputElement.prototype)) {
  import("input-switch-polyfill");
}

const el = {
  shieldBadge: document.getElementById("shieldBadge"),
  overallStatus: document.getElementById("overallStatus"),
  torDetailLine: document.getElementById("torDetailLine"),
  btnToggleTor: document.getElementById("btnToggleTor"),
  bootstrapLabel: document.getElementById("bootstrapLabel"),
  bootstrapPct: document.getElementById("bootstrapPct"),
  bootstrapProgress: document.getElementById("bootstrapProgress"),
  chkUseBridges: document.getElementById("chkUseBridges"),
  bridgeCountDesc: document.getElementById("bridgeCountDesc"),
  chkGnomeProxy: document.getElementById("chkGnomeProxy"),
  gnomeProxyDesc: document.getElementById("gnomeProxyDesc"),
  chkIncludeHttp: document.getElementById("chkIncludeHttp"),
  statVpn: document.getElementById("statVpn"),
  statVpnHint: document.getElementById("statVpnHint"),
  statGnome: document.getElementById("statGnome"),
  statGnomeHint: document.getElementById("statGnomeHint"),
  statDirectIp: document.getElementById("statDirectIp"),
  statTorIp: document.getElementById("statTorIp"),
  statTorHint: document.getElementById("statTorHint"),
  btnVerify: document.getElementById("btnVerify"),
  btnChromium: document.getElementById("btnChromium"),
  btnRefresh: document.getElementById("btnRefresh"),
  feedbackBanner: document.getElementById("feedbackBanner"),
  tabBridges: document.getElementById("tab-bridges"),
  tabLogs: document.getElementById("tab-logs"),
  panelBridges: document.getElementById("panel-bridges"),
  panelLogs: document.getElementById("panel-logs"),
  bridgesTextarea: document.getElementById("bridgesTextarea"),
  btnSaveBridges: document.getElementById("btnSaveBridges"),
  btnSaveRestart: document.getElementById("btnSaveRestart"),
  torLogsOutput: document.getElementById("torLogsOutput"),
};

let currentStatus = null;
let busy = false;
let activeTab = "bridges";
let bridgesLoaded = false;
let lastGnomeVerify = null;

function showFeedback(message, type = "info") {
  el.feedbackBanner.textContent = message;
  el.feedbackBanner.className = `feedback-banner ${type}`;
  el.feedbackBanner.hidden = false;
}

function clearFeedback() {
  el.feedbackBanner.hidden = true;
}

function renderStatus(status) {
  currentStatus = status;

  // VPN card
  if (status.vpn_active) {
    el.statVpn.textContent = status.vpn_interfaces.join(", ");
    el.statVpn.className = "stat-value ok";
    el.statVpnHint.textContent = "WireGuard interface UP";
  } else {
    el.statVpn.textContent = "Disconnected";
    el.statVpn.className = "stat-value warn";
    el.statVpnHint.textContent = "No WireGuard interface active";
  }

  // Tor status & bootstrap
  const pct = status.tor_running ? status.tor_bootstrap : 0;
  el.bootstrapProgress.value = pct;
  el.bootstrapPct.textContent = `${pct}%`;

  if (status.tor_running && pct === 100) {
    el.btnToggleTor.textContent = "Stop Tor";
    el.btnToggleTor.className = "btn btn-danger";
    const modeLabel = status.tor_using_bridges ? "obfs4 bridges" : "direct Tor";
    el.torDetailLine.textContent = `Running (PID ${status.tor_pid}) via ${modeLabel} — SOCKS5 127.0.0.1:9050`;
    el.bootstrapLabel.textContent = "Bootstrapped 100% — Ready";
  } else if (status.tor_running) {
    el.btnToggleTor.textContent = "Stop Tor";
    el.btnToggleTor.className = "btn btn-danger";
    el.torDetailLine.textContent = `Starting (PID ${status.tor_pid})…`;
    el.bootstrapLabel.textContent = status.tor_status_line || "Bootstrapping…";
  } else {
    el.btnToggleTor.textContent = "Start Tor";
    el.btnToggleTor.className = "btn btn-primary";
    el.torDetailLine.textContent = "Tor daemon is stopped";
    el.bootstrapLabel.textContent = "Idle";
  }

  // Overall status pill & shield
  if (status.tor_running && pct === 100 && status.gnome_proxy.enabled_for_tor) {
    el.overallStatus.textContent = "Protected (GNOME Proxy ON)";
    el.overallStatus.className = "status-pill protected";
    el.shieldBadge.className = "shield-badge active";
  } else if (status.tor_running && pct === 100) {
    el.overallStatus.textContent = "Tor Ready (Proxy OFF)";
    el.overallStatus.className = "status-pill protected";
    el.shieldBadge.className = "shield-badge active";
  } else if (status.tor_running) {
    el.overallStatus.textContent = `Connecting (${pct}%)…`;
    el.overallStatus.className = "status-pill connecting";
    el.shieldBadge.className = "shield-badge connecting";
  } else {
    el.overallStatus.textContent = "Inactive";
    el.overallStatus.className = "status-pill";
    el.shieldBadge.className = "shield-badge";
  }

  // Bridge info
  el.bridgeCountDesc.textContent =
    status.configured_bridges > 0
      ? `${status.configured_bridges} active obfs4 bridge(s) in bridges.txt`
      : "No active bridges in bridges.txt (will run without bridges if unchecked)";

  // Sync GNOME proxy toggles without firing change listeners
  el.chkGnomeProxy.disabled = !status.gnome_proxy.available;
  el.chkGnomeProxy.checked = status.gnome_proxy.enabled_for_tor;
  el.chkIncludeHttp.disabled = !status.gnome_proxy.available;
  el.chkIncludeHttp.checked = status.gnome_proxy.includes_http;

  if (!status.gnome_proxy.available) {
    el.gnomeProxyDesc.textContent = "gsettings not available on this desktop";
    el.statGnome.textContent = "Unavailable";
    el.statGnome.className = "stat-value warn";
    el.statGnomeHint.textContent = "gsettings not found";
  } else if (status.gnome_proxy.enabled_for_tor) {
    const extra = status.gnome_proxy.includes_http ? " + HTTP/HTTPS :8118" : "";
    el.gnomeProxyDesc.textContent = `ON — SOCKS5 127.0.0.1:9050${extra} (Chrome is routed through Tor)`;
    if (!lastGnomeVerify) {
      el.statGnome.textContent = "ON (Tor SOCKS5)";
      el.statGnome.className = "stat-value ok";
      el.statGnomeHint.textContent = `127.0.0.1:9050${extra} — click Verify to confirm via GNOME`;
    }
  } else {
    el.gnomeProxyDesc.textContent = `OFF (mode: ${status.gnome_proxy.mode}) — toggle ON to route Chrome through Tor`;
    el.statGnome.textContent = `OFF (${status.gnome_proxy.mode})`;
    el.statGnome.className = "stat-value warn";
    el.statGnomeHint.textContent = "Chrome using direct/VPN routing";
  }
}

async function refreshStatus() {
  try {
    const status = await invoke("get_status");
    renderStatus(status);
    if (activeTab === "logs") {
      const logs = await invoke("get_tor_logs", { lines: 80 });
      el.torLogsOutput.textContent = logs;
      el.torLogsOutput.scrollTop = el.torLogsOutput.scrollHeight;
    }
  } catch (err) {
    showFeedback(`Status error: ${err}`, "error");
  }
}

async function loadBridges() {
  try {
    const content = await invoke("get_bridges");
    el.bridgesTextarea.value = content;
    bridgesLoaded = true;
  } catch (err) {
    showFeedback(`Failed to load bridges.txt: ${err}`, "error");
  }
}

el.btnToggleTor.addEventListener("click", async () => {
  if (busy) return;
  busy = true;
  el.btnToggleTor.disabled = true;
  clearFeedback();

  try {
    if (currentStatus?.tor_running) {
      await invoke("stop_tor", { disableGnomeProxy: true });
      showFeedback("Tor stopped (GNOME proxy automatically turned OFF so Chrome stays online).", "info");
    } else {
      const useBridges = el.chkUseBridges.checked;
      await invoke("start_tor", { useBridges });
      showFeedback(
        useBridges
          ? "Starting Tor with obfs4 bridges… waiting for 100% bootstrap."
          : "Starting Tor without bridges… waiting for 100% bootstrap.",
        "success"
      );
    }
    await refreshStatus();
  } catch (err) {
    showFeedback(`Tor error: ${err}`, "error");
  } finally {
    busy = false;
    el.btnToggleTor.disabled = false;
  }
});

async function applyGnomeProxy() {
  clearFeedback();
  lastGnomeVerify = null;
  const enabled = el.chkGnomeProxy.checked;
  const includeHttp = el.chkIncludeHttp.checked;

  if (enabled && (!currentStatus?.tor_running || currentStatus.tor_bootstrap < 100)) {
    showFeedback(
      "Heads up: Tor is not at 100% yet. Chrome may wait until Tor finishes bootstrapping.",
      "info"
    );
  }

  try {
    await invoke("set_gnome_proxy", { enabled, includeHttp });
    await refreshStatus();
    if (enabled) {
      showFeedback("GNOME System Proxy enabled — Chrome traffic is now routed through Tor (127.0.0.1:9050).", "success");
    } else {
      showFeedback("GNOME System Proxy disabled — Chrome is back to direct/VPN routing.", "info");
    }
  } catch (err) {
    showFeedback(`Failed to update GNOME proxy: ${err}`, "error");
    await refreshStatus();
  }
}

el.chkGnomeProxy.addEventListener("change", applyGnomeProxy);
el.chkIncludeHttp.addEventListener("change", () => {
  if (el.chkGnomeProxy.checked) {
    applyGnomeProxy();
  }
});

el.btnVerify.addEventListener("click", async () => {
  el.btnVerify.disabled = true;
  el.btnVerify.textContent = "Verifying…";
  clearFeedback();

  try {
    const res = await invoke("verify_connection");
    el.statDirectIp.textContent = res.direct_ip || "unreachable";

    // Update Tor Exit IP card
    if (res.is_tor && res.tor_ip) {
      el.statTorIp.textContent = res.tor_ip;
      el.statTorIp.className = "stat-value mono ok";
      el.statTorHint.textContent = "Verified: IsTor = true";
    } else {
      el.statTorIp.textContent = "Not connected";
      el.statTorIp.className = "stat-value warn";
      el.statTorHint.textContent = res.error || "IsTor = false";
    }

    // Update GNOME Proxy card with real GNOME/GIO verification
    lastGnomeVerify = res;
    if (res.gnome_proxy_enabled) {
      if (res.gnome_is_tor && res.gnome_ip) {
        const httpNote =
          res.http_tunnel_ok === true
            ? " • HTTP :8118 OK"
            : res.http_tunnel_ok === false
              ? " • HTTP :8118 FAILED"
              : "";
        el.statGnome.textContent = "Verified via GNOME";
        el.statGnome.className = "stat-value ok";
        el.statGnomeHint.textContent = `GNOME exit IP: ${res.gnome_ip}${httpNote}`;
      } else {
        el.statGnome.textContent = "GNOME Check Failed";
        el.statGnome.className = "stat-value warn";
        el.statGnomeHint.textContent =
          res.gnome_error || `GNOME exited directly (${res.gnome_ip || "unknown"})`;
      }
    } else {
      el.statGnome.textContent = "OFF (Direct/VPN)";
      el.statGnome.className = "stat-value warn";
      el.statGnomeHint.textContent = res.gnome_ip
        ? `GNOME apps exiting via ${res.gnome_ip}`
        : "Toggle GNOME System Proxy ON to route Chrome";
    }

    if (res.is_tor && res.gnome_proxy_enabled && res.gnome_is_tor) {
      showFeedback(
        `All checks passed! Direct/VPN: ${res.direct_ip} → Tor: ${res.tor_ip} • GNOME verified: ${res.gnome_ip}`,
        "success"
      );
    } else if (res.is_tor && !res.gnome_proxy_enabled) {
      showFeedback(
        `Tor is working (${res.tor_ip}), but GNOME System Proxy is OFF. Toggle it ON to route Chrome through Tor.`,
        "info"
      );
    } else if (res.is_tor && res.gnome_proxy_enabled && !res.gnome_is_tor) {
      showFeedback(
        `Tor works (${res.tor_ip}), but GNOME check failed: ${res.gnome_error || "GNOME did not exit via Tor"}`,
        "error"
      );
    } else {
      showFeedback(res.error || "Traffic is not exiting through Tor.", "error");
    }
  } catch (err) {
    showFeedback(`Verification failed: ${err}`, "error");
  } finally {
    el.btnVerify.disabled = false;
    el.btnVerify.textContent = "Verify IP & Leak Check";
  }
});

el.btnChromium.addEventListener("click", async () => {
  clearFeedback();
  try {
    const msg = await invoke("launch_tor_chromium");
    showFeedback(msg, "success");
  } catch (err) {
    showFeedback(`Could not launch Chromium: ${err}`, "error");
  }
});

el.btnRefresh.addEventListener("click", () => {
  clearFeedback();
  refreshStatus();
});

el.btnSaveBridges.addEventListener("click", async () => {
  clearFeedback();
  try {
    const count = await invoke("save_bridges", { content: el.bridgesTextarea.value });
    showFeedback(`Saved bridges.txt (${count} active obfs4 bridge(s)).`, "success");
    await refreshStatus();
  } catch (err) {
    showFeedback(`Save failed: ${err}`, "error");
  }
});

el.btnSaveRestart.addEventListener("click", async () => {
  clearFeedback();
  el.btnSaveRestart.disabled = true;
  try {
    const count = await invoke("save_bridges", { content: el.bridgesTextarea.value });
    if (count === 0) {
      showFeedback("Saved, but no active obfs4 lines found. Add at least one obfs4 line first.", "error");
      return;
    }
    el.chkUseBridges.checked = true;
    await invoke("stop_tor", { disableGnomeProxy: false });
    await invoke("start_tor", { useBridges: true });
    showFeedback(`Saved ${count} bridge(s) and restarted Tor with obfs4.`, "success");
    await refreshStatus();
  } catch (err) {
    showFeedback(`Save & Restart failed: ${err}`, "error");
  } finally {
    el.btnSaveRestart.disabled = false;
  }
});

function switchTab(tab) {
  activeTab = tab;
  const isBridges = tab === "bridges";
  el.tabBridges.classList.toggle("active", isBridges);
  el.tabBridges.setAttribute("aria-selected", String(isBridges));
  el.panelBridges.hidden = !isBridges;

  el.tabLogs.classList.toggle("active", !isBridges);
  el.tabLogs.setAttribute("aria-selected", String(!isBridges));
  el.panelLogs.hidden = isBridges;

  if (!isBridges) {
    refreshStatus();
  } else if (!bridgesLoaded) {
    loadBridges();
  }
}

el.tabBridges.addEventListener("click", () => switchTab("bridges"));
el.tabLogs.addEventListener("click", () => switchTab("logs"));

// Initial load + gentle 2.5s polling for live bootstrap & proxy sync
await loadBridges();
await refreshStatus();
setInterval(refreshStatus, 2500);
