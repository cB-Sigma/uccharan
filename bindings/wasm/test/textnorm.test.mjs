import assert from "node:assert/strict";
import { test } from "node:test";
import * as uccharan from "../index.js";

const TSV = "word\tlang\tipa\tsay\tmatch\nKokoro\ten\tkəkˈɔːɹoʊ\t\t\nSBI\thi\t\tएस बी आई\tcase\n";

test("normalize gives spoken text and UTF-16 spans", async () => {
  const text = "Pay ₹500 by 10:30.";
  const n = await uccharan.normalize(text);
  assert.equal(n.text, "Pay five hundred rupees by ten thirty.");
  const money = n.spans.find((s) => s.kind === "money");
  assert.equal(text.slice(money.start, money.end), "₹500");
  assert.equal(n.text.slice(money.outStart, money.outEnd), "five hundred rupees");
  assert.equal(n.spans.map((s) => s.text).join(""), text);
  const hi = await uccharan.normalize("कुल 12 students", { lang: "hi", digits: "request" });
  assert.equal(hi.text, "कुल बारह students");
});

test("word lists in both modes", async () => {
  const n = await uccharan.normalize("SBI में", { lang: "hi", wordlists: [TSV] });
  assert.equal(n.text, "एस बी आई में");
  const ipa = await uccharan.phonemize("Kokoro", { wordlists: [TSV] });
  assert.ok(ipa.startsWith("kəkˈɔːɹoʊ"), ipa);
  const pls = await uccharan.convertWordlist(TSV, "pls", "en-us");
  assert.ok(pls.includes("<phoneme>kəkˈɔːɹoʊ</phoneme>"));
  const r = await uccharan.respell("kəkˈɔːɹoʊ", { lang: "hi", source: "en-us" });
  assert.equal(r.text, "ककॉरो");
});

test("Latin-script languages read their own numbers", async () => {
  const de = await uccharan.normalize("Ich habe 23 Euro.", { lang: "de" });
  assert.equal(de.text, "Ich habe dreiundzwanzig Euro.");
  const n = de.spans.find((s) => s.kind === "number");
  assert.equal(n.lang, "de");
  assert.equal(n.value, "23");
  const be = await uccharan.normalize("Il a 71 ans.", { lang: "fr-be" });
  assert.equal(be.text, "Il a septante et un ans.");
  const es = await uccharan.normalize("Tengo 1 500 libros.", { lang: "es", improved: true });
  assert.equal(es.text, "Tengo mil quinientos libros.");
  const da = await uccharan.normalize("Jeg har 95 kroner.", { lang: "da", digits: "request" });
  assert.equal(da.text, "Jeg har femoghalvfems kroner.");
});

test("non-Latin languages write numbers in their own script", async () => {
  const twentyFive = {
    hi: "पच्चीस", bn: "পঁচিশ", mr: "पंचवीस", te: "ఇరవై ఐదు", ne: "पच्चीस",
    kn: "ಇಪ್ಪತ್ತೈದು", ta: "இருபத்து ஐந்து", ml: "ഇരുപത്തിയഞ്ച്", as: "পঁচিশ",
    gu: "પચ્ચીસ", pa: "ਪੱਚੀ", or: "ପଚିଶ", kok: "पंचवीस", si: "විසිපහ", sa: "पच्चीस",
    ur: "پچیس", sd: "پنجويهه", bho: "पच्चीस", mai: "पच्चीस", mag: "पच्चीस",
    doi: "पच्चीस", brx: "पच्चीस",
  };
  const block = (lang) =>
    ({ bn: 0x0980, as: 0x0980, pa: 0x0a00, gu: 0x0a80, or: 0x0b00, ta: 0x0b80, te: 0x0c00,
       kn: 0x0c80, ml: 0x0d00, si: 0x0d80, ur: 0x0600, sd: 0x0600 })[lang] ?? 0x0900;
  for (const [lang, want] of Object.entries(twentyFive)) {
    assert.equal((await uccharan.normalize("25", { lang })).text, want, lang);
    const lo = block(lang);
    for (const text of ["1,234", "3.5", "₹500", "15/08/2024"]) {
      const out = (await uccharan.normalize(text, { lang })).text;
      for (const c of out) {
        const u = c.codePointAt(0);
        assert.ok(" ‌‍".includes(c) || (u >= lo && u < lo + 0x80) ||
          (lo === 0x0600 && u < 0x0700 && u >= 0x0600), `${lang} ${text}: ${out}`);
      }
    }
  }
  assert.equal((await uccharan.normalize("1,234", { lang: "si" })).text, "එක්දහස් දෙසිය තිස්හතර");
  assert.equal((await uccharan.normalize("25 9 100", { lang: "kok" })).text, "पंचवीस णव शंबर");
  assert.equal((await uccharan.normalize("₹500", { lang: "sd" })).text, "پنج سؤ رپيا");
});

test("dates and ordinals are read in full", async () => {
  assert.equal((await uccharan.normalize("15.08.2024", { lang: "de" })).text,
    "fünfzehnter August zweitausend vierundzwanzig");
  assert.equal((await uccharan.normalize("le 1er mai", { lang: "fr" })).text, "le premier mai");
  assert.equal((await uccharan.normalize("200 000", { lang: "fr" })).text, "deux cent mille");
  assert.equal((await uccharan.normalize("el 2º", { lang: "es" })).text, "el segundo");
  assert.equal((await uccharan.normalize("25,50 €", { lang: "de" })).text, "fünfundzwanzig Euro fünfzig");
  assert.equal((await uccharan.normalize("Am 3. Mai", { lang: "de" })).text, "Am dritten Mai");
});

test("times, currency plurals, hyphenated ordinals and sentence ends", async () => {
  const n = async (t, lang) => (await uccharan.normalize(t, { lang })).text;
  assert.equal(await n("Il est 10h30.", "fr"), "Il est dix heures trente.");
  assert.equal(await n("às 10h30", "pt"), "às dez horas e trinta");
  assert.equal(await n("um 10.30 Uhr", "de"), "um zehn Uhr dreißig");
  assert.equal(await n("Kello on 10.30.", "fi"), "Kello on kymmenen kolmekymmentä.");
  assert.equal(await n("Ich kaufe das iPhone 15. Am Montag kommt es.", "de"),
    "Ich kaufe das iPhone fünfzehn. Am Montag kommt es.");
  assert.equal(await n("the 3rd-floor flat", "en-us"), "the third floor flat");
  assert.equal(await n("25 €", "fr"), "vingt-cinq euros");
  assert.equal(await n("25 €", "es"), "veinticinco euros");
  assert.equal(await n("25 €", "it"), "venticinque euro");
  assert.equal(await n("25 €", "pt"), "vinte e cinco euros");
});
