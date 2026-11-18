#!/usr/bin/env python3
"""Kashmiri coverage check on the sentence sets.

espeak-ng has no Kashmiri voice, so the sentence sets of Kashmiri cannot be
scored against a reference. This script measures what can be measured
without one, on any of tools/eval/data/ks_{dev,heldout,wiki}.tsv
(Perso-Arabic script), for uccharan's reading (both modes give the same
reading):

- words: Perso-Arabic words in the set (letters with their marks), and the
  share read to a non-empty pronunciation made only of the phonemes of
  lang/ks.lang that has a vowel;
- words written with vowel marks and without any (the latter read with the
  inherent a and Urdu-type deletion), and the share of each;
- other characters: letters outside the Arabic script, and words with Latin
  letters.

The held-out sets are only measured, never used to change the rules.

    python tools/eval/ks_check.py --set heldout --out tools/eval/results
"""

from __future__ import annotations

import argparse
import json
import re
import unicodedata
from pathlib import Path

from common import run_uccharan

HERE = Path(__file__).resolve().parent
PHONEMES = set("aeiouəɨɔptkbdɡmnŋɲɳshrljʋxqfɣzʃʒʈɖɽʰʲː̃")
VOWELS = set("aeiouəɨɔ")


def _range(a: int, b: int) -> str:
    return f"{chr(a)}-{chr(b)}"


# A word: letters of the Arabic block (with marks), at least one letter
# (tatweel and hamza alone are not letters here).
WORD = re.compile("[" + _range(0x620, 0x65F) + _range(0x670, 0x6D3) + _range(0x6D5, 0x6ED) + _range(0x750, 0x77F) + "]+")
LETTER = re.compile("[" + _range(0x620, 0x620) + _range(0x622, 0x63F) + _range(0x641, 0x64A) + _range(0x66E, 0x6D3) + _range(0x750, 0x77F) + "]")
# A vowel mark, or a letter that only a marked spelling uses.
MARK = re.compile("[" + _range(0x64E, 0x65F) + chr(0x670) + chr(0x672) + chr(0x673) + chr(0x6C4) + chr(0x6C6) + chr(0x620) + chr(0x6CE) + "]")


def readable(p: str) -> bool:
    s = set(unicodedata.normalize("NFD", p)) - set("ˈˌ ")
    return bool(s) and s <= PHONEMES and bool(s & VOWELS)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--set", default="heldout")
    ap.add_argument("--out", type=Path)
    args = ap.parse_args()
    rows = [l.split("\t", 1) for l in (HERE / "data" / f"ks_{args.set}.tsv").read_text(encoding="utf-8").splitlines() if "\t" in l]
    texts = [t for _, t in rows]
    words = [w for t in texts for w in WORD.findall(t) if LETTER.search(w) and w != "ء"]
    vocab = sorted(set(words))
    reading = dict(zip(vocab, run_uccharan(vocab, "ks", extra=("--improved",))))
    ok = sum(1 for w in words if readable(reading[w]))
    marked = [w for w in words if MARK.search(w)]
    other = sum(1 for t in texts for c in t if c.isalpha() and not (0x600 <= ord(c) <= 0x6FF or 0x750 <= ord(c) <= 0x77F))
    latin = sum(1 for t in texts for w in t.split() if re.search(r"[A-Za-z]", w))
    compat = run_uccharan(texts[:50], "ks")
    improved = run_uccharan(texts[:50], "ks", extra=("--improved",))
    res = {
        "set": args.set,
        "sentences": len(texts),
        "words": len(words),
        "words_read": ok / max(1, len(words)),
        "words_with_vowel_marks": len(marked) / max(1, len(words)),
        "other_script_letters": other,
        "latin_words": latin,
        "modes_identical_on_first_50": compat == improved,
    }
    print(f"ks {args.set}: {len(texts)} sentences, {len(words)} words, read {100 * res['words_read']:.2f}%, "
          f"with vowel marks {100 * res['words_with_vowel_marks']:.1f}%, other-script letters {other}, "
          f"Latin words {latin}, modes identical: {res['modes_identical_on_first_50']}")
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / f"ks_check_{args.set}.json").write_text(json.dumps(res, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
