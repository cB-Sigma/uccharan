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
