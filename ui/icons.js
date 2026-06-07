// Inline SVG icon library. Single source of truth — replaces Unicode glyphs.
// Each icon is a 16x16 stroke-based shape, currentColor.

const ICONS = {
  close: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <path d="M3.5 3.5l9 9M12.5 3.5l-9 9" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
  </svg>`,
  refresh: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <path d="M2.5 8a5.5 5.5 0 0 1 9.5-3.78V3M13.5 8a5.5 5.5 0 0 1-9.5 3.78V13M12 4.5h-2.5M4 11.5h2.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
  </svg>`,
  arrow_up: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <path d="M8 13V3M3.5 7.5L8 3l4.5 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
  </svg>`,
  arrow_down: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <path d="M8 3v10M12.5 8.5L8 13l-4.5-4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
  </svg>`,
  check: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <path d="M3 8.5L6.5 12 13 5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
  </svg>`,
  alert: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <path d="M8 5v4M8 11.5v.01" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
    <circle cx="8" cy="8" r="6.5" stroke="currentColor" stroke-width="1.5"/>
  </svg>`,
  info: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <circle cx="8" cy="8" r="6.5" stroke="currentColor" stroke-width="1.5"/>
    <path d="M8 7.5V11M8 5v.01" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
  </svg>`,
  spark: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <path d="M8 2l1.4 4.2L13.6 8l-4.2 1.4L8 13.6 6.6 9.4 2.4 8l4.2-1.4L8 2z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" fill="currentColor" fill-opacity="0.18"/>
  </svg>`,
  key: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <circle cx="5.5" cy="10.5" r="3" stroke="currentColor" stroke-width="1.5"/>
    <path d="M7.5 9l5-5M11 5.5l1.5 1.5M9.5 7l1.5 1.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
  </svg>`,
  keyboard: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <rect x="1.5" y="3.5" width="13" height="9" rx="1.5" stroke="currentColor" stroke-width="1.5"/>
    <path d="M4 6h.01M6.5 6h.01M9 6h.01M11.5 6h.01M4 8.5h.01M6.5 8.5h.01M9 8.5h.01M11.5 8.5h.01M5 10.5h6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
  </svg>`,
  sliders: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <path d="M3 5h10M3 11h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
    <circle cx="6" cy="5" r="1.5" stroke="currentColor" stroke-width="1.5" fill="var(--bg-canvas)"/>
    <circle cx="11" cy="11" r="1.5" stroke="currentColor" stroke-width="1.5" fill="var(--bg-canvas)"/>
  </svg>`,
  shield: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <path d="M8 1.5l5.5 2v4.5c0 3-2.5 5.5-5.5 6.5C5 13.5 2.5 11 2.5 8V3.5L8 1.5z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
  </svg>`,
  doc: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <path d="M3 1.5h6.5L13 5v9.5H3v-13z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
    <path d="M9.5 1.5V5H13" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
  </svg>`,
  rocket: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <path d="M8 1.5c2.2 1.4 3.3 3.6 3.3 5.8 0 1.4-.4 2.7-1 3.7H5.7c-.6-1-1-2.3-1-3.7 0-2.2 1.1-4.4 3.3-5.8z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
    <circle cx="8" cy="6.5" r="1.5" stroke="currentColor" stroke-width="1.5"/>
    <path d="M5.7 11l-1.5 1.5v2L6 13.5M10.3 11l1.5 1.5v2L10 13.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="M7 14.5l1 1 1-1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
  </svg>`,
  loader: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <path d="M8 1.5v3M8 11.5v3M14.5 8h-3M4.5 8h-3M12.6 3.4l-2.1 2.1M5.5 10.5l-2.1 2.1M12.6 12.6l-2.1-2.1M5.5 5.5L3.4 3.4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
  </svg>`,
  search: `<svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <circle cx="7" cy="7" r="4.5" stroke="currentColor" stroke-width="1.5"/>
    <path d="M10.5 10.5L14 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
  </svg>`,
  brand: `<svg viewBox="0 0 256 256" xmlns="http://www.w3.org/2000/svg" aria-hidden="true" style="display:block;width:100%;height:100%;">
    <defs>
      <linearGradient id="bm-bg" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="#FF8E5E"/>
        <stop offset="100%" stop-color="#D9461A"/>
      </linearGradient>
      <linearGradient id="bm-sh" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="rgba(255,255,255,0.22)"/>
        <stop offset="55%" stop-color="rgba(255,255,255,0)"/>
      </linearGradient>
    </defs>
    <rect x="8" y="8" width="240" height="240" rx="56" fill="url(#bm-bg)"/>
    <rect x="8" y="8" width="240" height="240" rx="56" fill="url(#bm-sh)"/>
    <rect x="8" y="8" width="240" height="240" rx="56" fill="none" stroke="rgba(255,255,255,0.18)" stroke-width="1.5"/>
    <path d="M 72 152 L 128 96 L 184 152" stroke="#fff" stroke-width="22" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="M 96 180 L 128 148 L 160 180" stroke="rgba(255,255,255,0.55)" stroke-width="16" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
    <circle cx="128" cy="76" r="9" fill="#fff"/>
    <circle cx="128" cy="76" r="14" fill="none" stroke="#fff" stroke-width="2" opacity="0.45"/>
  </svg>`,
};

window.icon = function (name) {
  return ICONS[name] || "";
};

window.injectIcons = function (root = document) {
  root.querySelectorAll("[data-icon]").forEach((el) => {
    const name = el.getAttribute("data-icon");
    if (ICONS[name]) {
      el.innerHTML = ICONS[name];
    }
  });
};

document.addEventListener("DOMContentLoaded", () => window.injectIcons());
