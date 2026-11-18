import assert from "node:assert/strict";
import { test } from "node:test";
import * as uccharan from "../index.js";

const nfd = (s) => s.normalize("NFD");

test("phonemize loads data lazily", async () => {
  assert.equal(uccharan.isLoaded("en-us"), false);
  assert.equal(await uccharan.phonemize("Hello, world."), "həlˈoʊ, wˈɜːld.");
  assert.equal(uccharan.isLoaded("en-us"), true);
  assert.equal(uccharan.isLoaded("hi"), false);
  assert.match(await uccharan.version(), /^\d+\.\d+\.\d+$/);
});

test("languages and code mixing", async () => {
  assert.equal(await uccharan.phonemize("The car park is far.", { lang: "en-gb" }), "ðə kˈɑː pˈɑːk ɪz fˈɑː.");
  assert.equal(nfd(await uccharan.phonemize("नमस्ते दुनिया", { lang: "hi" })), nfd("nəmˈʌsteː dˈʊnɪjˌaː"));
  assert.deepEqual(uccharan.languagesFor("मैं office जा रहा हूँ", "hi"), ["hi", "en-gb"]);
  const mixed = await uccharan.phonemize("मैं office जा रहा हूँ", { lang: "hi", languageFlags: true });
  assert.ok(mixed.includes("(en)") && mixed.includes("(hi)"));
  const roman = await uccharan.phonemize("main ghar ja raha hoon", { lang: "hi", hinglish: true });
  assert.equal(nfd(roman), nfd(await uccharan.phonemize("मैं घर जा रहा हूँ", { lang: "hi" })));
});

test("profiles", async () => {
  const s = await uccharan.phonemize("Hi there. Bye!", { profile: "piper" });
  assert.deepEqual(s, ["hˈaɪ ðˈɛɹ.", "bˈaɪ!"]);
  const cps = await uccharan.phonemize("Hi there. Bye!", { profile: "sherpa-onnx" });
  assert.deepEqual(cps, s.map((x) => Array.from(x, (c) => c.codePointAt(0))));
  const k = await uccharan.phonemize("The butter is better.", { profile: "kokoro" });
  assert.ok(k.includes("T"));
  const ids = uccharan.kokoroIds(k);
  assert.equal(ids[0], 0);
  assert.equal(ids.at(-1), 0);
  assert.deepEqual(uccharan.kittenIds("a"), [0, 43, 10, 0]);
  assert.deepEqual(uccharan.matchaIds("ab"), [0, 43, 0, 44, 0]);
  const cfg = { phoneme_id_map: { _: [0], "^": [1], $: [2], h: [20], a: [14], "ɪ": [74], "ˈ": [120], ".": [10] } };
  assert.deepEqual(uccharan.piperIds("hˈaɪ.", cfg), { ids: [1, 0, 20, 0, 120, 0, 14, 0, 74, 0, 10, 0, 2], missing: [] });
});

test("sync API and errors", async () => {
  await uccharan.loadLanguage("en-us");
  assert.equal(uccharan.phonemizeSync("Hello"), "həlˈoʊ");
  await assert.rejects(() => uccharan.phonemize("x", { lang: "xx" }), /unknown language/);
  await assert.rejects(() => uccharan.phonemize("x", { profile: "nope" }), /unknown profile/);
  assert.equal(await uccharan.phonemize(""), "");
});

test("synchronous use after init and loadLanguage only", async () => {
  await uccharan.init();
  await uccharan.loadLanguage("en-gb");
  assert.equal(uccharan.phonemizeSync("The car park is far.", { lang: "en-gb" }), "ðə kˈɑː pˈɑːk ɪz fˈɑː.");
  assert.throws(() => uccharan.phonemizeSync("x", { lang: "xx" }));
});

test("Kokoro English: compiled misaki data is fetched on demand", async () => {
  assert.equal(await uccharan.phonemize("I read the book yesterday.", { profile: "kokoro" }), "ˌI ɹˈɛd ðə bˈʊk jˈɛstəɹdˌA.");
  assert.equal(uccharan.hasMisaki("en-us"), true);
  assert.equal(uccharan.hasMisaki("en-gb"), false);
  await uccharan.loadMisaki("en-gb", { hello: "həlˈQ" }, "{}");
  assert.equal(await uccharan.phonemize("Hello.", { lang: "en-gb", profile: "kokoro" }), "həlˈQ.");
});

test("data-driven Indic languages need no data file", async () => {
  const { phonemize, LANGUAGES } = await import("../index.js");
  assert.ok(LANGUAGES.includes("bn"));
  assert.equal(await phonemize("কমলা", { lang: "bn" }), "kˈɔmɔlˌa");
  assert.equal(await phonemize("తెలుగు", { lang: "te" }), "tˈeluɡu");
});

test("a word table is fetched on first use, and the sync call says when it is missing", async () => {
  // Swedish has not been used by any test above.
  assert.equal(uccharan.isLoaded("sv"), false);
  assert.throws(() => uccharan.phonemizeSync("Hej", { lang: "sv" }), /language data for "sv" is not loaded/);
  assert.ok((await uccharan.phonemize("Hej", { lang: "sv" })).length > 0);
  assert.equal(uccharan.isLoaded("sv"), true);
  assert.ok(uccharan.phonemizeSync("Hej", { lang: "sv" }).length > 0);
});

test("Bhojpuri, Maithili and Magahi load the Hindi data and need no file of their own", async () => {
  for (const lang of ["mai", "bho", "mag"]) {
    const out = await uccharan.phonemize("प्रणाम! आज मौसम बहुत नीक बा।", { lang });
    assert.ok(out.length > 0, lang);
  }
  assert.equal(uccharan.isLoaded("hi"), true);
});

test("Dogri: Hindi reading in compatible mode, tone in improved mode", async () => {
  assert.ok(uccharan.LANGUAGES.includes("doi"));
  assert.equal(await uccharan.phonemize("घर", { lang: "doi" }), await uccharan.phonemize("घर", { lang: "hi" }));
  assert.equal(await uccharan.phonemize("घर", { lang: "dogri", improved: true }), "kˈʌ̀ɾ");
  assert.equal((await uccharan.normalize("25", { lang: "doi", improved: true })).text, "पंजी");
  assert.equal((await uccharan.transliterate("tusen kiyan ho", { lang: "doi" })).text, "तुसें कि’यां हो");
});

test("Bodo: Hindi reading in compatible mode, Bodo vowels in improved mode", async () => {
  assert.ok(uccharan.LANGUAGES.includes("brx"));
  assert.equal(await uccharan.phonemize("बोलो", { lang: "brx" }), await uccharan.phonemize("बोलो", { lang: "hi" }));
  assert.equal(await uccharan.phonemize("बोलो", { lang: "bodo", improved: true }), "bˈɯlɯ");
  assert.equal(await uccharan.phonemize("बर’", { lang: "brx", improved: true }), "bˈɔrɔ");
  assert.equal((await uccharan.normalize("25", { lang: "brx", improved: true })).text, "नैजिबा");
  assert.equal((await uccharan.transliterate("ang boro raw bungw", { lang: "brx" })).text, "आं बर’ राव बुङो");
});

test("Santali: one reading in both modes (espeak-ng cannot read Ol Chiki)", async () => {
  assert.ok(uccharan.LANGUAGES.includes("sat"));
  assert.equal(await uccharan.phonemize("ᱯᱟᱹᱨᱥᱤ", { lang: "sat" }), await uccharan.phonemize("ᱯᱟᱹᱨᱥᱤ", { lang: "santali", improved: true }));
  assert.equal(await uccharan.phonemize("ᱯᱟᱹᱨᱥᱤ", { lang: "sat" }), "pərsˈi");
  assert.equal(await uccharan.phonemize("ᱢᱤᱫ", { lang: "sat" }), "mˈit̚ʔ");
  assert.equal((await uccharan.normalize("25", { lang: "sat" })).text, "ᱵᱟᱨ ᱜᱮᱞ ᱢᱚᱬᱮ");
  assert.equal((await uccharan.transliterate("johar inj disom do santali kana", { lang: "sat" })).text, "ᱡᱚᱦᱟᱨ ᱤᱧ ᱫᱤᱥᱚᱢ ᱫᱚ ᱥᱟᱱᱛᱟᱲᱤ ᱠᱟᱱᱟ");
});

test("Manipuri: Meetei Mayek and Bengali script, one reading in both modes", async () => {
  assert.ok(uccharan.LANGUAGES.includes("mni"));
  assert.equal(await uccharan.phonemize("ꯃꯤꯇꯩꯂꯣꯟ", { lang: "mni" }), await uccharan.phonemize("ꯃꯤꯇꯩꯂꯣꯟ", { lang: "manipuri", improved: true }));
  assert.equal(await uccharan.phonemize("ꯃꯤꯇꯩꯂꯣꯟ", { lang: "mni" }), "miteilon");
  assert.equal(await uccharan.phonemize("মৈতৈলোন", { lang: "mni" }), "meiteilon");
  assert.equal((await uccharan.normalize("25", { lang: "mni" })).text, "ꯀꯨꯟꯃꯉꯥ");
  assert.equal((await uccharan.normalize("অমা 25", { lang: "mni" })).text, "অমা কুন্মঙা");
  assert.equal((await uccharan.transliterate("eina nangbu nungsi", { lang: "mni" })).text, "ꯑꯩꯅꯥ ꯅꯪꯕꯨ ꯅꯨꯡꯁꯤ");
});
