import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import * as uccharan from "../index.js";

const TOY = new URL("../../../crates/uccharan/tests/data/xlit-toy.ucd", import.meta.url);

test("without a pack a rule converter answers", async () => {
  const t = await uccharan.transliterate("kamal", { lang: "bn", english: "convert" });
  assert.equal(t.text, "কমল");
  assert.equal(t.pack, false);
  assert.equal(uccharan.hasTranslit("bn"), false);
});

test("transliterate with a pack gives text and UTF-16 spans", async () => {
  await uccharan.loadTranslit("hi", await readFile(TOY));
  assert.equal(uccharan.hasTranslit("hi"), true);
  const text = "mujhe kal the jana hai 😀!";
  const t = await uccharan.transliterate(text);
  assert.equal(t.pack, true);
  assert.equal(t.text, "मुझे कल the जाना है 😀!");
  assert.equal(t.spans.map((s) => s.text).join(""), text);
  assert.equal(t.spans.map((s) => s.written).join(""), t.text);
  const kal = t.spans.find((s) => s.text === "kal");
  assert.equal(kal.kind, "indic");
  assert.equal(text.slice(kal.start, kal.end), "kal");
  assert.equal(t.text.slice(kal.outStart, kal.outEnd), "कल");
  assert.equal(kal.alternatives[0].text, "कल");
  assert.equal(t.spans.find((s) => s.text === "the").kind, "english");
  const all = uccharan.transliterateSync("kal the jana hai", { english: "convert" });
  assert.ok(!all.text.includes("the"));
  assert.throws(() => uccharan.transliterateSync("kal", { english: "maybe" }));
});
