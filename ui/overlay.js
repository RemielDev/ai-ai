// Overlay frontend — receives suggestions from backend, handles nav & accept.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

const stateLoading = document.getElementById("state-loading");
const stateError = document.getElementById("state-error");
const errorBody = document.getElementById("error-body");
const listEl = document.getElementById("suggestions");
const brandSub = document.getElementById("brand-sub");
const closeBtn = document.getElementById("close-btn");
const regenBtn = document.getElementById("regenerate-btn");
const retryBtn = document.getElementById("retry-btn");

let suggestions = [];
let activeIndex = 0;

function setMode(mode) {
  stateLoading.classList.toggle("hidden", mode !== "loading");
  stateError.classList.toggle("hidden", mode !== "error");
  listEl.classList.toggle("hidden", mode !== "list");
  brandSub.textContent = mode === "loading" ? "thinking…" : "";
}

function render() {
  listEl.innerHTML = "";
  suggestions.forEach((s, i) => {
    const li = document.createElement("li");
    li.className = "suggestion" + (i === activeIndex ? " active" : "");
    li.dataset.index = i;
    li.innerHTML = `
      <span class="num">${i + 1}</span>
      <span class="text"></span>
    `;
    li.querySelector(".text").textContent = s.text;
    li.addEventListener("mouseenter", () => {
      activeIndex = i;
      render();
    });
    li.addEventListener("click", () => accept(i));
    listEl.appendChild(li);
  });
}

async function accept(i) {
  if (i < 0 || i >= suggestions.length) return;
  try {
    await invoke("accept_suggestion", { index: i });
  } catch (e) {
    console.error("accept failed", e);
    brandSub.textContent = "paste failed";
  }
}

async function close() {
  try { await invoke("close_overlay"); } catch (_) {}
}

async function regenerate() {
  setMode("loading");
  try {
    await invoke("regenerate_suggestions");
  } catch (e) {
    setMode("error");
    errorBody.textContent = String(e);
  }
}

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    e.preventDefault();
    close();
  } else if (e.key === "ArrowDown") {
    e.preventDefault();
    if (suggestions.length) {
      activeIndex = (activeIndex + 1) % suggestions.length;
      render();
    }
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    if (suggestions.length) {
      activeIndex = (activeIndex - 1 + suggestions.length) % suggestions.length;
      render();
    }
  } else if (e.key === "Enter" && !(e.ctrlKey && e.shiftKey)) {
    // Plain Enter also accepts when overlay focused — UX nicety.
    e.preventDefault();
    accept(activeIndex);
  } else if (e.key >= "1" && e.key <= "6") {
    const i = parseInt(e.key, 10) - 1;
    if (i < suggestions.length) {
      activeIndex = i;
      render();
      accept(i);
    }
  }
});

closeBtn.addEventListener("click", close);
regenBtn.addEventListener("click", regenerate);
retryBtn.addEventListener("click", regenerate);

listen("suggestions", (event) => {
  suggestions = event.payload || [];
  activeIndex = 0;
  render();
  setMode("list");
});

listen("suggestion-error", (event) => {
  errorBody.textContent = String(event.payload || "Unknown error");
  setMode("error");
});

// Triggered when the action hotkey fires while overlay focused.
listen("accept-highlighted", () => {
  accept(activeIndex);
});

setMode("loading");

// Focus the document so keyboard nav works immediately.
window.addEventListener("load", () => {
  document.body.focus?.();
  getCurrentWindow().setFocus().catch(() => {});
});
