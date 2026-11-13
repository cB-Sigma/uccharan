// Rasterise the brand pages written by build.py with Chromium (Playwright).
//   node tools/brand/render.mjs
import { createRequire } from "node:module";
import { fileURLToPath, pathToFileURL } from "node:url";
import path from "node:path";

const require = createRequire(import.meta.url);
const { chromium } = require("playwright");
const here = path.dirname(fileURLToPath(import.meta.url));
const work = path.join(here, ".work");
const out = path.join(here, "..", "..", "docs", "assets", "brand");

const browser = await chromium.launch();
async function shot(file, target, width, height, scale, selector) {
  const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: scale });
  await page.goto(pathToFileURL(path.join(work, file)).href);
  await page.waitForTimeout(250);
  const el = selector ? page.locator(selector) : page;
  await el.screenshot({ path: target, omitBackground: true, ...(selector ? {} : { clip: { x: 0, y: 0, width, height } }) });
  await page.close();
  console.log(path.relative(process.cwd(), target));
}
async function icon(svg, target, size) {
  const page = await browser.newPage({ viewport: { width: size, height: size }, deviceScaleFactor: 1 });
  await page.setContent(`<html><body style="margin:0;background:transparent"><img src="${pathToFileURL(path.join(work, svg)).href}" width="${size}" height="${size}" style="display:block"></body></html>`);
  await page.waitForTimeout(150);
  await page.screenshot({ path: target, omitBackground: true, clip: { x: 0, y: 0, width: size, height: size } });
  await page.close();
  console.log(path.relative(process.cwd(), target));
}
await shot("banner-dark.html", path.join(out, "banner-dark.png"), 2000, 600, 1.6);
await shot("banner-light.html", path.join(out, "banner-light.png"), 2000, 600, 1.6);
await shot("social-preview.html", path.join(out, "social-preview.png"), 1280, 640, 1);
await shot("button.html", path.join(out, "button-live-demo.png"), 400, 80, 2, "#b");
await shot("sheet.html", path.join(work, "sheet.png"), 1100, 1000, 1);
for (const s of [16, 32, 48]) await icon(s < 20 ? "icon-small.svg" : "icon-mark.svg", path.join(out, "favicon", `favicon-${s}.png`), s);
await icon("icon-full.svg", path.join(out, "favicon", "apple-touch-icon.png"), 180);
await icon("icon-full.svg", path.join(out, "favicon", "icon-192.png"), 192);
await icon("icon-full.svg", path.join(out, "favicon", "icon-512.png"), 512);
await browser.close();
