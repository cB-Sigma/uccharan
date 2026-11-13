// Live demo: everything runs in the page through the WebAssembly build.
import * as uccharan from "./pkg/index.js";

const $ = (id) => document.getElementById(id);
const els = { lang: $("lang"), profile: $("profile"), normalize: $("normalize"), hinglish: $("hinglish"), hinglishWrap: $("hinglish-wrap"),
  text: $("text"), out: $("out"), timing: $("timing"), copy: $("copy"), ids: $("ids"), idsWrap: $("ids-wrap"), note: $("mode-note"),
  examples: $("examples"), title: $("out-title") };

const EXAMPLES = {
  "en-us": [
    "Hello, world. This runs in your browser.",
    "I read the book yesterday, and I will read it again.",
    "Dr. Smith paid $12.50 on 3/14/2024 at 3:30pm.",
    "She plays bass in a band and caught a bass in the lake.",
    "Call 555-1234 about pages 10-15.",
  ],
  "en-gb": [
    "The car park is far from the water.",
    "They live close to a live wire.",
    "It cost £3.50 on the 5th of May.",
  ],
  hi: [
    "नमस्ते दुनिया, आप कैसे हैं?",
    "मैं office जा रहा हूँ और WhatsApp पर message करूँगा।",
    "लड़कपन की बातें और ₹2,50,000 का हिसाब।",
    "kya haal hai, main ghar ja raha hoon",
  ],
};
const NOTES = {
  compatible: "Compatible mode reproduces the phonemes existing espeak-trained voices learned, including their quirks.",
  improved: "Improved mode reads homographs, names, numbers and Hindi schwas more correctly. Existing voices were not trained on these forms.",
};

const mode = () => document.querySelector('input[name="mode"]:checked').value;

function options() {
  return { lang: els.lang.value, profile: els.profile.value, normalize: els.normalize.value, improved: mode() === "improved",
    hinglish: els.lang.value === "hi" && els.hinglish.checked };
}

function fmt(us) {
  if (us < 1000) return `${us.toFixed(us < 10 ? 1 : 0)} µs`;
  return `${(us / 1000).toFixed(2)} ms`;
}

let pending = 0;
async function render() {
  const ticket = ++pending;
  const opt = options();
  const text = els.text.value;
  els.hinglishWrap.hidden = opt.lang !== "hi";
  els.note.textContent = NOTES[mode()];
  try {
    const needed = uccharan.languagesFor(text || "a", opt.lang);
    const missing = needed.filter((l) => !uccharan.isLoaded(l));
    if (missing.length) {
      els.timing.textContent = `loading ${missing.join(", ")}…`;
      await Promise.all(missing.map((l) => uccharan.loadLanguage(l)));
      if (ticket !== pending) return;
    }
    // Time the call: repeat until at least 2 ms have passed so that the clock's resolution does not matter.
    let result;
    let runs = 0;
    const t0 = performance.now();
    do { result = uccharan.phonemizeSync(text, opt); runs++; } while (performance.now() - t0 < 2 && runs < 2000);
    const us = ((performance.now() - t0) * 1000) / runs;
    const words = text.trim() ? text.trim().split(/\s+/).length : 0;
    els.timing.textContent = words ? `${fmt(us)} · ${words} word${words === 1 ? "" : "s"}` : "ready";
    show(result, opt);
  } catch (err) {
    els.out.textContent = String(err && err.message ? err.message : err);
    els.timing.textContent = "error";
  }
}

function show(result, opt) {
  els.out.classList.toggle("small", opt.profile === "sherpa-onnx");
  els.out.replaceChildren();
  let ids = null;
  if (opt.profile === "piper") {
    els.title.textContent = "Piper sentences";
    for (const s of result) { const d = document.createElement("span"); d.className = "sent"; d.textContent = s; els.out.append(d); }
  } else if (opt.profile === "sherpa-onnx") {
    els.title.textContent = "Codepoints per sentence";
    for (const s of result) { const d = document.createElement("span"); d.className = "sent"; d.textContent = `[${s.join(", ")}]`; els.out.append(d); }
  } else {
    els.title.textContent = opt.profile === "kokoro" ? "Kokoro tokens" : "Phonemes";
    els.out.textContent = result;
    if (opt.profile === "kokoro") ids = uccharan.kokoroIds(result);
    if (opt.profile === "kitten") ids = uccharan.kittenIds(result);
    if (opt.profile === "matcha") ids = uccharan.matchaIds(result);
  }
  els.idsWrap.hidden = !ids;
  if (ids) els.ids.textContent = Array.from(ids).join(" ");
}

function fillExamples() {
  els.examples.replaceChildren();
  for (const ex of EXAMPLES[els.lang.value]) {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = ex;
    b.addEventListener("click", () => {
      els.text.value = ex;
      if (/^[a-z ,]+$/.test(ex) && els.lang.value === "hi") els.hinglish.checked = true;
      render();
    });
    els.examples.append(b);
  }
}

els.text.addEventListener("input", render);
for (const el of [els.profile, els.normalize, els.hinglish]) el.addEventListener("change", render);
for (const r of document.querySelectorAll('input[name="mode"]')) r.addEventListener("change", render);
els.lang.addEventListener("change", () => { fillExamples(); els.text.value = EXAMPLES[els.lang.value][0]; render(); });
els.copy.addEventListener("click", async () => {
  try { await navigator.clipboard.writeText(els.out.innerText); els.copy.textContent = "Copied"; } catch (e) { els.copy.textContent = "Select and copy"; }
  setTimeout(() => { els.copy.textContent = "Copy"; }, 1200);
});
$("theme").addEventListener("click", () => {
  const root = document.documentElement;
  const dark = root.dataset.theme ? root.dataset.theme === "dark" : matchMedia("(prefers-color-scheme: dark)").matches;
  root.dataset.theme = dark ? "light" : "dark";
  try { localStorage.setItem("uccharan-theme", root.dataset.theme); } catch (e) { /* private mode */ }
});
for (const tab of document.querySelectorAll(".tabs button")) {
  tab.addEventListener("click", () => {
    for (const t of document.querySelectorAll(".tabs button")) t.setAttribute("aria-selected", String(t === tab));
    for (const p of document.querySelectorAll("pre[data-pane]")) p.hidden = p.dataset.pane !== tab.dataset.tab;
  });
}

fillExamples();
els.text.value = EXAMPLES["en-us"][0];
uccharan.init().then(async () => {
  $("version").textContent = await uccharan.version();
  render();
}).catch((err) => { els.out.textContent = `Could not start the WebAssembly module: ${err.message}`; els.timing.textContent = "error"; });
