// WebAssembly speed of the npm package in Node.
//
//   node bindings/wasm/build.mjs
//   node tools/bench/wasm_bench.mjs [--limit N] [--out FILE]
//
// Phonemizes the held-out sentences one call per sentence and reports start-up,
// latency and throughput, plus the size of what a browser downloads.
import { readFileSync, statSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { performance } from "node:perf_hooks";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const arg = (name) => { const i = process.argv.indexOf(name); return i > 0 ? process.argv[i + 1] : null; };
const limit = Number(arg("--limit") || 1000);

function sentences(file) {
  return readFileSync(join(root, "tools/eval/data", file), "utf8").split("\n").filter((l) => l.includes("\t")).map((l) => l.split("\t")[1]).slice(0, limit);
}
const pct = (xs, q) => [...xs].sort((a, b) => a - b)[Math.min(xs.length - 1, Math.floor(q * xs.length))];

const pkg = join(root, "bindings", "wasm");
const size = (p) => statSync(p).size;
const result = {
  node: process.version,
  size_bytes: {
    wasm: size(join(pkg, "dist/uccharan_wasm_bg.wasm")),
    glue_js: size(join(pkg, "dist/uccharan_wasm.js")) + size(join(pkg, "index.js")),
    data_en_us: size(join(pkg, "data/en-us.ucd")),
    data_en_gb: size(join(pkg, "data/en-gb.ucd")),
    data_hi: size(join(pkg, "data/hi.ucd")),
  },
  languages: {},
};

const t00 = performance.now();
const u = await import(pathToFileURL(join(pkg, "index.js")).href);
await u.init();
result.module_init_ms = performance.now() - t00;
for (const [lang, file] of [["en-us", "en_heldout.tsv"], ["en-gb", "en_heldout.tsv"], ["hi", "hi_heldout.tsv"]]) {
  const texts = sentences(file);
  const opt = { lang, normalize: "espeak", punctuation: false };
  const t0 = performance.now();
  await u.loadLanguage(lang);
  u.phonemizeSync(texts[0], opt);
  const load = performance.now() - t0;
  for (const t of texts.slice(0, 100)) u.phonemizeSync(t, opt);
  let best = null;
  for (let r = 0; r < 5; r++) {
    const lat = [];
    for (const t of texts) { const a = performance.now(); u.phonemizeSync(t, opt); lat.push(performance.now() - a); }
    const total = lat.reduce((x, y) => x + y, 0);
    if (!best || total < best.total) best = { total, lat };
  }
  const words = texts.reduce((n, t) => n + t.trim().split(/\s+/).length, 0);
  const res = { sentences: texts.length, load_data_plus_first_call_ms: load, latency_us_p50: 1000 * pct(best.lat, 0.5), latency_us_p99: 1000 * pct(best.lat, 0.99), words_per_second: words / (best.total / 1000) };
  console.log(`[${lang}] load+first ${load.toFixed(1)} ms  p50 ${res.latency_us_p50.toFixed(1)} µs  p99 ${res.latency_us_p99.toFixed(1)} µs  ${Math.round(res.words_per_second)} words/s`);
  result.languages[lang] = res;
}
console.log(`module init ${result.module_init_ms.toFixed(1)} ms; wasm ${(result.size_bytes.wasm / 1024).toFixed(0)} KiB`);
const out = arg("--out");
if (out) { mkdirSync(dirname(out), { recursive: true }); writeFileSync(out, JSON.stringify(result, null, 1) + "\n"); console.log("wrote", out); }
