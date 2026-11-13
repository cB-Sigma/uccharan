#!/usr/bin/env python3
"""Accuracy on the hand-curated gold sets in tools/eval/gold.

The gold sets do not come from any phonemizer: expected pronunciations follow
CMUdict variants and standard descriptions of English and Hindi phonology,
and the number/abbreviation sets only state how the text is *said in words*.
Every file documents its check in its header. Systems compared:

* ``reference``             — the espeak-ng you installed (optional);
* ``uccharan-compatible``   — compatible mode with reference-style number reading
                              (what a drop-in replacement produces);
* ``uccharan-default``      — compatible phonology with natural number reading
                              (the library default);
* ``uccharan-improved``     — improved mode (plus the Hinglish option for
                              romanised Hindi).

    python tools/eval/gold_eval.py --out tools/eval/results
"""

from __future__ import annotations

import argparse
import json
import re
import unicodedata
from pathlib import Path

from common import VOICES, VOWELS, load_oracle, read_tsv, run_uccharan

GOLD = Path(__file__).resolve().parent / "gold"
DATA = Path(__file__).resolve().parent / "data"

# file, language, check, title
SETS = [
    ("en_homographs.tsv", "en-us", "word", "English homographs in context"),
    ("en_names.tsv", "en-us", "skeleton", "English proper nouns and brands"),
    ("en_numbers.tsv", "en-us", "say", "English numbers, dates, money, units"),
    ("en_abbreviations.tsv", "en-us", "say", "English abbreviations and initialisms"),
    ("hi_schwa.tsv", "hi", "broad", "Hindi schwa deletion (compounds, suffixes)"),
    ("hi_nasals.tsv", "hi", "broad", "Hindi anusvara / chandrabindu"),
    ("hi_nukta.tsv", "hi", "broad", "Hindi nukta consonants"),
    ("hi_numbers.tsv", "hi", "say", "Hindi numbers, money, abbreviations"),
    ("mix_english_in_hindi.tsv", "hi", "inventory", "English words in Hindi text (stays in the Hindi phone set)"),
    ("mix_hindi_in_english.tsv", "en-us", "word-broad", "Hindi words in English text"),
    ("mix_romanised_hindi.tsv", "hi", "say-hinglish", "Romanised Hindi (experimental)"),
    # Written after the improved-mode rules were frozen; never used to adjust them.
    ("unseen/en_homographs.tsv", "en-us", "word", "UNSEEN: English homographs"),
    ("unseen/en_names.tsv", "en-us", "skeleton", "UNSEEN: English proper nouns"),
    ("unseen/en_numbers.tsv", "en-us", "say", "UNSEEN: English numbers and units"),
    ("unseen/hi_words.tsv", "hi", "broad", "UNSEEN: Hindi schwa / nasal / nukta words"),
    ("unseen/mix.tsv", "hi", "say-hinglish", "UNSEEN: Romanised Hindi"),
]
SYSTEMS = ["reference", "uccharan-compatible", "uccharan-default", "uccharan-improved"]


def rows_of(name: str) -> list[list[str]]:
    out = []
    for line in (GOLD / name).read_text(encoding="utf-8").splitlines():
        if line and not line.startswith("#"):
            out.append(line.split("\t"))
    return out


class Systems:
    def __init__(self, oracle, binary):
        self.oracle = oracle
        self.binary = binary

    def available(self) -> list[str]:
        return [s for s in SYSTEMS if s != "reference" or self.oracle is not None]

    def run(self, system: str, lang: str, texts: list[str], hinglish: bool = False) -> list[str]:
        if system == "reference":
            return [" ".join(re.sub(r"\([a-z-]+\)", "", self.oracle.phonemize(t, VOICES[lang])).split()) for t in texts]
        if system == "uccharan-compatible":
            return run_uccharan(texts, lang, normalize="espeak", binary=self.binary)
        if system == "uccharan-default":
            return run_uccharan(texts, lang, normalize="natural", binary=self.binary)
        extra = ("--improved", "--hinglish") if hinglish else ("--improved",)
        return run_uccharan(texts, lang, normalize="natural", binary=self.binary, extra=extra)


def strip_stress(s: str) -> str:
    return s.replace("ˈ", "").replace("ˌ", "")


def squash(s: str) -> str:
    return re.sub(r"[\sˈˌ.,;:!?-]", "", unicodedata.normalize("NFD", s))


def word_groups(out: str) -> list[str]:
    """Output words and joins of up to three neighbours (for names that a
    system splits)."""
    ws = out.split()
    g = list(ws)
    for n in (2, 3):
        g += ["".join(ws[i : i + n]) for i in range(len(ws) - n + 1)]
    return g


def broad(s: str) -> str:
    s = unicodedata.normalize("NFD", strip_stress(s)).replace("ː", "").replace("ʌ", "ə")
    s = re.sub(r"r(?!\.)", "ɾ", s)
    return s


def skeleton(word: str) -> tuple[str, str]:
    """(consonants, primary-stressed vowel nucleus) after a few notational merges."""
    w = unicodedata.normalize("NFD", word)
    for a, b in (("n̩", "ən"), ("l̩", "əl"), ("ɚ", "əɹ"), ("ɝ", "ɜɹ"), ("ᵻ", "ɪ"), ("ɐ", "ə"), ("ɾ", "t"), ("ʔ", "t"), ("ː", ""), ("ˌ", "")):
        w = w.replace(a, b)
    w = re.sub(r"ɔ(?![ɪɹ])", "ɑ", w)
    w = w.replace("ɜɹ", "ɜ").replace("ɜ", "ɜɹ")
    cons = "".join(c for c in w if c not in VOWELS and c != "ˈ")
    i = w.find("ˈ")
    tail = w[i + 1 :] if i >= 0 else w
    m = re.search(f"[{''.join(VOWELS)}]+", tail)
    return cons, (m.group(0) if m else "")


def check_rows(kind: str, rows: list[list[str]], lang: str, system: str, sysx: Systems, inventory: set[str] | None):
    """Yields (id, passed, got)."""
    if kind in ("word", "skeleton", "word-broad"):
        outs = sysx.run(system, lang, [r[1] for r in rows])
        for r, out in zip(rows, outs):
            expected = r[2].split("|")
            if kind == "word":
                ok = any(strip_stress(g) in expected for g in word_groups(out))
            elif kind == "word-broad":
                want = {broad(e) for e in expected}
                ok = any(broad(g) in want for g in word_groups(out))
            else:
                want = {skeleton(e) for e in expected}
                ok = any(skeleton(g) in want for g in word_groups(out))
            yield r[0], ok, out
    elif kind == "broad":
        outs = sysx.run(system, lang, [r[1] for r in rows])
        for r, out in zip(rows, outs):
            want = {broad(e) for e in r[2].split("|")}
            yield r[0], broad(out) in want, out
    elif kind in ("say", "say-hinglish"):
        hing = kind == "say-hinglish"
        outs = sysx.run(system, lang, [r[1] for r in rows], hinglish=hing)
        alts = [r[2].split("|") for r in rows]
        flat = [a for al in alts for a in al]
        said = iter(sysx.run(system, lang, flat))
        for r, out, al in zip(rows, outs, alts):
            forms = [next(said) for _ in al]
            yield r[0], squash(out) in {squash(f) for f in forms}, out
    elif kind == "inventory":
        outs = sysx.run(system, lang, [r[1] for r in rows])
        for r, out in zip(rows, outs):
            extra = sorted({c for c in unicodedata.normalize("NFD", out) if c not in inventory and c not in " ˈˌ"})
            yield r[0], not extra, out + ("   [outside the Hindi set: " + " ".join(extra) + "]" if extra else "")


def hindi_inventory(system: str, sysx: Systems) -> set[str]:
    """Phones a system uses for Hindi words: its output over Devanagari-only
    sentences of the development set."""
    texts = [t for _, t in read_tsv(DATA / "hi_dev.tsv") if not re.search(r"[A-Za-z0-9]", t)][:1500]
    inv: set[str] = set()
    for out in sysx.run(system, "hi", texts):
        inv.update(unicodedata.normalize("NFD", out))
    return inv


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--out", type=Path)
    ap.add_argument("--show", default="", help="print failures of this system (development aid)")
    ap.add_argument("--only", default="")
    ap.add_argument("--uccharan-bin")
    args = ap.parse_args()
    oracle = load_oracle()
    sysx = Systems(oracle, args.uccharan_bin)
    systems = sysx.available()
    result = {"reference": f"espeak-ng {oracle.version()}" if oracle else None, "sets": []}
    inventories = {}
    print(f"{'set':58s} {'n':>4s}  " + "  ".join(f"{s:>20s}" for s in systems))
    totals = {s: [0, 0] for s in systems}
    unseen = {s: [0, 0] for s in systems}
    for name, lang, kind, title in SETS:
        if args.only and args.only not in name:
            continue
        rows = rows_of(name)
        entry = {"file": name, "title": title, "language": lang, "check": kind, "items": len(rows), "systems": {}}
        for s in systems:
            inv = None
            if kind == "inventory":
                if s not in inventories:
                    inventories[s] = hindi_inventory(s, sysx)
                inv = inventories[s]
            res = list(check_rows(kind, rows, lang, s, sysx, inv))
            passed = sum(ok for _, ok, _ in res)
            entry["systems"][s] = {"passed": passed, "accuracy": passed / len(rows), "failed_ids": [i for i, ok, _ in res if not ok]}
            bucket = unseen if name.startswith("unseen/") else totals
            bucket[s][0] += passed
            bucket[s][1] += len(rows)
            if args.show == s:
                for (i, ok, got), r in zip(res, rows):
                    if not ok:
                        print(f"      FAIL {i}: {r[1]}  →  {got}   (want {r[2] if len(r) > 2 else ''})")
        result["sets"].append(entry)
        print(f"{title:58s} {len(rows):4d}  " + "  ".join(f"{entry['systems'][s]['passed']:4d} ({100 * entry['systems'][s]['accuracy']:5.1f}%)      " for s in systems))
    result["total_development_sets"] = {s: {"passed": p, "items": n, "accuracy": p / max(1, n)} for s, (p, n) in totals.items()}
    result["total_unseen_sets"] = {s: {"passed": p, "items": n, "accuracy": p / max(1, n)} for s, (p, n) in unseen.items()}
    for label, t in (("all development sets", totals), ("all unseen sets", unseen)):
        print(f"{label:58s} {max(n for _, n in t.values()):4d}  " + "  ".join(f"{p:4d} ({100 * p / max(1, n):5.1f}%)      " for p, n in t.values()))
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / "gold.json").write_text(json.dumps(result, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
