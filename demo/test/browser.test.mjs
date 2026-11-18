// Browser test of the demo in Chromium (Playwright), plus the README screenshots and GIF.
//   node bindings/wasm/build.mjs && node demo/build.mjs [--xlit DIR]
//   node demo/test/browser.test.mjs [--shots DIR] [--frames DIR] [--gif FILE]
// --frames writes the GIF's frames and their durations (frames.txt, an ffmpeg
// concat list); --gif also joins them into FILE with ffmpeg.
// Needs the `playwright` package and its Chromium; it is not part of CI.
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const { chromium } = require("playwright");
const here = dirname(fileURLToPath(import.meta.url));
const arg = (name) => { const i = process.argv.indexOf(name); return i > 0 ? process.argv[i + 1] : null; };
const shots = arg("--shots");
const gif = arg("--gif");
const frames = arg("--frames") ? resolve(arg("--frames")) : gif ? mkdtempSync(join(tmpdir(), "uccharan-frames-")) : null;
const port = 8137;
const W = 1180;
const H = 980;

const server = spawn(process.execPath, [join(here, "..", "serve.mjs"), String(port)], { stdio: ["ignore", "pipe", "inherit"] });
await new Promise((resolve) => server.stdout.once("data", resolve));
const browser = await chromium.launch();
const url = `http://127.0.0.1:${port}/`;
const nfd = (s) => s.normalize("NFD");
let checks = 0;
const ok = (cond, msg) => { assert.ok(cond, msg); checks++; };
const list = [];
const outHas = (page, re) => page.waitForFunction((src) => new RegExp(src).test(document.getElementById("out").textContent), re.source);

try {
  // ---- desktop, dark ----------------------------------------------------
  const page = await browser.newPage({ viewport: { width: W, height: H }, deviceScaleFactor: 2, colorScheme: "dark" });
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });

  // Frames for the GIF: each one is shown for `ms`.
  const snap = async (ms) => {
    if (!frames) return;
    await page.waitForTimeout(60);
    const name = `f${String(list.length).padStart(3, "0")}.png`;
    await page.screenshot({ path: join(frames, name), clip: { x: 0, y: 0, width: W, height: H } });
    list.push([name, ms]);
  };
  // Type text into the box a few characters at a time, recording as it goes.
  const typeIn = async (text, step = 5) => {
    await page.fill("#text", "");
    const chars = Array.from(text);
    for (let i = 0; i < chars.length; i += step) {
      await page.type("#text", chars.slice(i, i + step).join(""), { delay: frames ? 15 : 0 });
      if (i + step < chars.length) await snap(110);
    }
  };
  const task = (t) => page.locator(`input[name="task"][value="${t}"]`).check({ force: true });
  const mode = (m) => page.locator(`input[name="mode"][value="${m}"]`).check({ force: true });
  if (frames) mkdirSync(frames, { recursive: true });
  if (shots) mkdirSync(shots, { recursive: true });

  await page.goto(url);
  const out = page.locator("#out");
  await page.waitForFunction(() => /µs|ms/.test(document.getElementById("timing").textContent));
  ok((await out.textContent()).startsWith("həlˈoʊ, wˈɜːld."), "default text is phonemized on load");
  ok(/^\d+\.\d+\.\d+$/.test(await page.locator("#version").textContent()), "version is shown");
  ok(/(µs|ms) · \d+ words/.test(await page.locator("#timing").textContent()), "timing is shown");
  ok((await page.locator("#lang option").count()) === 41, "every language tag is offered");
  ok(/44 languages/.test(await page.locator("h1").textContent()), "headline names the languages");
  await snap(1600);

  // English: live update while typing, then compatible / improved
  await typeIn("I read it yesterday.");
  await outHas(page, /jˈɛstɚdˌeɪ/);
  ok((await out.textContent()).includes("ɹˈiːd"), "compatible mode keeps the reference reading");
  await snap(1300);
  await mode("improved");
  await outHas(page, /ɹˈɛd/);
  ok(true, "improved mode reads the past tense");
  await snap(1500);
  if (shots) await page.screenshot({ path: join(shots, "demo-dark.png"), clip: { x: 0, y: 0, width: W, height: H } });

  // output profiles
  await page.selectOption("#profile", "piper");
  await page.fill("#text", "Hi there. Bye!");
  await page.waitForFunction(() => document.querySelectorAll("#out .sent").length === 2);
  ok(true, "Piper profile shows one line per sentence");
  await page.selectOption("#profile", "sherpa-onnx");
  await outHas(page, /^\[104,/);
  ok(true, "sherpa-onnx profile shows codepoints");
  await page.fill("#text", "I read it yesterday.");
  await page.selectOption("#profile", "kokoro");
  await page.waitForFunction(() => !document.getElementById("ids-wrap").hidden);
  ok((await page.locator("#ids").textContent()).startsWith("0 "), "Kokoro ids are padded with 0");
  await snap(1500);
  await page.selectOption("#profile", "espeak-ipa");
  await mode("compatible");

  // Hindi, code mixing
  await page.selectOption("#lang", "hi");
  await page.waitForFunction(() => document.getElementById("out").textContent.length > 5 && !document.getElementById("hinglish-wrap").hidden);
  ok(nfd(await out.textContent()).startsWith(nfd("nəmˈʌsteː")), "Hindi data is loaded on demand");
  await typeIn("मैं office जा रहा हूँ।", 4);
  await outHas(page, /ɟˈaː/);
  await snap(1500);
  await page.fill("#text", "kya haal hai");
  await page.locator("#hinglish").check();
  await outHas(page, /hˈaːl/);
  ok(true, "romanised Hindi option works");
  await page.locator("#hinglish").uncheck();

  // romanised Hindi into Devanagari
  await task("native");
  await typeIn("main kal Dilli ja raha hoon");
  await outHas(page, /जा रहा/);
  ok(/[ऀ-ॿ]/.test(await out.textContent()), "romanised Hindi is written in Devanagari");
  ok(await page.locator("#profile").isDisabled(), "the output profile is off outside phonemes");
  await snap(2000);

  // French with a number: spoken text, then phonemes; Belgian French
  await page.selectOption("#lang", "fr");
  ok(await page.locator('input[name="task"][value="native"]').isDisabled(), "no transliteration for French");
  ok((await page.locator('input[name="task"]:checked').getAttribute("value")) === "phonemes", "task falls back to phonemes");
  await task("spoken");
  await typeIn("Il a payé 92 € le 14 juillet.");
  await outHas(page, /quatre-vingt-douze/);
  ok(true, "French numbers are read out");
  await snap(1800);
  await task("phonemes");
  await outHas(page, /katʁəvɛ̃dˈuz/);
  await snap(1400);
  await task("spoken");
  await page.selectOption("#lang", "fr-be");
  await page.fill("#text", "Il a payé 92 € le 14 juillet.");
  await outHas(page, /nonante-deux/);
  ok(true, "Belgian French says nonante");
  await snap(1800);

  // German phonemes
  await page.selectOption("#lang", "de");
  await outHas(page, /zweitausendneunzehn|zwei/);
  await task("phonemes");
  await outHas(page, /bɛɾlˈiːn|bɛʁlˈiːn/);
  ok(true, "German is phonemised");

  // numbers
  await page.selectOption("#lang", "en-us");
  await page.fill("#text", "It costs $5.50.");
  await outHas(page, /sˈɛnts/);
  await page.selectOption("#normalize", "espeak");
  await outHas(page, /zˈiəɹoʊ/);
  ok(true, "number reading can follow the reference");
  await page.selectOption("#normalize", "natural");

  // theme toggle and persistence
  await page.click("#theme");
  ok((await page.evaluate(() => document.documentElement.dataset.theme)) === "light", "theme toggles");
  const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  ok(bg === "rgb(246, 246, 244)", `light background applied (${bg})`);
  await page.fill("#text", "Dr. Smith paid $12.50 on 3/14/2024 at 3:30pm.");
  await outHas(page, /dˈɑːktɚ/);
  await mode("improved");
  if (shots) await page.screenshot({ path: join(shots, "demo-light.png"), clip: { x: 0, y: 0, width: W, height: H } });
  await page.reload();
  await page.waitForFunction(() => /µs|ms/.test(document.getElementById("timing").textContent));
  ok((await page.evaluate(() => document.documentElement.dataset.theme)) === "light", "theme choice persists");
  ok(errors.length === 0, `no console errors: ${errors.join(" | ")}`);
  await page.close();

  // ---- phone ---------------------------------------------------------------
  const phone = await browser.newPage({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, colorScheme: "dark" });
  await phone.goto(url);
  await phone.waitForFunction(() => /µs|ms/.test(document.getElementById("timing").textContent));
  const overflow = await phone.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  ok(overflow <= 0, `no horizontal scrolling on a phone (overflow ${overflow}px)`);
  await phone.selectOption("#lang", "hi");
  await phone.locator('input[name="task"][value="native"]').check({ force: true });
  await phone.waitForFunction(() => /[ऀ-ॿ]/.test(document.getElementById("out").textContent));
  const overflow2 = await phone.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  ok(overflow2 <= 0, "no horizontal scrolling with Hindi controls");
  if (shots) await phone.screenshot({ path: join(shots, "demo-phone.png") });
  await phone.close();
  console.log(`ok: ${checks} checks passed`);
} finally {
  await browser.close();
  server.kill();
}

if (frames) {
  // ffmpeg's concat demuxer needs the last file repeated to honour its duration.
  const lines = list.flatMap(([f, ms]) => [`file '${f}'`, `duration ${(ms / 1000).toFixed(3)}`]);
  lines.push(`file '${list[list.length - 1][0]}'`);
  writeFileSync(join(frames, "frames.txt"), lines.join("\n") + "\n");
  console.log(`${list.length} frames in ${frames}`);
}
if (gif) {
  const r = spawnSync("ffmpeg", ["-y", "-v", "error", "-f", "concat", "-safe", "0", "-i", join(frames, "frames.txt"),
    "-vf", "scale=900:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=96:stats_mode=diff[p];[b][p]paletteuse=dither=none:diff_mode=rectangle",
    "-loop", "0", resolve(gif)], { stdio: "inherit" });
  if (r.status !== 0) throw new Error("ffmpeg failed");
  if (!arg("--frames")) rmSync(frames, { recursive: true, force: true });
  console.log(`wrote ${gif}`);
}
