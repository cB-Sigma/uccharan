// Browser test of the demo in Chromium (Playwright), plus the README screenshots.
//   node bindings/wasm/build.mjs && node demo/build.mjs
//   node demo/test/browser.test.mjs [--shots DIR] [--frames DIR]
// Needs the `playwright` package and its Chromium; it is not part of CI.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import { mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const { chromium } = require("playwright");
const here = dirname(fileURLToPath(import.meta.url));
const arg = (name) => { const i = process.argv.indexOf(name); return i > 0 ? process.argv[i + 1] : null; };
const shots = arg("--shots");
const frames = arg("--frames");
const port = 8137;

const server = spawn(process.execPath, [join(here, "..", "serve.mjs"), String(port)], { stdio: ["ignore", "pipe", "inherit"] });
await new Promise((resolve) => server.stdout.once("data", resolve));
const browser = await chromium.launch();
const url = `http://127.0.0.1:${port}/`;
const nfd = (s) => s.normalize("NFD");
let checks = 0;
const ok = (cond, msg) => { assert.ok(cond, msg); checks++; };

try {
  // ---- desktop, dark ----------------------------------------------------
  const page = await browser.newPage({ viewport: { width: 1180, height: 860 }, deviceScaleFactor: 2, colorScheme: "dark" });
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
  await page.goto(url);
  const out = page.locator("#out");
  await page.waitForFunction(() => /µs|ms/.test(document.getElementById("timing").textContent));
  ok((await out.textContent()).startsWith("həlˈoʊ, wˈɜːld."), "default text is phonemized on load");
  ok(/^\d+\.\d+\.\d+$/.test(await page.locator("#version").textContent()), "version is shown");
  ok(/(µs|ms) · \d+ words/.test(await page.locator("#timing").textContent()), "timing is shown");

  // live update while typing
  await page.fill("#text", "");
  await page.type("#text", "I read it yesterday.", { delay: frames ? 45 : 0 });
  await page.waitForFunction(() => document.getElementById("out").textContent.includes("jˈɛstɚdˌeɪ"));
  ok((await out.textContent()).includes("ɹˈiːd"), "compatible mode keeps the reference reading");
  if (shots) { mkdirSync(shots, { recursive: true }); }
  let frame = 0;
  const snap = async () => { if (frames) { mkdirSync(frames, { recursive: true }); await page.screenshot({ path: join(frames, `f${String(frame++).padStart(3, "0")}.png`), clip: { x: 0, y: 0, width: 1180, height: 760 } }); } };
  await snap();
  await page.locator('input[name="mode"][value="improved"]').check({ force: true });
  await page.waitForFunction(() => document.getElementById("out").textContent.includes("ɹˈɛd"));
  ok(true, "improved mode reads the past tense");
  await snap();
  if (shots) await page.screenshot({ path: join(shots, "demo-dark.png"), clip: { x: 0, y: 0, width: 1180, height: 760 } });

  // profiles
  await page.selectOption("#profile", "piper");
  await page.fill("#text", "Hi there. Bye!");
  await page.waitForFunction(() => document.querySelectorAll("#out .sent").length === 2);
  ok(true, "Piper profile shows one line per sentence");
  await snap();
  await page.selectOption("#profile", "sherpa-onnx");
  await page.waitForFunction(() => document.getElementById("out").textContent.startsWith("[104,"));
  ok(true, "sherpa-onnx profile shows codepoints");
  await page.selectOption("#profile", "kokoro");
  await page.waitForFunction(() => !document.getElementById("ids-wrap").hidden);
  ok((await page.locator("#ids").textContent()).startsWith("0 "), "Kokoro ids are padded with 0");
  await snap();
  await page.selectOption("#profile", "espeak-ipa");

  // Hindi, code mixing, romanised Hindi
  await page.selectOption("#lang", "hi");
  await page.waitForFunction(() => document.getElementById("out").textContent.length > 5 && !document.getElementById("hinglish-wrap").hidden);
  ok(nfd(await out.textContent()).startsWith(nfd("nəmˈʌsteː")), "Hindi data is loaded on demand");
  await snap();
  await page.fill("#text", "मैं office जा रहा हूँ।");
  await page.waitForFunction(() => document.getElementById("out").textContent.includes("ɟˈaː"));
  await snap();
  await page.fill("#text", "kya haal hai");
  await page.locator("#hinglish").check();
  await page.waitForFunction(() => document.getElementById("out").textContent.includes("hˈaːl"));
  ok(true, "romanised Hindi option works");
  await snap();
  await page.selectOption("#lang", "en-gb");
  await page.waitForFunction(() => document.getElementById("out").textContent.includes("kˈɑː pˈɑːk"));
  ok(true, "British English works");
  await snap();

  // numbers
  await page.selectOption("#lang", "en-us");
  await page.fill("#text", "It costs $5.50.");
  await page.waitForFunction(() => document.getElementById("out").textContent.includes("sˈɛnts"));
  await page.selectOption("#normalize", "espeak");
  await page.waitForFunction(() => document.getElementById("out").textContent.includes("zˈiəɹoʊ"));
  ok(true, "number reading can follow the reference");
  await page.selectOption("#normalize", "natural");

  // theme toggle and persistence
  await page.click("#theme");
  ok((await page.evaluate(() => document.documentElement.dataset.theme)) === "light", "theme toggles");
  const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  ok(bg === "rgb(246, 246, 244)", `light background applied (${bg})`);
  await page.fill("#text", "Dr. Smith paid $12.50 on 3/14/2024 at 3:30pm.");
  await page.waitForFunction(() => document.getElementById("out").textContent.includes("dˈɑːktɚ"));
  if (shots) await page.screenshot({ path: join(shots, "demo-light.png"), clip: { x: 0, y: 0, width: 1180, height: 760 } });
  await snap();
  await page.reload();
  await page.waitForFunction(() => /µs|ms/.test(document.getElementById("timing").textContent));
  ok((await page.evaluate(() => document.documentElement.dataset.theme)) === "light", "theme choice persists");
  ok(errors.length === 0, `no console errors: ${errors.join(" | ")}`);
  await page.close();

  // ---- phone ---------------------------------------------------------------
  const phone = await browser.newPage({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 3, isMobile: true, hasTouch: true, colorScheme: "dark" });
  await phone.goto(url);
  await phone.waitForFunction(() => /µs|ms/.test(document.getElementById("timing").textContent));
  const overflow = await phone.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  ok(overflow <= 0, `no horizontal scrolling on a phone (overflow ${overflow}px)`);
  await phone.selectOption("#lang", "hi");
  await phone.waitForFunction(() => document.getElementById("out").textContent.length > 5 && !document.getElementById("hinglish-wrap").hidden);
  const overflow2 = await phone.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  ok(overflow2 <= 0, "no horizontal scrolling with Hindi controls");
  if (shots) await phone.screenshot({ path: join(shots, "demo-phone.png") });
  await phone.close();
  console.log(`ok: ${checks} checks passed`);
} finally {
  await browser.close();
  server.kill();
}
