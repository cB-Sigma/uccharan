// uccharan — text to phonemes in the browser and Node. Apache-2.0.
//
// The WebAssembly core carries no language data. Each language's data file
// (`data/<lang>.ucd`) is fetched the first time it is needed.

import initWasm, * as wasm from "./dist/uccharan_wasm.js";
import { PACKS } from "./packs.js";

export const LANGUAGES = Object.freeze(["en-us", "en-gb", "hi", "bn", "mr", "te", "ne", "kn", "ta", "ml", "as", "gu", "bho", "mai", "pa", "or", "kok", "si", "sa"]);
export const PROFILES = Object.freeze(["espeak-ipa", "piper", "kokoro", "kitten", "matcha", "sherpa-onnx"]);

const ALIASES = { en: "en-us", "en-us": "en-us", "en-gb": "en-gb", "en-gb-x-rp": "en-gb", "en-uk": "en-gb", hi: "hi", "hi-in": "hi", bn: "bn", "bn-in": "bn", "bn-bd": "bn", mr: "mr", te: "te", ne: "ne", "ne-np": "ne", kn: "kn", ta: "ta", ml: "ml", as: "as", gu: "gu", bho: "bho", bhojpuri: "bho", mai: "mai", maithili: "mai", pa: "pa", "pa-in": "pa", or: "or", odia: "or", oriya: "or", kok: "kok", gom: "kok", konkani: "kok", si: "si", sinhala: "si", sa: "sa", sanskrit: "sa" };
const isNode = typeof process !== "undefined" && process.versions != null && process.versions.node != null && typeof window === "undefined";

let ready = null;
let wasmReady = false;
let dataBase = new URL("./data/", import.meta.url);
const loading = new Map();

function canonical(lang) {
  const tag = ALIASES[String(lang).toLowerCase().replace("_", "-")];
  if (!tag) throw new Error(`unknown language "${lang}" (expected ${LANGUAGES.join(", ")})`);
  return tag;
}

async function readBytes(url) {
  if (isNode && url.protocol === "file:") {
    const { readFile } = await import("node:fs/promises");
    return new Uint8Array(await readFile(url));
  }
  const res = await fetch(url);
  if (!res.ok) throw new Error(`failed to fetch ${url}: ${res.status}`);
  return new Uint8Array(await res.arrayBuffer());
}

async function sha256Hex(bytes) {
  const subtle = globalThis.crypto?.subtle ?? (isNode ? (await import("node:crypto")).webcrypto.subtle : null);
  if (!subtle) return null;
  const digest = new Uint8Array(await subtle.digest("SHA-256", bytes));
  return Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join("");
}

// Fetch a pack and check it against the SHA-256 sum this version was built with.
async function readPack(name) {
  const url = new URL(`${name}.ucd`, dataBase);
  const bytes = await readBytes(url);
  const want = PACKS[name];
  if (want) {
    const got = await sha256Hex(bytes);
    if (got != null && (got !== want.sha256 || bytes.length !== want.size)) {
      throw new Error(`${url}: SHA-256 ${got} does not match the ${name} pack of this uccharan version`);
    }
  }
  return bytes;
}

/**
 * Initialise the WebAssembly module. Called implicitly by the async
 * functions; call it yourself to choose where the files come from.
 */
export function init(options = {}) {
  if (!ready) {
    if (options.dataBaseUrl) dataBase = new URL(options.dataBaseUrl, typeof location !== "undefined" ? location.href : import.meta.url);
    ready = (async () => {
      let source = options.wasm;
      if (source == null) source = await readBytes(new URL("./dist/uccharan_wasm_bg.wasm", import.meta.url));
      else if (typeof source === "string" || source instanceof URL) source = await readBytes(new URL(source, import.meta.url));
      await initWasm({ module_or_path: source });
      wasmReady = true;
    })();
  }
  return ready;
}

/** Load a language's data (fetched from the data directory unless bytes are given). */
export async function loadLanguage(lang, bytes) {
  const tag = canonical(lang);
  await init();
  if (bytes) {
    wasm.load(tag, bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes));
    return;
  }
  if (wasm.has(tag)) return;
  if (!loading.has(tag)) {
    loading.set(
      tag,
      readPack(tag).then((b) => wasm.load(tag, b)).finally(() => loading.delete(tag)),
    );
  }
  await loading.get(tag);
}

/**
 * Load misaki's English dictionaries from their JSON (text or parsed objects
 * of `us_gold.json` and `us_silver.json`, or the `gb_` files for en-gb) in
 * place of the compiled copy that `loadKokoro` fetches. Call it before the
 * first Kokoro English request.
 */
export async function loadMisaki(lang, gold, silver) {
  await init();
  const text = (x) => (typeof x === "string" ? x : JSON.stringify(x));
  wasm.loadMisaki(canonical(lang), text(gold), text(silver));
}

/**
 * Fetch the Kokoro English data (misaki's dictionaries, compiled) for a
 * language from the data directory. `phonemize` does this by itself for the
 * `kokoro` profile; call it before `phonemizeSync`.
 */
export async function loadKokoro(lang = "en-us") {
  const tag = canonical(lang);
  await init();
  if (!tag.startsWith("en") || wasm.hasMisaki(tag)) return;
  const key = `kokoro-${tag}`;
  if (!loading.has(key)) {
    loading.set(key, readPack(key).then((b) => { if (!wasm.hasMisaki(tag)) wasm.loadKokoro(b); }).finally(() => loading.delete(key)));
  }
  await loading.get(key);
}

/**
 * Load the optional WikiPron pack of a language (`wikipron-en-us`,
 * `wikipron-en-gb`, `wikipron-hi`). These packs hold Wiktionary
 * pronunciations under CC BY-SA 4.0, not Apache-2.0, so they are not in this
 * package and never fetched automatically: download the file from the
 * GitHub release, then pass its bytes, or place it in the data directory
 * (see `init`). Files from the data directory are checked by SHA-256. The
 * improved mode uses the pack for words outside uccharan's own lexicon.
 */
export async function loadWikipron(lang, bytes) {
  const key = `wikipron-${canonical(lang)}`;
  await init();
  if (bytes) {
    wasm.loadPack(bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes));
    return;
  }
  if (!loading.has(key)) {
    loading.set(key, readPack(key).then((b) => wasm.loadPack(b)));
  }
  await loading.get(key);
}

/** True when misaki's dictionaries are loaded for the language. */
export function hasMisaki(lang) {
  return wasmReady && wasm.hasMisaki(canonical(lang));
}

/** True when the language's data is loaded. */
export function isLoaded(lang) {
  return ready != null && wasmReady && wasm.has(canonical(lang));
}

const HAS_LATIN = /[A-Za-z]/;
const HAS_DEVANAGARI = /[ऀ-ॿ]/;

/** Languages whose data a request needs (code-mixed text needs both). */
export function languagesFor(text, lang = "en-us") {
  const tag = canonical(lang);
  const need = [tag];
  const english = tag === "en-us" || tag === "en-gb";
  if (!english && HAS_LATIN.test(text)) need.push("en-gb");
  if (tag !== "hi" && HAS_DEVANAGARI.test(text)) need.push("hi");
  return need;
}

function run(text, o) {
  const lang = canonical(o.lang ?? "en-us");
  const profile = o.profile ?? "espeak-ipa";
  const normalize = o.normalize ?? "natural";
  const improved = Boolean(o.improved);
  const hinglish = Boolean(o.hinglish);
  if (profile === "piper" || profile === "sherpa-onnx") {
    const sentences = wasm.sentences(text, lang, normalize, improved, hinglish);
    if (profile === "piper") return sentences;
    return sentences.map((s) => Array.from(s, (ch) => ch.codePointAt(0)));
  }
  return wasm.render(text, lang, profile, normalize, improved, hinglish, o.punctuation !== false, Boolean(o.languageFlags));
}

/**
 * Convert text to phonemes, loading language data on first use.
 * Returns a string, or for the `piper` profile an array of sentence strings,
 * or for `sherpa-onnx` an array of codepoint arrays.
 */
export async function phonemize(text, options = {}) {
  await init();
  wasmReady = true;
  await Promise.all(languagesFor(text, options.lang).map((l) => loadLanguage(l)));
  if (options.profile === "kokoro" && canonical(options.lang ?? "en-us").startsWith("en")) await loadKokoro(options.lang ?? "en-us");
  return run(text, options);
}

/** Synchronous variant; the needed languages must already be loaded. */
export function phonemizeSync(text, options = {}) {
  if (!wasmReady) throw new Error("uccharan is not initialised: await init() and loadLanguage() first");
  for (const l of languagesFor(text, options.lang)) {
    if (!wasm.has(l)) throw new Error(`language data for "${l}" is not loaded: await loadLanguage("${l}") first`);
  }
  return run(text, options);
}

/** Piper phoneme ids `[BOS, PAD, p1, PAD, …, EOS]` for one sentence string and a voice config object. */
export function piperIds(sentence, voiceConfig) {
  const map = voiceConfig.phoneme_id_map ?? voiceConfig;
  const pad = map["_"] ?? [0];
  const ids = [];
  const missing = [];
  if (map["^"]) ids.push(...map["^"], ...pad);
  for (const ch of sentence.normalize("NFD")) {
    const v = map[ch];
    if (v) ids.push(...v, ...pad);
    else missing.push(ch);
  }
  if (map["$"]) ids.push(...map["$"]);
  return { ids, missing };
}

/** Kokoro-82M input ids for a Kokoro token string. */
export function kokoroIds(tokens) {
  return Array.from(wasm.kokoroIds(tokens));
}

/** Kitten TTS 0.8 input ids for a phoneme string. */
export function kittenIds(phonemes) {
  return Array.from(wasm.kittenIds(phonemes));
}

/** Matcha-TTS input ids for a phoneme string. */
export function matchaIds(phonemes) {
  return Array.from(wasm.matchaIds(phonemes));
}

/** Library version (available after init). */
export async function version() {
  await init();
  wasmReady = true;
  return wasm.version();
}
