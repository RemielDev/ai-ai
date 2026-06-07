const { invoke } = window.__TAURI__.core;
const opener = window.__TAURI__.opener;
const { getCurrentWindow } = window.__TAURI__.window;

function goTo(step) {
  document.querySelectorAll(".wizard-step").forEach((el) => {
    el.classList.toggle("is-visible", el.dataset.step === String(step));
  });
  document.querySelectorAll(".wizard-dot").forEach((d) => {
    const n = parseInt(d.dataset.step, 10);
    d.classList.toggle("is-active", n === step);
    d.classList.toggle("is-done", n < step);
  });
  if (step === 3) refreshKeyStep();
}

document.querySelectorAll("[data-go]").forEach((btn) => {
  btn.addEventListener("click", () => goTo(parseInt(btn.dataset.go, 10)));
});

document.addEventListener("click", async (e) => {
  const a = e.target.closest("[data-external]");
  if (!a) return;
  e.preventDefault();
  try { if (opener && opener.openUrl) await opener.openUrl(a.getAttribute("data-external")); } catch (_) {}
});

function toast(msg, kind = "info") {
  const root = document.getElementById("toast-root");
  const el = document.createElement("div");
  el.className = "toast" + (kind === "success" ? " is-success" : kind === "error" ? " is-error" : "");
  el.textContent = msg;
  root.appendChild(el);
  setTimeout(() => { el.style.opacity = "0"; setTimeout(() => el.remove(), 200); }, 2400);
}

const providerSelect = document.getElementById("provider-select");
const providerInfo   = document.getElementById("provider-info");

function refreshProviderInfo() {
  const p = window.PROVIDERS[providerSelect.value];
  providerInfo.innerHTML = `Default model: <code>${p.defaultModel}</code> · Get a key at <a href="#" data-external="${p.consoleUrl}">${new URL(p.consoleUrl).host}</a>`;
}
providerSelect.addEventListener("change", refreshProviderInfo);
refreshProviderInfo();

function refreshKeyStep() {
  const p = window.PROVIDERS[providerSelect.value];
  document.getElementById("key-title").textContent = `Your ${p.label} key`;
  document.getElementById("key-blurb").innerHTML =
    `Get one at <a href="#" data-external="${p.consoleUrl}">${new URL(p.consoleUrl).host}</a>. AI-AI uses your key directly — prompts never touch our servers.`;
  const input = document.getElementById("key-input");
  input.placeholder = `${p.keyPrefix}...`;
}

document.getElementById("save-key-btn").addEventListener("click", async () => {
  const v = document.getElementById("key-input").value.trim();
  const fb = document.getElementById("key-feedback");
  const provider = providerSelect.value;
  if (!v) { fb.textContent = "Paste a key, or skip for now."; fb.style.color = "var(--danger)"; return; }
  try {
    await invoke("save_api_key", { provider, key: v });
    // Save provider + default model so validate uses the right config.
    await invoke("save_settings", {
      settings: {
        provider,
        model: window.PROVIDERS[provider].defaultModel,
        summon_hotkey: "Ctrl+Shift+Space",
        action_hotkey: "Ctrl+Shift+Enter",
        suggestion_count: 4,
        style_preset: "default",
        auto_send_after_paste: false,
        telemetry_opt_in: false,
        first_run: true,
        first_launch_at: new Date().toISOString(),
      },
    });
    try {
      await invoke("validate_api_key");
      fb.textContent = "Verified.";
      fb.style.color = "var(--success)";
      toast("Key saved and verified.", "success");
      setTimeout(() => goTo(4), 400);
    } catch (e) {
      fb.textContent = "Saved but provider didn't accept it: " + e;
      fb.style.color = "var(--warning)";
    }
  } catch (e) {
    fb.textContent = String(e);
    fb.style.color = "var(--danger)";
  }
});

document.getElementById("skip-key-btn").addEventListener("click", () => goTo(4));

document.getElementById("finish-btn").addEventListener("click", async () => {
  try { await invoke("mark_first_run_done"); } catch (_) {}
  try { await getCurrentWindow().close(); } catch (_) {}
});
