// Settings frontend.
// Tabs, hotkey capture widget, live validation, toast notifications.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const opener = window.__TAURI__.opener;

const els = {
  // Setup
  apiKey:       document.getElementById("api-key-input"),
  saveKey:      document.getElementById("save-key"),
  clearKey:     document.getElementById("clear-key"),
  keyStatus:    document.getElementById("key-status"),
  keyHint:      document.getElementById("key-hint"),
  keyMissingBanner: document.getElementById("key-missing-banner"),
  model:        document.getElementById("model-select"),
  modelCost:    document.getElementById("model-cost"),
  summon:       document.getElementById("summon-hotkey"),
  action:       document.getElementById("action-hotkey"),
  summonKeys:   document.getElementById("summon-keys"),
  actionKeys:   document.getElementById("action-keys"),
  // Behavior
  count:        document.getElementById("count"),
  countDisplay: document.getElementById("count-display"),
  style:        document.getElementById("style"),
  autoSend:     document.getElementById("auto-send"),
  autostart:    document.getElementById("autostart"),
  // Privacy
  telemetry:    document.getElementById("telemetry"),
  licenseStatus: document.getElementById("license-status"),
  licenseKey:   document.getElementById("license-key"),
  activateLicense: document.getElementById("activate-license"),
  resetAll:     document.getElementById("reset-all"),
  // About
  appVersion:   document.getElementById("app-version"),
  aboutVersion: document.getElementById("about-version"),
  aboutCount:   document.getElementById("about-count"),
  checkUpdates: document.getElementById("check-updates"),
  updateStatus: document.getElementById("update-status"),
  // Trial
  trialBanner:  document.getElementById("trial-banner"),
  trialDays:    document.getElementById("trial-days-left"),
  trialBuy:     document.getElementById("trial-buy"),
  // Actions
  saveAll:      document.getElementById("save-all"),
  revert:       document.getElementById("revert"),
  savedFlash:   document.getElementById("saved-flash"),
  toastRoot:    document.getElementById("toast-root"),
};

const MODEL_COST = {
  "claude-haiku-4-6":  "~$0.001 / call",
  "claude-sonnet-4-7": "~$0.01 / call",
};

let originalSettings = null;
let recordingTarget = null;

// ============ Tabs ============
const tabs = document.querySelectorAll('[role="tab"]');
const panels = document.querySelectorAll('[role="tabpanel"]');
tabs.forEach((tab) => {
  tab.addEventListener("click", () => activateTab(tab.id));
  tab.addEventListener("keydown", (e) => {
    const order = Array.from(tabs);
    const idx = order.indexOf(tab);
    if (e.key === "ArrowRight") order[(idx + 1) % order.length].focus();
    if (e.key === "ArrowLeft")  order[(idx - 1 + order.length) % order.length].focus();
    if (e.key === "Home")       order[0].focus();
    if (e.key === "End")        order[order.length - 1].focus();
  });
});
function activateTab(id) {
  tabs.forEach((t) => t.setAttribute("aria-selected", t.id === id ? "true" : "false"));
  panels.forEach((p) => {
    const isMatch = p.getAttribute("aria-labelledby") === id;
    p.classList.toggle("is-visible", isMatch);
  });
}

// ============ Toasts ============
function toast(msg, kind = "info", ms = 2400) {
  const el = document.createElement("div");
  el.className = "toast" + (kind === "success" ? " is-success" : kind === "error" ? " is-error" : "");
  el.textContent = msg;
  els.toastRoot.appendChild(el);
  setTimeout(() => {
    el.style.opacity = "0";
    setTimeout(() => el.remove(), 200);
  }, ms);
}

// ============ Hotkey capture ============
function renderHotkey(value, container) {
  container.innerHTML = "";
  if (!value) {
    const placeholder = document.createElement("span");
    placeholder.style.color = "var(--text-muted)";
    placeholder.textContent = "Not set";
    container.appendChild(placeholder);
    return;
  }
  value.split("+").forEach((part) => {
    const kbd = document.createElement("kbd");
    kbd.textContent = part.trim();
    container.appendChild(kbd);
  });
}

function startRecording(targetId, recorder) {
  recordingTarget = { targetId, recorder };
  recorder.classList.add("is-recording");
  recorder.querySelector(".hotkey-record-action").textContent = "Press keys…";
}

function stopRecording() {
  if (!recordingTarget) return;
  recordingTarget.recorder.classList.remove("is-recording");
  recordingTarget.recorder.querySelector(".hotkey-record-action").textContent = "Record";
  recordingTarget = null;
}

document.querySelectorAll(".hotkey-recorder").forEach((rec) => {
  rec.addEventListener("click", () => startRecording(rec.dataset.target, rec));
  rec.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      startRecording(rec.dataset.target, rec);
    }
  });
});

document.addEventListener("keydown", (e) => {
  if (!recordingTarget) return;
  // Ignore lone modifier presses
  if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
  e.preventDefault();
  e.stopPropagation();

  const parts = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.shiftKey) parts.push("Shift");
  if (e.altKey) parts.push("Alt");

  let key = e.key;
  if (key === " ") key = "Space";
  else if (key === "Enter") key = "Enter";
  else if (key === "Escape") {
    stopRecording();
    return;
  } else if (key.length === 1) key = key.toUpperCase();

  if (!parts.length) {
    toast("Hotkey needs a modifier (Ctrl, Shift, or Alt).", "error");
    return;
  }
  parts.push(key);
  const value = parts.join("+");

  document.getElementById(recordingTarget.targetId).value = value;
  const display = recordingTarget.targetId === "summon-hotkey" ? els.summonKeys : els.actionKeys;
  renderHotkey(value, display);
  stopRecording();
}, true);

// ============ External link interception ============
document.addEventListener("click", async (e) => {
  const a = e.target.closest("[data-external]");
  if (!a) return;
  e.preventDefault();
  try {
    if (opener && opener.openUrl) {
      await opener.openUrl(a.getAttribute("data-external"));
    }
  } catch (err) {
    console.warn("opener failed", err);
  }
});

// ============ Status helpers ============
function setKeyStatus(state, msg) {
  els.keyStatus.className = "status-line";
  let icon = "loader";
  if (state === "ok")      { els.keyStatus.classList.add("is-ok");      icon = "check"; }
  if (state === "warn")    { els.keyStatus.classList.add("is-warn");    icon = "alert"; }
  if (state === "danger")  { els.keyStatus.classList.add("is-danger");  icon = "alert"; }
  if (state === "loading") { els.keyStatus.classList.add("is-loading"); icon = "loader"; }
  els.keyStatus.innerHTML = `<span data-icon="${icon}"></span> ${msg}`;
  window.injectIcons(els.keyStatus);
  els.keyMissingBanner.classList.toggle("hidden", state !== "danger");
}

async function refreshKeyStatus() {
  setKeyStatus("loading", "Checking…");
  try {
    const present = await invoke("get_api_key_status");
    if (present) setKeyStatus("ok", "API key saved");
    else         setKeyStatus("danger", "No API key set");
  } catch (e) {
    setKeyStatus("danger", String(e));
  }
}

async function refreshLicense() {
  try {
    const s = await invoke("license_status");
    els.licenseStatus.className = "status-line " + (s.valid ? "is-ok" : "is-warn");
    els.licenseStatus.innerHTML =
      `<span data-icon="${s.valid ? "check" : "alert"}"></span> ${s.tier} — ${s.message}`;
    window.injectIcons(els.licenseStatus);
  } catch (e) {
    els.licenseStatus.className = "status-line is-danger";
    els.licenseStatus.innerHTML = `<span data-icon="alert"></span> ${e}`;
    window.injectIcons(els.licenseStatus);
  }
}

function updateCountDisplay() {
  els.countDisplay.textContent = `${els.count.value} suggestions`;
}

function updateModelCost() {
  els.modelCost.textContent = MODEL_COST[els.model.value] || "";
}

// ============ Load / save ============
async function loadAll() {
  const s = await invoke("load_settings");
  originalSettings = s;
  els.model.value = s.model;
  els.summon.value = s.summon_hotkey;
  els.action.value = s.action_hotkey;
  els.count.value = s.suggestion_count;
  els.style.value = s.style_preset;
  els.autoSend.checked = s.auto_send_after_paste;
  els.telemetry.checked = s.telemetry_opt_in;

  renderHotkey(s.summon_hotkey, els.summonKeys);
  renderHotkey(s.action_hotkey, els.actionKeys);
  updateCountDisplay();
  updateModelCost();

  // Autostart status from backend
  try {
    const enabled = await invoke("get_autostart");
    els.autostart.checked = !!enabled;
  } catch (_) { /* autostart plugin unavailable */ }

  // Stats
  try {
    const info = await invoke("get_app_info");
    if (info && info.version) {
      els.appVersion.textContent   = "v" + info.version;
      els.aboutVersion.textContent = info.version;
    }
    if (info && typeof info.suggestions_today === "number") {
      els.aboutCount.textContent = info.suggestions_today;
    }
  } catch (_) {}

  // Trial
  try {
    const trial = await invoke("trial_status");
    if (trial && trial.in_trial) {
      els.trialDays.textContent = trial.days_left;
      els.trialBanner.classList.remove("hidden");
    } else {
      els.trialBanner.classList.add("hidden");
    }
  } catch (_) {}
}

function readSettings() {
  return {
    model: els.model.value,
    summon_hotkey: els.summon.value.trim(),
    action_hotkey: els.action.value.trim(),
    suggestion_count: parseInt(els.count.value, 10) || 4,
    style_preset: els.style.value,
    auto_send_after_paste: els.autoSend.checked,
    telemetry_opt_in: els.telemetry.checked,
    first_run: false,
  };
}

function flashSaved() {
  els.savedFlash.classList.remove("hidden");
  window.injectIcons(els.savedFlash);
  setTimeout(() => els.savedFlash.classList.add("hidden"), 1800);
}

// ============ Event wiring ============
els.saveKey.addEventListener("click", async () => {
  const v = els.apiKey.value.trim();
  if (!v) { toast("Paste a key first.", "error"); return; }
  els.saveKey.disabled = true;
  setKeyStatus("loading", "Validating…");
  try {
    await invoke("save_api_key", { key: v });
    // Probe with a tiny request
    try {
      await invoke("validate_api_key");
      setKeyStatus("ok", "API key verified");
      els.apiKey.value = "";
      toast("API key saved and verified.", "success");
    } catch (e) {
      setKeyStatus("warn", "Saved, but couldn't verify: " + e);
      toast("Saved, but Anthropic didn't accept it.", "error");
    }
  } catch (e) {
    setKeyStatus("danger", String(e));
    toast(String(e), "error");
  } finally {
    els.saveKey.disabled = false;
  }
});

els.clearKey.addEventListener("click", async () => {
  await invoke("clear_api_key");
  await refreshKeyStatus();
  toast("API key removed.", "success");
});

els.saveAll.addEventListener("click", async () => {
  try {
    const s = readSettings();
    await invoke("save_settings", { settings: s });
    try { await invoke("set_autostart", { enabled: els.autostart.checked }); } catch (_) {}
    originalSettings = s;
    flashSaved();
  } catch (e) {
    toast("Failed to save: " + e, "error");
  }
});

els.revert.addEventListener("click", () => {
  if (!originalSettings) return;
  els.model.value = originalSettings.model;
  els.summon.value = originalSettings.summon_hotkey;
  els.action.value = originalSettings.action_hotkey;
  els.count.value = originalSettings.suggestion_count;
  els.style.value = originalSettings.style_preset;
  els.autoSend.checked = originalSettings.auto_send_after_paste;
  els.telemetry.checked = originalSettings.telemetry_opt_in;
  renderHotkey(originalSettings.summon_hotkey, els.summonKeys);
  renderHotkey(originalSettings.action_hotkey, els.actionKeys);
  updateCountDisplay();
  updateModelCost();
  toast("Reverted.", "info");
});

els.activateLicense.addEventListener("click", async () => {
  const k = els.licenseKey.value.trim();
  if (!k) { toast("Paste a license key first.", "error"); return; }
  const s = await invoke("activate_license", { key: k });
  await refreshLicense();
  if (s.valid) toast("License activated.", "success");
  else toast("Activation failed: " + s.message, "error");
});

els.resetAll.addEventListener("click", async () => {
  if (!confirm("Reset all settings and remove your API key? This can't be undone.")) return;
  try {
    await invoke("reset_all");
    await loadAll();
    await refreshKeyStatus();
    toast("Everything reset.", "success");
  } catch (e) {
    toast("Reset failed: " + e, "error");
  }
});

els.count.addEventListener("input", updateCountDisplay);
els.model.addEventListener("change", updateModelCost);

els.checkUpdates.addEventListener("click", async () => {
  els.updateStatus.className = "status-line is-loading";
  els.updateStatus.innerHTML = `<span data-icon="loader"></span> Checking…`;
  window.injectIcons(els.updateStatus);
  try {
    const r = await invoke("check_for_updates");
    els.updateStatus.className = "status-line is-ok";
    els.updateStatus.innerHTML = `<span data-icon="check"></span> ${r}`;
    window.injectIcons(els.updateStatus);
  } catch (e) {
    els.updateStatus.className = "status-line is-warn";
    els.updateStatus.innerHTML = `<span data-icon="alert"></span> ${e}`;
    window.injectIcons(els.updateStatus);
  }
});

els.trialBuy.addEventListener("click", async () => {
  try {
    if (opener && opener.openUrl) await opener.openUrl("https://gumroad.com");
  } catch (_) {}
});

// Init
(async () => {
  await Promise.all([loadAll(), refreshKeyStatus(), refreshLicense()]);
})();
