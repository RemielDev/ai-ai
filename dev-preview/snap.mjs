// Capture mockup screenshots of every UI surface.
// Run from project root: node dev-preview/snap.mjs

import { chromium } from "playwright";
import { fileURLToPath } from "url";
import path from "path";
import fs from "fs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_DIR = path.resolve(__dirname, "..", "ui");
const OUT_DIR = path.resolve(__dirname, "..", "mockups");
fs.mkdirSync(OUT_DIR, { recursive: true });

// Inject the Tauri stub before any page script runs.
const stubSrc = fs.readFileSync(path.join(__dirname, "stub.js"), "utf8");

const shots = [
  // ---------- Overlay ----------
  {
    file: "overlay.html",
    name: "overlay-loading",
    viewport: { width: 600, height: 400 },
    setup: async (page) => {
      // default state — loading
    },
  },
  {
    file: "overlay.html",
    name: "overlay-suggestions",
    viewport: { width: 600, height: 400 },
    setup: async (page) => {
      await page.waitForTimeout(200);
      await page.evaluate(() => {
        window.__STUB_EMIT__("steer", "more technical, focus on edge cases");
        window.__STUB_EMIT__("suggestions", [
          { text: "Can you walk me through how this handles concurrent writes under high load?" },
          { text: "What are the specific edge cases this design doesn't cover yet?" },
          { text: "Show me a worked example with realistic input data, not the happy path." },
          { text: "How would this approach break if I scaled the user count by 100x?" },
        ]);
      });
      await page.waitForTimeout(500);
    },
  },
  {
    file: "overlay.html",
    name: "overlay-error",
    viewport: { width: 600, height: 400 },
    setup: async (page) => {
      await page.waitForTimeout(200);
      await page.evaluate(() => {
        window.__STUB_EMIT__("suggestion-error", {
          title: "Rate limited",
          body: "Anthropic rate limit hit. Wait a moment and retry.",
        });
      });
      await page.waitForTimeout(300);
    },
  },
  {
    file: "overlay.html",
    name: "overlay-empty",
    viewport: { width: 600, height: 400 },
    setup: async (page) => {
      await page.waitForTimeout(200);
      await page.evaluate(() => {
        window.__STUB_EMIT__("suggestions", []);
      });
      await page.waitForTimeout(300);
    },
  },

  // ---------- Settings ----------
  {
    file: "settings.html",
    name: "settings-setup",
    viewport: { width: 760, height: 1100 },
    setup: async (page) => { await page.waitForTimeout(500); },
  },
  {
    file: "settings.html",
    name: "settings-behavior",
    viewport: { width: 760, height: 1100 },
    setup: async (page) => {
      await page.waitForTimeout(400);
      await page.click("#tab-behavior");
      await page.waitForTimeout(300);
    },
  },
  {
    file: "settings.html",
    name: "settings-privacy",
    viewport: { width: 760, height: 1100 },
    setup: async (page) => {
      await page.waitForTimeout(400);
      await page.click("#tab-privacy");
      await page.waitForTimeout(300);
    },
  },
  {
    file: "settings.html",
    name: "settings-about",
    viewport: { width: 760, height: 1100 },
    setup: async (page) => {
      await page.waitForTimeout(400);
      await page.click("#tab-about");
      await page.waitForTimeout(300);
    },
  },

  // ---------- Onboarding ----------
  {
    file: "onboarding.html",
    name: "onboarding-step1",
    viewport: { width: 560, height: 640 },
    setup: async (page) => { await page.waitForTimeout(400); },
  },
  {
    file: "onboarding.html",
    name: "onboarding-step2",
    viewport: { width: 560, height: 640 },
    setup: async (page) => {
      await page.waitForTimeout(300);
      await page.click('[data-go="2"]');
      await page.waitForTimeout(400);
    },
  },
  {
    file: "onboarding.html",
    name: "onboarding-step3",
    viewport: { width: 560, height: 640 },
    setup: async (page) => {
      await page.waitForTimeout(300);
      await page.click('[data-go="2"]');
      await page.waitForTimeout(300);
      await page.evaluate(() => {
        document.querySelectorAll(".wizard-step").forEach((el) => {
          el.classList.toggle("is-visible", el.dataset.step === "3");
        });
        document.querySelectorAll(".wizard-dot").forEach((d) => {
          const n = parseInt(d.dataset.step, 10);
          d.classList.toggle("is-active", n === 3);
          d.classList.toggle("is-done", n < 3);
        });
      });
      await page.waitForTimeout(400);
    },
  },

  // ---------- About ----------
  {
    file: "about.html",
    name: "about",
    viewport: { width: 460, height: 480 },
    setup: async (page) => { await page.waitForTimeout(400); },
  },
];

const browser = await chromium.launch();
for (const shot of shots) {
  const ctx = await browser.newContext({ viewport: shot.viewport, deviceScaleFactor: 2 });
  // Inject stub before any page script.
  await ctx.addInitScript({ content: stubSrc });
  const page = await ctx.newPage();
  page.on("console", (msg) => {
    if (msg.type() === "error") console.error(`[${shot.name}] ${msg.text()}`);
  });
  const url = "file:///" + path.join(UI_DIR, shot.file).replace(/\\/g, "/");
  await page.goto(url);
  if (shot.setup) await shot.setup(page);
  const outPath = path.join(OUT_DIR, `${shot.name}.png`);
  await page.screenshot({ path: outPath, omitBackground: false });
  console.log("wrote", outPath);
  await ctx.close();
}
await browser.close();
console.log("done");
