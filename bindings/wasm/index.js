// uccharan — text to phonemes in the browser and Node. Apache-2.0.
//
// The WebAssembly core carries no language data. Each language's data file
// (`data/<lang>.ucd`) is fetched the first time it is needed.

import initWasm, * as wasm from "./dist/uccharan_wasm.js";
import { PACKS } from "./packs.js";

export const LANGUAGES = Object.freeze(["en-us", "en-gb", "hi", "bn", "mr", "te", "ne", "kn", "ta", "ml", "as", "gu", "bho", "mai", "pa", "or", "kok", "si", "sa", "ur", "sd", "mag", "es", "de", "nl", "fr", "it", "pt-br", "pt", "pl", "tr", "cs", "sv", "ca", "ro", "hu", "fi", "nb", "da"]);
export const PROFILES = Object.freeze(["espeak-ipa", "piper", "kokoro", "kitten", "matcha", "sherpa-onnx"]);

const ALIASES = { en: "en-us", "en-us": "en-us", "en-gb": "en-gb", "en-gb-x-rp": "en-gb", "en-uk": "en-gb", hi: "hi", "hi-in": "hi", bn: "bn", "bn-in": "bn", "bn-bd": "bn", mr: "mr", te: "te", ne: "ne", "ne-np": "ne", kn: "kn", ta: "ta", ml: "ml", as: "as", gu: "gu", bho: "bho", bhojpuri: "bho", mai: "mai", maithili: "mai", pa: "pa", "pa-in": "pa", or: "or", odia: "or", oriya: "or", kok: "kok", gom: "kok", konkani: "kok", si: "si", sinhala: "si", sa: "sa", sanskrit: "sa", ur: "ur", "ur-pk": "ur", urdu: "ur", sd: "sd", sindhi: "sd", mag: "mag", magahi: "mag", es: "es", "es-es": "es", spanish: "es", de: "de", "de-de": "de", deu: "de", german: "de", nl: "nl", "nl-nl": "nl", "nl-be": "nl", nld: "nl", dutch: "nl", fr: "fr", "fr-fr": "fr", fra: "fr", french: "fr", it: "it", "it-it": "it", ita: "it", italian: "it", "pt-br": "pt-br", "por-br": "pt-br", brazilian: "pt-br", pt: "pt", "pt-pt": "pt", por: "pt", portuguese: "pt", pl: "pl", "pl-pl": "pl", pol: "pl", polish: "pl", tr: "tr", "tr-tr": "tr", tur: "tr", turkish: "tr", cs: "cs", "cs-cz": "cs", ces: "cs", cze: "cs", czech: "cs", sv: "sv", "sv-se": "sv", swe: "sv", swedish: "sv", ca: "ca", "ca-es": "ca", cat: "ca", catalan: "ca", ro: "ro", "ro-ro": "ro", ron: "ro", rum: "ro", romanian: "ro", hu: "hu", "hu-hu": "hu", hun: "hu", hungarian: "hu", fi: "fi", "fi-fi": "fi", fin: "fi", finnish: "fi", nb: "nb", "nb-no": "nb", no: "nb", nob: "nb", nor: "nb", norwegian: "nb", da: "da", "da-dk": "da", dan: "da", danish: "da" };
const isNode = typeof process !== "undefined" && process.versions != null && process.versions.node != null && typeof window === "undefined";

let ready = null;
let wasmReady = false;
let dataBase = new URL("./data/", import.meta.url);
const loading = new Map();
// Latin-script languages read Latin text themselves.
const LATIN = new Set(["es", "de", "nl", "fr", "it", "pt-br", "pt", "pl", "tr", "cs", "sv", "ca", "ro", "hu", "fi", "nb", "da"]);
const WORD_TABLES = new Set(["ur", "sd", "de", "nl", "fr", "it", "pt-br", "pt", "pl", "tr", "cs", "sv", "ca", "ro", "hu", "fi", "nb", "da"]);
const tablesLoaded = new Set();

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
  // Urdu and Sindhi read with rules alone until their word tables arrive.
  if (wasm.has(tag) && (!WORD_TABLES.has(tag) || tablesLoaded.has(tag))) return;
  if (WORD_TABLES.has(tag)) tablesLoaded.add(tag);
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
  // Latin-script languages read Latin text themselves.
  const latin = LATIN.has(tag);
  if (!english && !latin && HAS_LATIN.test(text)) need.push("en-gb");
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
  if (o.wordlists && o.wordlists.length) {
    return wasm.renderWordlists(text, lang, profile, normalize, improved, hinglish, o.punctuation !== false, Boolean(o.languageFlags), Array.from(o.wordlists));
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

// Normalise-only mode and pronunciation word lists.

function normalizeRun(text, o) {
  const lang = canonical(o.lang ?? "en-us");
  const opts = {
    mode: o.improved ? "improved" : "compatible",
    digits: o.digits ?? "context",
    en_scale: o.enScale ?? "auto",
    latin: o.latin ?? "keep",
    urls: o.urls ?? "read",
    respell: o.respell ?? "native",
    wordlists: Array.from(o.wordlists ?? []),
  };
  const r = JSON.parse(wasm.normalizeJson(text, lang, JSON.stringify(opts)));
  return {
    text: r.text,
    sentences: r.sentences.map((s) => ({ start: s.utf16[0], end: s.utf16[1] })),
    spans: r.spans.map((s) => ({
      kind: s.kind,
      lang: s.lang,
      start: s.src.utf16[0],
      end: s.src.utf16[1],
      outStart: s.out.utf16[0],
      outEnd: s.out.utf16[1],
      text: s.text,
      spoken: s.spoken,
      src: s.src,
      out: s.out,
      entry: s.entry ?? null,
      value: s.value ?? null,
    })),
  };
}

/**
 * Spoken form of `text` for a voice that reads text, with typed spans whose
 * `start`/`end` are UTF-16 offsets into `text` (`src.byte`, `src.char` give
 * byte and code-point offsets).
 */
export async function normalize(text, options = {}) {
  await init();
  wasmReady = true;
  const langs = new Set(languagesFor(text, options.lang));
  if (!canonical(options.lang ?? "en-us").startsWith("en")) langs.add("en-gb");
  await Promise.all([...langs].map((l) => loadLanguage(l)));
  return normalizeRun(text, options);
}

/** Synchronous variant of `normalize`; the needed languages must be loaded. */
export function normalizeSync(text, options = {}) {
  if (!wasmReady) throw new Error("uccharan is not initialised: await init() and loadLanguage() first");
  return normalizeRun(text, options);
}

// Romanised text into native script.

/**
 * Load the transliteration pack of a language (`xlit-hi`, `xlit-mr`,
 * `xlit-bn`, `xlit-ta`, `xlit-te`, `xlit-gu`, `xlit-pa`, `xlit-kn`,
 * `xlit-ml`). These packs are trained on the Dakshina and Aksharantar word
 * pairs and are under CC BY-SA 4.0, not Apache-2.0, so they are not in this
 * package and never fetched automatically: download the file from the
 * GitHub release, then pass its bytes, or place it in the data directory
 * (see `init`). Files from the data directory are checked by SHA-256.
 */
export async function loadTranslit(lang, bytes) {
  const key = `xlit-${canonical(lang)}`;
  await init();
  wasmReady = true;
  if (bytes) {
    wasm.loadPack(bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes));
    return;
  }
  if (!loading.has(key)) {
    loading.set(key, readPack(key).then((b) => wasm.loadPack(b)));
  }
  await loading.get(key);
}

/** True when the transliteration pack of the language is loaded. */
export function hasTranslit(lang) {
  return wasmReady && wasm.hasTranslit(canonical(lang));
}

function transliterateRun(text, o) {
  const lang = canonical(o.lang ?? "hi");
  const opts = { english: o.english ?? "keep", alternatives: o.alternatives ?? 3, context: o.context !== false };
  const r = JSON.parse(wasm.transliterateJson(text, lang, JSON.stringify(opts)));
  return {
    text: r.text,
    pack: wasm.hasTranslit(lang),
    spans: r.spans.map((s) => ({
      kind: s.kind,
      start: s.src.utf16[0],
      end: s.src.utf16[1],
      outStart: s.out.utf16[0],
      outEnd: s.out.utf16[1],
      text: s.text,
      written: s.written,
      src: s.src,
      out: s.out,
      alternatives: s.alternatives ?? [],
    })),
  };
}

/**
 * Write the romanised words of `text` in the script of `options.lang`
 * (default `hi`): "aap kaise ho" → "आप कैसे हो". English words stay as they
 * are (`english: "keep"`), are respelled in the script (`"native"`), or no
 * language decision is made (`"convert"`). Spans carry UTF-16 offsets into
 * `text` and the alternatives of each word. Uses the language's pack when
 * `loadTranslit` has loaded it (`pack` in the result); otherwise a plain
 * rule converter answers.
 */
export async function transliterate(text, options = {}) {
  await init();
  wasmReady = true;
  const lang = canonical(options.lang ?? "hi");
  const langs = new Set([lang, "en-gb"]);
  if (["bho", "mai", "mag"].includes(lang)) langs.add("hi");
  await Promise.all([...langs].map((l) => loadLanguage(l).catch(() => {})));
  return transliterateRun(text, options);
}

/** Synchronous variant of `transliterate`; the needed data must be loaded. */
export function transliterateSync(text, options = {}) {
  if (!wasmReady) throw new Error("uccharan is not initialised: await init() and loadLanguage() first");
  return transliterateRun(text, options);
}

/** Respell IPA for a text voice reading `lang`; null when no writer exists. */
export async function respell(ipa, options = {}) {
  await init();
  wasmReady = true;
  const lang = canonical(options.lang ?? "en-us");
  await loadLanguage(lang);
  const r = wasm.respell(ipa, lang, options.source ?? "", options.style ?? "native");
  return r ? JSON.parse(r) : null;
}

/** Convert a word list (TSV, PLS or JSON text) to "tsv", "json" or "pls". */
export async function convertWordlist(text, to, lang = "") {
  await init();
  wasmReady = true;
  return wasm.convertWordlist(text, to, lang ? canonical(lang) : "");
}

// Sound-alike matching.

function soundCandidates(items) {
  return items.map((c) => ({
    start: c.src.utf16[0],
    end: c.src.utf16[1],
    text: c.text,
    term: c.term,
    index: c.index,
    score: c.score,
    exact: c.exact,
    kind: c.kind,
    words: c.words,
    src: c.src,
  }));
}

/**
 * A vocabulary prepared for sound-alike matching; make one with
 * `createSoundIndex`. `find` and `apply` are synchronous: the language of
 * the transcript must be loaded (`createSoundIndex` loads English, Hindi and
 * the languages it is given).
 */
export class SoundIndex {
  constructor(native) {
    this.native = native;
  }

  /** Number of terms. */
  get length() {
    return this.native.length;
  }

  /** Places in `text` that sound like a term (UTF-16 `start`/`end`). */
  find(text, options = {}) {
    const lang = canonical(options.lang ?? "en-us");
    const opts = { max_words: options.maxWords ?? 3, min_score: options.minScore ?? 0.6, all: Boolean(options.all) };
    return soundCandidates(JSON.parse(this.native.findJson(text, lang, JSON.stringify(opts))).candidates);
  }

  /** `text` with what sounds like a term replaced by the term, and every change made. */
  apply(text, options = {}) {
    const lang = canonical(options.lang ?? "en-us");
    const opts = { max_words: options.maxWords ?? 3, fix_case: options.fixCase !== false };
    if (options.threshold != null) opts.threshold = options.threshold;
    if (options.uncertainThreshold != null) opts.uncertain_threshold = options.uncertainThreshold;
    if (options.ordinaryThreshold != null) opts.ordinary_threshold = options.ordinaryThreshold;
    if (options.confidence != null) opts.confidence = Array.from(options.confidence);
    if (options.maxConfidence != null) opts.max_confidence = options.maxConfidence;
    const r = JSON.parse(this.native.applyJson(text, lang, JSON.stringify(opts)));
    return {
      text: r.text,
      changes: r.changes.map((c) => ({
        start: c.src.utf16[0],
        end: c.src.utf16[1],
        outStart: c.out.utf16[0],
        outEnd: c.out.utf16[1],
        from: c.from,
        to: c.to,
        index: c.index,
        score: c.score,
        caseOnly: c.case_only,
        src: c.src,
        out: c.out,
      })),
      candidates: soundCandidates(r.candidates),
    };
  }

  /** Release the WebAssembly memory of the index. */
  free() {
    this.native.free();
  }
}

/**
 * Build a sound-alike index over a vocabulary: an array of terms (strings,
 * or objects with `word` and optional `lang`, `ipa`, `say`), word-list text
 * (TSV, PLS or JSON), or text with one term per line. `options.langs` are
 * the languages a term without phonemes is read in (default en-us and hi);
 * `options.ordinaryWords` adds to the words taken for ordinary speech.
 */
export async function createSoundIndex(vocabulary, options = {}) {
  await init();
  wasmReady = true;
  const langs = (options.langs ?? ["en-us", "hi"]).map(canonical);
  await Promise.all([...new Set(["en-us", "hi", ...langs])].map((l) => loadLanguage(l)));
  const text = typeof vocabulary === "string" ? vocabulary : JSON.stringify(Array.from(vocabulary));
  const ordinary = Array.from(options.ordinaryWords ?? []);
  return new SoundIndex(new wasm.SoundIndex(text, JSON.stringify({ langs, ordinary_words: ordinary })));
}

/** Places in `text` that sound like a term of `vocabulary` (a `SoundIndex`, or anything `createSoundIndex` takes). */
export async function matchTerms(text, vocabulary, options = {}) {
  const own = !(vocabulary instanceof SoundIndex);
  const index = own ? await createSoundIndex(vocabulary, options) : vocabulary;
  await Promise.all(languagesFor(text, options.lang).map((l) => loadLanguage(l)));
  try {
    return index.find(text, options);
  } finally {
    if (own) index.free();
  }
}
