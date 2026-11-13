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
