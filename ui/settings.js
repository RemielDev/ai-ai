// Settings frontend.

const { invoke } = window.__TAURI__.core;

const els = {
  apiKey: document.getElementById("api-key-input"),
  saveKey: document.getElementById("save-key"),
  clearKey: document.getElementById("clear-key"),
  keyStatus: document.getElementById("key-status"),
  model: document.getElementById("model-select"),
  summon: document.getElementById("summon-hotkey"),
  action: document.getElementById("action-hotkey"),
  count: document.getElementById("count"),
  style: document.getElementById("style"),
  autoSend: document.getElementById("auto-send"),
  telemetry: document.getElementById("telemetry"),
  saveAll: document.getElementById("save-all"),
  savedFlash: document.getElementById("saved-flash"),
  licenseStatus: document.getElementById("license-status"),
  licenseKey: document.getElementById("license-key"),
  activateLicense: document.getElementById("activate-license"),
};

async function refreshKeyStatus() {
  const present = await invoke("get_api_key_status");
  if (present) {
    els.keyStatus.textContent = "✓ API key saved in Windows Credential Manager";
    els.keyStatus.className = "status ok";
  } else {
    els.keyStatus.textContent = "No API key set — features will be disabled.";
    els.keyStatus.className = "status err";
  }
}

async function refreshLicense() {
  try {
    const s = await invoke("license_status");
    els.licenseStatus.textContent = `${s.valid ? "✓" : "✗"} ${s.tier} — ${s.message}`;
    els.licenseStatus.className = "status " + (s.valid ? "ok" : "err");
  } catch (e) {
    els.licenseStatus.textContent = String(e);
    els.licenseStatus.className = "status err";
  }
}

async function loadSettings() {
  const s = await invoke("load_settings");
  els.model.value = s.model;
  els.summon.value = s.summon_hotkey;
  els.action.value = s.action_hotkey;
  els.count.value = s.suggestion_count;
  els.style.value = s.style_preset;
  els.autoSend.checked = s.auto_send_after_paste;
  els.telemetry.checked = s.telemetry_opt_in;
}

function readSettings() {
  return {
    model: els.model.value,
    summon_hotkey: els.summon.value.trim(),
    action_hotkey: els.action.value.trim(),
    suggestion_count: Math.max(3, Math.min(6, parseInt(els.count.value, 10) || 4)),
    style_preset: els.style.value,
    auto_send_after_paste: els.autoSend.checked,
    telemetry_opt_in: els.telemetry.checked,
    first_run: false,
  };
}

function flashSaved() {
  els.savedFlash.classList.remove("hidden");
  setTimeout(() => els.savedFlash.classList.add("hidden"), 1400);
}

els.saveKey.addEventListener("click", async () => {
  try {
    await invoke("save_api_key", { key: els.apiKey.value });
    els.apiKey.value = "";
    await refreshKeyStatus();
  } catch (e) {
    alert(String(e));
  }
});

els.clearKey.addEventListener("click", async () => {
  await invoke("clear_api_key");
  await refreshKeyStatus();
});

els.saveAll.addEventListener("click", async () => {
  try {
    await invoke("save_settings", { settings: readSettings() });
    flashSaved();
  } catch (e) {
    alert("Failed to save: " + e);
  }
});

els.activateLicense.addEventListener("click", async () => {
  const s = await invoke("activate_license", { key: els.licenseKey.value });
  els.licenseStatus.textContent = `${s.valid ? "✓" : "✗"} ${s.tier} — ${s.message}`;
  els.licenseStatus.className = "status " + (s.valid ? "ok" : "err");
});

(async () => {
  await Promise.all([loadSettings(), refreshKeyStatus(), refreshLicense()]);
})();
