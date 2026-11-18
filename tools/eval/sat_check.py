#!/usr/bin/env python3
"""Santali (Ol Chiki) coverage check on the sentence sets.

espeak-ng has no Santali voice and reads Ol Chiki letters as their code
points, so the sentence sets of Santali cannot be scored against a
reference. This script measures what can be measured without one, on any of
tools/eval/data/sat_{dev,heldout,flores,tatoeba,wiki}.tsv, for uccharan's
reading (both modes give the same reading):

- words: Ol Chiki words in the set, and the share read to a non-empty
  pronunciation made only of the phonemes of lang/sat.lang;
- marks not said: Ol Chiki marks the rules do not read because the letter
  before them does not take them (ᱹ after a consonant, ᱼ ᱽ after a letter
  other than ᱜ ᱡ ᱫ ᱵ, ...), per thousand Ol Chiki characters;
- other characters: characters outside Ol Chiki, ASCII and common
  punctuation (Bengali or Devanagari letters inside Santali text).

The held-out sets are only measured, never used to change the rules.

    python tools/eval/sat_check.py --set heldout --out tools/eval/results
"""

from __future__ import annotations

import argparse
import json
import re
import unicodedata
from pathlib import Path

from common import run_uccharan

HERE = Path(__file__).resolve().parent
PHONEMES = set("aeiouɔəɛ̃ːptʈckbdɖɟɡmnɳɲŋshrɽljw̚ʔʰʱ")
VOWELS = set("ᱚᱟᱤᱩᱮᱳ")
GLOTTAL = set("ᱜᱡᱫᱵ")
WORD = re.compile(r"[ᱚ-ᱽ]+")


def stray_marks(word: str) -> int:
    """Marks of `word` that the Santali rules do not say."""
    n = 0
    for i, c in enumerate(word):
        prev = word[i - 1] if i else ""
        prev2 = word[i - 2] if i > 1 else ""
        if c == "ᱹ" and prev not in "ᱟᱮᱚ":
            n += 1
        elif c in "ᱸᱺ" and prev not in VOWELS and not (prev == "ᱹ" and prev2 in "ᱟᱮᱚ"):
            n += 1
        elif c == "ᱻ" and prev not in VOWELS | GLOTTAL | {"ᱸ", "ᱹ"}:
            n += 1
        elif c in "ᱼᱽ" and prev not in GLOTTAL:
            n += 1
        elif c == "ᱷ" and (not prev or prev in VOWELS):
            n += 1
    return n


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--set", default="heldout")
    ap.add_argument("--out", type=Path)
    args = ap.parse_args()
    rows = [l.split("\t", 1) for l in (HERE / "data" / f"sat_{args.set}.tsv").read_text(encoding="utf-8").splitlines() if "\t" in l]
    texts = [t for _, t in rows]
    # A mark standing alone (ᱹ ᱺ typed as punctuation) is not a word.
    words = [w for t in texts for w in WORD.findall(t) if re.search(r"[ᱚ-ᱷ]", w)]
    outs = run_uccharan(sorted(set(words)), "sat", extra=("--improved",))
    reading = dict(zip(sorted(set(words)), outs))
    ok = sum(1 for w in words if reading[w] and set(unicodedata.normalize("NFD", reading[w])) - set("ˈˌ ") <= PHONEMES)
    ol = sum(1 for t in texts for c in t if "᱐" <= c <= "᱿")
    stray = sum(stray_marks(w) for w in words)
    other = sum(1 for t in texts for c in t if c.isalpha() and not ("᱐" <= c <= "᱿") and not c.isascii())
    latin = sum(1 for t in texts for w in t.split() if re.search(r"[A-Za-z]", w))
    # Both modes give the same reading.
    compat = run_uccharan(texts[:50], "sat")
    improved = run_uccharan(texts[:50], "sat", extra=("--improved",))
    res = {
        "set": args.set,
        "sentences": len(texts),
        "ol_chiki_words": len(words),
        "words_read": ok / max(1, len(words)),
        "marks_not_said_per_1000_ol_chiki_chars": 1000 * stray / max(1, ol),
        "other_script_letters": other,
        "latin_words": latin,
        "modes_identical_on_first_50": compat == improved,
    }
    print(f"sat {args.set}: {len(texts)} sentences, {len(words)} Ol Chiki words, read {100 * res['words_read']:.2f}%, "
          f"marks not said {res['marks_not_said_per_1000_ol_chiki_chars']:.2f} per 1000 Ol Chiki characters, "
          f"other-script letters {other}, Latin words {latin}, modes identical: {res['modes_identical_on_first_50']}")
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / f"sat_check_{args.set}.json").write_text(json.dumps(res, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
