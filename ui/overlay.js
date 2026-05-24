// Overlay frontend.
// Listens for backend events, renders suggestion list, handles focus & keyboard.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

const els = {
  shell:         document.getElementById("shell"),
  loading:       document.getElementById("state-loading"),
  error:         document.getElementById("state-error"),
  empty:         document.getElementById("state-empty"),
  list:          document.getElementById("suggestions"),
  brandSub:      document.getElementById("brand-sub"),
  errorTitle:    document.getElementById("error-title"),
  errorBody:     document.getElementById("error-body"),
  steerBanner:   document.getElementById("steer-banner"),
  steerText:     document.getElementById("steer-text"),
  closeBtn:      document.getElementById("close-btn"),
  regenBtn:      document.getElementById("regenerate-btn"),
  retryBtn:      document.getElementById("retry-btn"),
  errorCloseBtn: document.getElementById("error-close-btn"),
};

let suggestions = [];
let activeIndex = 0;
let busy = false;

function setMode(mode) {
  els.loading.classList.toggle("hidden", mode !== "loading");
  els.error.classList.toggle("hidden",   mode !== "error");
  els.empty.classList.toggle("hidden",   mode !== "empty");
  els.list.classList.toggle("hidden",    mode !== "list");

  els.loading.setAttribute("aria-busy", mode === "loading" ? "true" : "false");

  const subMap = {
    loading: "thinking…",
    list:    `${suggestions.length} ideas`,
    error:   "error",
    empty:   "waiting",
  };
  els.brandSub.textContent = subMap[mode] || "";
}

function setSteer(text) {
  if (text && text.trim()) {
    els.steerText.textContent = text.trim();
    els.steerBanner.classList.remove("hidden");
  } else {
    els.steerBanner.classList.add("hidden");
  }
}

function render() {
  els.list.innerHTML = "";
  suggestions.forEach((s, i) => {
    const li = document.createElement("li");
    li.className = "suggestion" + (i === activeIndex ? " is-active" : "");
    li.setAttribute("role", "option");
    li.setAttribute("aria-selected", i === activeIndex ? "true" : "false");
    li.setAttribute("tabindex", "-1");
    li.dataset.index = String(i);

    const num = document.createElement("span");
    num.className = "suggestion-num";
    num.textContent = String(i + 1);

    const body = document.createElement("div");
    body.style.flex = "1";

    const text = document.createElement("div");
    text.className = "suggestion-text";
    text.textContent = s.text;

    const meta = document.createElement("div");
    meta.className = "suggestion-meta";
    meta.innerHTML = `<kbd>${i + 1}</kbd> or <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Enter</kbd> to paste`;

    body.appendChild(text);
    body.appendChild(meta);
    li.appendChild(num);
    li.appendChild(body);

    li.addEventListener("mouseenter", () => {
      activeIndex = i;
      updateActive();
    });
    li.addEventListener("click", () => accept(i));
    els.list.appendChild(li);
  });
  updateActive();
}

function updateActive() {
  els.list.querySelectorAll(".suggestion").forEach((el, i) => {
    const active = i === activeIndex;
    el.classList.toggle("is-active", active);
    el.setAttribute("aria-selected", active ? "true" : "false");
    if (active) el.scrollIntoView({ block: "nearest" });
  });
}

async function accept(i) {
  if (busy) return;
  if (i < 0 || i >= suggestions.length) return;
  busy = true;
  els.brandSub.textContent = "pasting…";
  try {
    await invoke("accept_suggestion", { index: i });
  } catch (e) {
    showError("Paste failed", String(e));
  } finally {
    busy = false;
  }
}

async function dismiss() {
  els.shell.classList.add("is-leaving");
  setTimeout(async () => {
    try { await invoke("close_overlay"); } catch (_) {}
  }, 120);
}

async function regenerate() {
  if (busy) return;
  busy = true;
  setMode("loading");
  try {
    await invoke("regenerate_suggestions");
  } catch (e) {
    showError("Couldn't regenerate", String(e));
  } finally {
    busy = false;
  }
}

function showError(title, body) {
  els.errorTitle.textContent = title;
  els.errorBody.textContent = body;
  setMode("error");
}

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") { e.preventDefault(); dismiss(); return; }
  if (e.key === "ArrowDown") {
    e.preventDefault();
    if (suggestions.length) {
      activeIndex = (activeIndex + 1) % suggestions.length;
      updateActive();
    }
    return;
  }
  if (e.key === "ArrowUp") {
    e.preventDefault();
    if (suggestions.length) {
      activeIndex = (activeIndex - 1 + suggestions.length) % suggestions.length;
      updateActive();
    }
    return;
  }
  if (e.key === "Enter") {
    e.preventDefault();
    accept(activeIndex);
    return;
  }
  if (e.key === "r" && (e.ctrlKey || e.metaKey)) {
    e.preventDefault();
    regenerate();
    return;
  }
  if (e.key >= "1" && e.key <= "6") {
    const i = parseInt(e.key, 10) - 1;
    if (i < suggestions.length) {
      activeIndex = i;
      updateActive();
      accept(i);
    }
  }
});

els.closeBtn.addEventListener("click", dismiss);
els.regenBtn.addEventListener("click", regenerate);
els.retryBtn.addEventListener("click", regenerate);
els.errorCloseBtn.addEventListener("click", dismiss);

listen("suggestions", (event) => {
  const payload = event.payload || {};
  suggestions = Array.isArray(payload) ? payload : (payload.suggestions || []);
  if (!suggestions.length) {
    setMode("empty");
    return;
  }
  activeIndex = 0;
  render();
  setMode("list");
});

listen("steer", (event) => {
  setSteer(event.payload || "");
});

listen("suggestion-error", (event) => {
  const payload = event.payload || {};
  const title = (payload && payload.title) || "Couldn't generate suggestions";
  const body = (payload && payload.body) || String(payload);
  showError(title, body);
});

listen("accept-highlighted", () => accept(activeIndex));

setMode("loading");

window.addEventListener("load", () => {
  document.body.focus?.();
  getCurrentWindow().setFocus().catch(() => {});
});
