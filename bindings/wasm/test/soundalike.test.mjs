import assert from "node:assert/strict";
import { test } from "node:test";
import * as uccharan from "../index.js";

test("sound-alike corrections list every change", async () => {
  const index = await uccharan.createSoundIndex(["Kothari", { word: "Paytm", say: "pay T M" }]);
  assert.equal(index.length, 2);
  const text = "😀 call Coat Harry and pay tm";
  const fixed = index.apply(text);
  assert.equal(fixed.text, "😀 call Kothari and Paytm");
  assert.deepEqual(fixed.changes.map((c) => [c.from, c.to]), [["Coat Harry", "Kothari"], ["pay tm", "Paytm"]]);
  for (const c of fixed.changes) {
    assert.equal(text.slice(c.start, c.end), c.from);
    assert.equal(fixed.text.slice(c.outStart, c.outEnd), c.to);
  }
  // A confident recogniser is left alone; the candidates are still reported.
  const sure = index.apply(text, { confidence: [1, 1, 1, 1, 1, 1, 1], maxConfidence: 0.5 });
  assert.equal(sure.text, text);
  assert.equal(sure.candidates.length, 2);
  assert.throws(() => index.apply(text, { confidence: [1] }));
  index.free();
});

test("sound-alike matching across scripts", async () => {
  const found = await uccharan.matchTerms("कोठारी जी को फ़ोन करना है।", ["Kothari"], { lang: "hi" });
  assert.equal(found.length, 1);
  assert.equal(found[0].term, "Kothari");
  assert.equal(found[0].text, "कोठारी");
  assert.equal(found[0].exact, false);
  const plain = await uccharan.matchTerms("The weather was fine.", ["Kothari"]);
  assert.deepEqual(plain, []);
});
