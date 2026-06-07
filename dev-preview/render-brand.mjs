// Renders the logo (PNG at 1024) and the banner (1280x640) from HTML templates.

import { chromium } from "playwright";
import { fileURLToPath } from "url";
import path from "path";
import fs from "fs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "..");
const BRAND_DIR = path.join(ROOT, "brand");
fs.mkdirSync(BRAND_DIR, { recursive: true });

const browser = await chromium.launch();

// ---------- Logo: render the 1024x1024 SVG to PNG ----------
{
  const ctx = await browser.newContext({
    viewport: { width: 1024, height: 1024 },
    deviceScaleFactor: 1,
  });
  const page = await ctx.newPage();
  const url = "file:///" + path.join(__dirname, "logo-source.html").replace(/\\/g, "/");
  await page.goto(url);
  await page.waitForTimeout(200);
  await page.screenshot({
    path: path.join(BRAND_DIR, "logo-1024.png"),
    omitBackground: true,
    clip: { x: 0, y: 0, width: 1024, height: 1024 },
  });
  console.log("wrote brand/logo-1024.png");
  await ctx.close();
}

// ---------- Banner: 1280x640 ----------
{
  const ctx = await browser.newContext({
    viewport: { width: 1280, height: 640 },
    deviceScaleFactor: 2,
  });
  const page = await ctx.newPage();
  const url = "file:///" + path.join(__dirname, "banner-source.html").replace(/\\/g, "/");
  await page.goto(url);
  await page.waitForTimeout(500);
  await page.screenshot({
    path: path.join(BRAND_DIR, "banner.png"),
    fullPage: false,
  });
  console.log("wrote brand/banner.png");
  await ctx.close();
}

await browser.close();
console.log("done");
