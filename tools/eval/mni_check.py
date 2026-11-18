#!/usr/bin/env python3
"""Manipuri (Meitei) coverage check on the sentence sets.

espeak-ng has no Meitei voice and reads Meetei Mayek letters as their code
points, so the sentence sets of Manipuri cannot be scored against a
reference. This script measures what can be measured without one, on any of
tools/eval/data/mni_{dev,heldout,wiki}.tsv (Meetei Mayek) and
mni_{flores,flores_dev}.tsv (Bengali script), for uccharan's reading (both
modes give the same reading):

- words: Meetei Mayek or Bengali-script words in the set, and the share read
  to a non-empty pronunciation made only of the phonemes of lang/mni.lang;
- Bengali script, with ``--indiccorp`` (AI4Bharat IndicCorp v2 ``mni.txt``,
  CC0, Meetei Mayek text): the share of Bengali-script word tokens whose
  reading is the reading of a Meetei Mayek word that occurs at least twice in
  that text, i.e. that the Bengali-script spelling is read as a word that
  Meetei Mayek writers spell (the IndicCorp text is a list of spellings
  here; nothing is tuned on it);
- other characters: letters outside the set's script, ASCII and common
  punctuation, and words with Latin letters.

The held-out sets are only measured, never used to change the rules.

    python tools/eval/mni_check.py --set flores --indiccorp mni.txt --out tools/eval/results
"""

from __future__ import annotations

import argparse
import collections
import json
import re
import unicodedata
from pathlib import Path

from common import run_uccharan

HERE = Path(__file__).resolve().parent
PHONEMES = set("aeiouɐptkbdɡmnŋshɾljwʰʱʃʒʈɖɳɲʂ")
MAYEK = re.compile(r"[ꯀ-꯭ꫠ-꫶]+")
BENGALI = re.compile(r"[ঀ-৥ৰ-৿]+")
LETTER_MAYEK = re.compile(r"[ꯀ-ꯪꫠ-ꫯꫵ]")
LETTER_BENGALI = re.compile("[অ-হৎড়-য়ৰৱ]")


def readable(p: str) -> bool:
    return bool(p) and set(unicodedata.normalize("NFD", p)) - set("ˈˌ ") <= PHONEMES


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--set", default="heldout")
    ap.add_argument("--indiccorp", type=Path, help="IndicCorp v2 mni.txt (Meetei Mayek), for the Bengali-script sets")
    ap.add_argument("--out", type=Path)
    args = ap.parse_args()
    rows = [l.split("\t", 1) for l in (HERE / "data" / f"mni_{args.set}.tsv").read_text(encoding="utf-8").splitlines() if "\t" in l]
    texts = [unicodedata.normalize("NFC", t) for _, t in rows]
    beng = sum(len(BENGALI.findall(t)) for t in texts) > sum(len(MAYEK.findall(t)) for t in texts)
    word_re, letter_re = (BENGALI, LETTER_BENGALI) if beng else (MAYEK, LETTER_MAYEK)
    words = [w for t in texts for w in word_re.findall(t) if letter_re.search(w)]
    vocab = sorted(set(words))
    reading = dict(zip(vocab, run_uccharan(vocab, "mni", extra=("--improved",))))
    ok = sum(1 for w in words if readable(reading[w]))
    script_chars = (lambda c: "ঀ" <= c <= "৿") if beng else (lambda c: "ꯀ" <= c <= "꯿" or "ꫠ" <= c <= "꫿")
    other = sum(1 for t in texts for c in t if c.isalpha() and not script_chars(c) and not c.isascii())
    latin = sum(1 for t in texts for w in t.split() if re.search(r"[A-Za-z]", w))
    compat = run_uccharan(texts[:50], "mni")
    improved = run_uccharan(texts[:50], "mni", extra=("--improved",))
    res = {
        "set": args.set,
        "script": "Bengali" if beng else "Meetei Mayek",
        "sentences": len(texts),
        "words": len(words),
        "words_read": ok / max(1, len(words)),
        "other_script_letters": other,
        "latin_words": latin,
        "modes_identical_on_first_50": compat == improved,
    }
    line = (f"mni {args.set} ({res['script']}): {len(texts)} sentences, {len(words)} words, read {100 * res['words_read']:.2f}%, "
            f"other-script letters {other}, Latin words {latin}, modes identical: {res['modes_identical_on_first_50']}")
    if beng and args.indiccorp:
        counts = collections.Counter(MAYEK.findall(args.indiccorp.read_text(encoding="utf-8")))
        attested = sorted(w for w, n in counts.items() if n >= 2 and LETTER_MAYEK.search(w))
        readings = set(run_uccharan(attested, "mni", extra=("--improved",)))
        hit = sum(1 for w in words if reading[w] in readings)
        res["read_as_attested_meetei_mayek_word"] = hit / max(1, len(words))
        res["attested_meetei_mayek_words"] = len(attested)
        line += f"; read as an attested Meetei Mayek word {100 * res['read_as_attested_meetei_mayek_word']:.2f}%"
    print(line)
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / f"mni_check_{args.set}.json").write_text(json.dumps(res, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
