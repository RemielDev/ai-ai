// Settings frontend - multi-provider, tabbed, with hotkey capture and live validation.

const { invoke } = window.__TAURI__.core;
const opener = window.__TAURI__.opener;

const els = {
  // Setup - provider
  provider:      document.getElementById("provider-select"),
  providerCost:  document.getElementById("provider-cost"),
  model:         document.getElementById("model-input"),
  modelHint:     document.getElementById("model-hint"),
  // Setup - key
  keyCardTitle:  document.getElementById("key-card-title"),
  keyDesc:       document.getElementById("key-desc"),
  keyConsoleLink: document.getElementById("key-console-link"),
  keyPrefixHint: document.getElementById("key-prefix-hint"),
  apiKey:        document.getElementById("api-key-input"),
  saveKey:       document.getElementById("save-key"),
  clearKey:      document.getElementById("clear-key"),
  keyStatus:     document.getElementById("key-status"),
  keyMissingBanner: document.getElementById("key-missing-banner"),
  // Setup - hotkeys
  summon:        document.getElementById("summon-hotkey"),
  action:        document.getElementById("action-hotkey"),
  summonKeys:    document.getElementById("summon-keys"),
  actionKeys:    document.getElementById("action-keys"),
  // Behavior
  count:         document.getElementById("count"),
  countDisplay:  document.getElementById("count-display"),
  style:         document.getElementById("style"),
  autoSend:      document.getElementById("auto-send"),
  autostart:     document.getElementById("autostart"),
  // Privacy
  telemetry:     document.getElementById("telemetry"),
  licenseStatus: document.getElementById("license-status"),
  resetAll:      document.getElementById("reset-all"),
  // About
  appVersion:    document.getElementById("app-version"),
  aboutVersion:  document.getElementById("about-version"),
  aboutCount:    document.getElementById("about-count"),
  checkUpdates:  document.getElementById("check-updates"),
  updateStatus:  document.getElementById("update-status"),
  // (Trial removed - this is the OSS build, always free.)
  // Actions
  saveAll:       document.getElementById("save-all"),
  revert:        document.getElementById("revert"),
  savedFlash:    document.getElementById("saved-flash"),
  toastRoot:     document.getElementById("toast-root"),
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
  setTimeout(() => { el.style.opacity = "0"; setTimeout(() => el.remove(), 200); }, ms);
}

// ============ Provider switching ============
function applyProvider(slug, opts = {}) {
  const p = window.PROVIDERS[slug];
  if (!p) return;
  els.providerCost.textContent = p.cost;
  els.keyCardTitle.textContent = `${p.label} API key`;
  els.keyConsoleLink.textContent = new URL(p.consoleUrl).host;
  els.keyConsoleLink.dataset.external = p.consoleUrl;
  els.keyPrefixHint.textContent = p.keyPrefix;
  els.apiKey.placeholder = `${p.keyPrefix}...`;
  els.modelHint.textContent = p.modelHint;
  if (opts.resetModel) {
    els.model.value = p.defaultModel;
  }
  refreshKeyStatus(slug);
}

els.provider.addEventListener("change", () => {
  applyProvider(els.provider.value, { resetModel: true });
});

// ============ Hotkey capture ============
function renderHotkey(value, container) {
  container.innerHTML = "";
  if (!value) {
    const p = document.createElement("span");
    p.style.color = "var(--text-muted)";
    p.textContent = "Not set";
    container.appendChild(p);
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
  if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
  e.preventDefault(); e.stopPropagation();
  const parts = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.shiftKey) parts.push("Shift");
  if (e.altKey) parts.push("Alt");
  let key = e.key;
  if (key === " ") key = "Space";
  else if (key === "Escape") { stopRecording(); return; }
  else if (key.length === 1) key = key.toUpperCase();
  if (!parts.length) { toast("Hotkey needs a modifier (Ctrl/Shift/Alt).", "error"); return; }
  parts.push(key);
  const value = parts.join("+");
  document.getElementById(recordingTarget.targetId).value = value;
  const display = recordingTarget.targetId === "summon-hotkey" ? els.summonKeys : els.actionKeys;
  renderHotkey(value, display);
  stopRecording();
}, true);

// ============ External links ============
document.addEventListener("click", async (e) => {
  const a = e.target.closest("[data-external]");
  if (!a || !a.dataset.external) return;
  e.preventDefault();
  try { if (opener && opener.openUrl) await opener.openUrl(a.dataset.external); } catch (_) {}
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

async function refreshKeyStatus(provider) {
  setKeyStatus("loading", "Checking…");
  try {
    const present = await invoke("get_api_key_status", { provider });
    setKeyStatus(present ? "ok" : "danger", present ? "API key saved" : "No API key set");
  } catch (e) {
    setKeyStatus("danger", String(e));
  }
}

// License panel is static in OSS build; no async refresh needed.
async function refreshLicense() { /* no-op */ }

function updateCountDisplay() {
  els.countDisplay.textContent = `${els.count.value} suggestions`;
}

// ============ Load / save ============
async function loadAll() {
  const s = await invoke("load_settings");
  originalSettings = s;
  els.provider.value = s.provider;
  els.model.value = s.model;
  applyProvider(s.provider, { resetModel: false });

  els.summon.value = s.summon_hotkey;
  els.action.value = s.action_hotkey;
  els.count.value = s.suggestion_count;
  els.style.value = s.style_preset;
  els.autoSend.checked = s.auto_send_after_paste;
  els.telemetry.checked = s.telemetry_opt_in;
  renderHotkey(s.summon_hotkey, els.summonKeys);
  renderHotkey(s.action_hotkey, els.actionKeys);
  updateCountDisplay();

  try {
    const enabled = await invoke("get_autostart");
    els.autostart.checked = !!enabled;
  } catch (_) {}

  try {
    const info = await invoke("get_app_info");
    if (info) {
      els.appVersion.textContent   = "v" + info.version;
      els.aboutVersion.textContent = info.version;
      els.aboutCount.textContent   = info.suggestions_today;
    }
  } catch (_) {}

}

function readSettings() {
  return {
    provider: els.provider.value,
    model: els.model.value.trim() || window.PROVIDERS[els.provider.value].defaultModel,
    summon_hotkey: els.summon.value.trim(),
    action_hotkey: els.action.value.trim(),
    suggestion_count: parseInt(els.count.value, 10) || 4,
    style_preset: els.style.value,
    auto_send_after_paste: els.autoSend.checked,
    telemetry_opt_in: els.telemetry.checked,
    first_run: false,
    first_launch_at: (originalSettings && originalSettings.first_launch_at) || new Date().toISOString(),
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
  const provider = els.provider.value;
  els.saveKey.disabled = true;
  setKeyStatus("loading", "Saving…");
  try {
    await invoke("save_api_key", { provider, key: v });
    // Persist current provider+model so validate_api_key uses the right config.
    await invoke("save_settings", { settings: readSettings() });
    setKeyStatus("loading", "Verifying…");
    try {
      await invoke("validate_api_key");
      setKeyStatus("ok", "API key verified");
      els.apiKey.value = "";
      toast("Key saved and verified.", "success");
    } catch (e) {
      setKeyStatus("warn", "Saved, but verify failed: " + e);
      toast("Saved, but the provider didn't accept it.", "error");
    }
  } catch (e) {
    setKeyStatus("danger", String(e));
    toast(String(e), "error");
  } finally {
    els.saveKey.disabled = false;
  }
});

els.clearKey.addEventListener("click", async () => {
  await invoke("clear_api_key", { provider: els.provider.value });
  await refreshKeyStatus(els.provider.value);
  toast("API key removed.", "success");
});

els.saveAll.addEventListener("click", async () => {
  try {
    await invoke("save_settings", { settings: readSettings() });
    try { await invoke("set_autostart", { enabled: els.autostart.checked }); } catch (_) {}
    originalSettings = readSettings();
    flashSaved();
  } catch (e) {
    toast("Failed to save: " + e, "error");
  }
});

els.revert.addEventListener("click", () => {
  if (!originalSettings) return;
  els.provider.value = originalSettings.provider;
  els.model.value = originalSettings.model;
  applyProvider(originalSettings.provider, { resetModel: false });
  els.summon.value = originalSettings.summon_hotkey;
  els.action.value = originalSettings.action_hotkey;
  els.count.value = originalSettings.suggestion_count;
  els.style.value = originalSettings.style_preset;
  els.autoSend.checked = originalSettings.auto_send_after_paste;
  els.telemetry.checked = originalSettings.telemetry_opt_in;
  renderHotkey(originalSettings.summon_hotkey, els.summonKeys);
  renderHotkey(originalSettings.action_hotkey, els.actionKeys);
  updateCountDisplay();
  toast("Reverted.", "info");
});

// Activate-license removed in OSS build.

els.resetAll.addEventListener("click", async () => {
  if (!confirm("Reset all settings and remove every saved API key? This can't be undone.")) return;
  try {
    await invoke("reset_all");
    await loadAll();
    toast("Everything reset.", "success");
  } catch (e) {
    toast("Reset failed: " + e, "error");
  }
});

els.count.addEventListener("input", updateCountDisplay);

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

const starCta = document.getElementById("star-cta");
if (starCta) starCta.addEventListener("click", async () => {
  try { if (opener && opener.openUrl) await opener.openUrl("https://github.com/RemielDev/ai-ai"); } catch (_) {}
});

(async () => { await Promise.all([loadAll(), refreshLicense()]); })();
