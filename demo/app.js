// Live demo: everything runs in the page through the WebAssembly build.
import * as uccharan from "./pkg/index.js";

const $ = (id) => document.getElementById(id);
const els = { lang: $("lang"), profile: $("profile"), normalize: $("normalize"), hinglish: $("hinglish"), hinglishWrap: $("hinglish-wrap"),
  text: $("text"), out: $("out"), timing: $("timing"), copy: $("copy"), ids: $("ids"), idsWrap: $("ids-wrap"), note: $("mode-note"),
  examples: $("examples"), title: $("out-title"), profileWrap: $("profile-wrap"), normalizeWrap: $("normalize-wrap"), modeWrap: $("mode-wrap"),
  taskNative: $("task-native"), xlitNote: $("xlit-note") };

// [tag, name, examples, romanised examples (languages with a transliterator)]
const GROUPS = [
  ["English and Hindi", [
    ["en-us", "English (US)", [
      "Hello, world. This runs in your browser.",
      "I read the book yesterday, and I will read it again.",
      "Dr. Smith paid $12.50 on 3/14/2024 at 3:30pm.",
      "She plays bass in a band and caught a bass in the lake.",
    ]],
    ["en-gb", "English (UK, RP)", ["The car park is far from the water.", "They live close to a live wire.", "It cost £3.50 on the 5th of May."]],
    ["hi", "Hindi · हिन्दी", [
      "नमस्ते दुनिया, आप कैसे हैं?",
      "मैं office जा रहा हूँ और WhatsApp पर message करूँगा।",
      "लड़कपन की बातें और ₹2,50,000 का हिसाब।",
      "ट्रेन 10:30 बजे आएगी।",
    ], ["aap kaise ho", "main kal Dilli ja raha hoon", "mujhe kal office jana hai"]],
  ]],
  ["South Asian", [
    ["bn", "Bengali · বাংলা", ["আমি ভাত খাই। আমার বয়স ২৫ বছর।"], ["ami tomake bhalobashi", "tumi kemon acho"]],
    ["mr", "Marathi · मराठी", ["मी उद्या मुंबईला जाणार आहे."], ["mi udya Mumbaila janar aahe"]],
    ["te", "Telugu · తెలుగు", ["నమస్కారం, మీరు ఎలా ఉన్నారు?"], ["meeru ela unnaru"]],
    ["ta", "Tamil · தமிழ்", ["வணக்கம், நீங்கள் எப்படி இருக்கிறீர்கள்?"], ["vanakkam, eppadi irukkeenga"]],
    ["gu", "Gujarati · ગુજરાતી", ["નમસ્તે, તમે કેમ છો?"], ["tame kem cho"]],
    ["kn", "Kannada · ಕನ್ನಡ", ["ನಮಸ್ಕಾರ, ನೀವು ಹೇಗಿದ್ದೀರಿ?"], ["neevu hegiddeera"]],
    ["ml", "Malayalam · മലയാളം", ["നമസ്കാരം, സുഖമാണോ?"], ["namaskaram, sukhamano"]],
    ["pa", "Punjabi · ਪੰਜਾਬੀ", ["ਸਤ ਸ੍ਰੀ ਅਕਾਲ, ਤੁਸੀਂ ਕਿਵੇਂ ਹੋ?"], ["tussi kiven ho"]],
    ["or", "Odia · ଓଡ଼ିଆ", ["ନମସ୍କାର, ଆପଣ କେମିତି ଅଛନ୍ତି?"]],
    ["as", "Assamese · অসমীয়া", ["মই অসমীয়া ভাষা কওঁ।"]],
    ["ne", "Nepali · नेपाली", ["म नेपाली बोल्छु।"]],
    ["si", "Sinhala · සිංහල", ["ආයුබෝවන්, ඔබට කොහොමද?"]],
    ["ur", "Urdu · اردو", ["آپ کیسے ہیں؟"]],
    ["sd", "Sindhi · سنڌي", ["توهان ڪيئن آهيو؟"]],
    ["kok", "Konkani · कोंकणी", ["तुमी कशें आसात?"]],
    ["sa", "Sanskrit · संस्कृतम्", ["सर्वे भवन्तु सुखिनः।"]],
    ["bho", "Bhojpuri · भोजपुरी", ["रउआ कइसन बानी?"]],
    ["mai", "Maithili · मैथिली", ["अहाँ केहन छी?"]],
    ["mag", "Magahi · मगही", ["तू कइसन हऽ?"]],
    ["doi", "Dogri · डोगरी", ["तुस कि’यां ओ? एह् मेरा घर ऐ।"], ["tusen kiyan o"]],
    ["brx", "Bodo · बर’", ["आं बर’ राव बुङो। नों माबोरै दं?"], ["ang boro raw bungw"]],
    ["sat", "Santali · ᱥᱟᱱᱛᱟᱲᱤ", ["ᱡᱚᱦᱟᱨ! ᱥᱟᱱᱛᱟᱲᱤ ᱯᱟᱹᱨᱥᱤ ᱫᱚ ᱚᱞ ᱪᱤᱠᱤ ᱛᱮ ᱚᱞᱚᱜᱼᱟ ᱾"], ["johar inj disom do santali kana"]],
    ["mni", "Manipuri · ꯃꯤꯇꯩꯂꯣꯟ", ["ꯃꯤꯇꯩꯂꯣꯟ ꯑꯁꯤ ꯃꯅꯤꯄꯨꯔꯒꯤ ꯂꯣꯟꯅꯤ꯫", "মৈতৈলোন অসি মণিপুরগী লোননি।"], ["eina nangbu nungsi"]],
  ]],
  ["European", [
    ["es", "Spanish · Español", ["Hola, ¿cómo estás? Tengo 21 años."]],
    ["de", "German · Deutsch", ["Ich wohne seit 2019 in Berlin.", "Guten Morgen, wie geht es dir?"]],
    ["fr", "French · Français", ["Il a payé 92 € le 14 juillet.", "Bonjour, comment allez-vous ?"]],
    ["fr-be", "French (Belgium)", ["Il y avait 75 personnes et 92 chaises."]],
    ["fr-ch", "French (Switzerland)", ["Il y avait 75 personnes et 92 chaises."]],
    ["it", "Italian · Italiano", ["Ho comprato 3 libri per 45 euro."]],
    ["pt-br", "Portuguese (Brazil)", ["Eu tenho 32 anos e moro em São Paulo."]],
    ["pt", "Portuguese (Portugal)", ["O comboio parte às 8 horas."]],
    ["nl", "Dutch · Nederlands", ["Ik woon sinds 2019 in Amsterdam."]],
    ["pl", "Polish · Polski", ["Mam 2 koty i 1 psa."]],
    ["cs", "Czech · Čeština", ["Dobrý den, jak se máte?"]],
    ["ro", "Romanian · Română", ["Bună ziua, ce mai faceți?"]],
    ["hu", "Hungarian · Magyar", ["Jó napot, hogy van?"]],
    ["tr", "Turkish · Türkçe", ["Bugün hava çok güzel, 25 derece."]],
    ["sv", "Swedish · Svenska", ["Jag har bott i Stockholm i 12 år."]],
    ["nb", "Norwegian Bokmål · Norsk", ["Jeg har bodd i Oslo i 10 år."]],
    ["da", "Danish · Dansk", ["Jeg har boet i København i 7 år."]],
    ["fi", "Finnish · Suomi", ["Hyvää huomenta, mitä kuuluu?"]],
    ["ca", "Catalan · Català", ["Bon dia, com estàs?"]],
  ]],
];
const LANGS = new Map();
for (const [group, langs] of GROUPS) {
  const og = document.createElement("optgroup");
  og.label = group;
  for (const [tag, name, examples, romanised] of langs) {
    LANGS.set(tag, { examples, romanised });
    const o = document.createElement("option");
    o.value = tag;
    o.textContent = name;
    og.append(o);
  }
  els.lang.append(og);
}

const NOTES = {
  compatible: "Compatible mode reproduces the phonemes existing espeak-trained voices learned, including their quirks.",
  improved: "Improved mode reads homographs, names, numbers and Hindi schwas more correctly. Existing voices were not trained on these forms.",
  spoken: "The spoken form of the text, for voices that read plain text: numbers, money, dates and times written out as words.",
  native: "Romanised words are written in the native script; English words are kept as they are.",
  rules: "The transliteration pack is not on this server, so a plain rule converter answers.",
};

const mode = () => document.querySelector('input[name="mode"]:checked').value;
const task = () => document.querySelector('input[name="task"]:checked').value;
const canTransliterate = (lang) => Boolean(LANGS.get(lang).romanised);
const examples = () => {
  const l = LANGS.get(els.lang.value);
  return task() === "native" ? l.romanised : l.examples;
};

function options() {
  return { lang: els.lang.value, profile: els.profile.value, normalize: els.normalize.value, improved: mode() === "improved",
    hinglish: els.lang.value === "hi" && els.hinglish.checked };
}

function fmt(us) {
  if (us < 1000) return `${us.toFixed(us < 10 ? 1 : 0)} µs`;
  return `${(us / 1000).toFixed(2)} ms`;
}

// Transliteration packs: "pack" once loaded, "rules" when the site does not carry one.
const xlit = new Map();
async function ensureTranslit(lang) {
  if (!xlit.has(lang)) {
    xlit.set(lang, uccharan.loadTranslit(lang).then(() => { els.xlitNote.hidden = false; return "pack"; }, () => "rules"));
  }
  return xlit.get(lang);
}

let pending = 0;
async function render() {
  const ticket = ++pending;
  const opt = options();
  const t = task();
  const text = els.text.value;
  els.hinglishWrap.hidden = opt.lang !== "hi" || t !== "phonemes";
  els.profileWrap.classList.toggle("off", t !== "phonemes");
  els.profile.disabled = t !== "phonemes";
  els.normalizeWrap.classList.toggle("off", t !== "phonemes");
  els.normalize.disabled = t !== "phonemes";
  els.modeWrap.classList.toggle("off", t === "native");
  els.modeWrap.disabled = t === "native";
  try {
    // The async calls fetch whatever data the text needs; the timed calls below are synchronous.
    const slow = setTimeout(() => { if (ticket === pending) els.timing.textContent = "loading…"; }, 60);
    let run;
    let note = t === "phonemes" ? NOTES[mode()] : NOTES[t];
    if (t === "native") {
      if ((await ensureTranslit(opt.lang)) === "rules") note = `${NOTES.native} ${NOTES.rules}`;
      await uccharan.transliterate(text, { lang: opt.lang });
      run = () => uccharan.transliterateSync(text, { lang: opt.lang }).text;
    } else if (t === "spoken") {
      await uccharan.normalize(text, { lang: opt.lang, improved: opt.improved });
      run = () => uccharan.normalizeSync(text, { lang: opt.lang, improved: opt.improved }).text;
    } else {
      await uccharan.phonemize(text, opt);
      run = () => uccharan.phonemizeSync(text, opt);
    }
    clearTimeout(slow);
    if (ticket !== pending) return;
    // Time the call: repeat until at least 2 ms have passed so that the clock's resolution does not matter.
    let result;
    let runs = 0;
    const t0 = performance.now();
    do { result = run(); runs++; } while (performance.now() - t0 < 2 && runs < 2000);
    const us = ((performance.now() - t0) * 1000) / runs;
    const words = text.trim() ? text.trim().split(/\s+/).length : 0;
    els.timing.textContent = words ? `${fmt(us)} · ${words} word${words === 1 ? "" : "s"}` : "ready";
    els.note.textContent = note;
    show(result, opt, t);
  } catch (err) {
    if (ticket !== pending) return;
    els.out.textContent = String(err && err.message ? err.message : err);
    els.timing.textContent = "error";
  }
}

function show(result, opt, t) {
  els.out.classList.toggle("small", t === "phonemes" && opt.profile === "sherpa-onnx");
  els.out.classList.toggle("text", t !== "phonemes");
  els.out.lang = t === "phonemes" ? "und" : opt.lang;
  els.out.replaceChildren();
  let ids = null;
  if (t === "spoken") {
    els.title.textContent = "Spoken text";
    els.out.textContent = result;
  } else if (t === "native") {
    els.title.textContent = "Native script";
    els.out.textContent = result;
  } else if (opt.profile === "piper") {
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
  for (const ex of examples()) {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = ex;
    b.lang = els.lang.value;
    b.addEventListener("click", () => {
      els.text.value = ex;
      render();
    });
    els.examples.append(b);
  }
}

// Any example of the language, in either list.
const isExample = (text) => [...LANGS.values()].some((l) => l.examples.includes(text) || (l.romanised || []).includes(text));

function syncTask() {
  const native = canTransliterate(els.lang.value);
  els.taskNative.classList.toggle("off", !native);
  els.taskNative.querySelector("input").disabled = !native;
  els.taskNative.title = native ? "" : "Transliteration is available for Hindi, Marathi, Bengali, Tamil, Telugu, Gujarati, Punjabi, Kannada and Malayalam";
  if (!native && task() === "native") document.querySelector('input[name="task"][value="phonemes"]').checked = true;
}

els.text.addEventListener("input", render);
for (const el of [els.profile, els.normalize, els.hinglish]) el.addEventListener("change", render);
for (const r of document.querySelectorAll('input[name="mode"]')) r.addEventListener("change", render);
for (const r of document.querySelectorAll('input[name="task"]')) {
  r.addEventListener("change", () => {
    const swap = !els.text.value.trim() || isExample(els.text.value);
    fillExamples();
    if (swap) els.text.value = examples()[0];
    render();
  });
}
els.lang.addEventListener("change", () => { syncTask(); fillExamples(); els.text.value = examples()[0]; render(); });
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

els.lang.value = "en-us";
syncTask();
fillExamples();
els.text.value = examples()[0];
uccharan.init().then(async () => {
  $("version").textContent = await uccharan.version();
  render();
}).catch((err) => { els.out.textContent = `Could not start the WebAssembly module: ${err.message}`; els.timing.textContent = "error"; });
