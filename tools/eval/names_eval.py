#!/usr/bin/env python3
"""Accuracy on the names gold set (tools/eval/gold/names.tsv), per origin and
per split.

The set holds about 380 personal names of many origins, each in a short
English sentence. References are CMUdict's ARPABET (BSD-2-Clause) where the
name is in CMUdict, and hand-written IPA from standard pronunciation
otherwise; several alternatives are accepted. The check compares the
consonant skeleton and the primary-stressed vowel (see gold_eval.skeleton).
The `dev` split may be used to adjust rules; the `heldout` split is not.

    python tools/eval/names_eval.py --split heldout --out tools/eval/results
"""

from __future__ import annotations

import argparse
import json
import re
from collections import defaultdict
from pathlib import Path

from common import load_oracle
from gold_eval import Systems, skeleton, word_groups

GOLD = Path(__file__).resolve().parent / "gold" / "names.tsv"

ARPA = {
    "AA": "ɑ", "AE": "æ", "AH": "ʌ", "AO": "ɔ", "AW": "aʊ", "AY": "aɪ", "B": "b", "CH": "tʃ", "D": "d", "DH": "ð", "EH": "ɛ",
    "ER": "ɜɹ", "EY": "eɪ", "F": "f", "G": "ɡ", "HH": "h", "IH": "ɪ", "IY": "i", "JH": "dʒ", "K": "k", "L": "l", "M": "m", "N": "n",
    "NG": "ŋ", "OW": "oʊ", "OY": "ɔɪ", "P": "p", "R": "ɹ", "S": "s", "SH": "ʃ", "T": "t", "TH": "θ", "UH": "ʊ", "UW": "u", "V": "v",
    "W": "w", "Y": "j", "Z": "z", "ZH": "ʒ",
}


def arpabet_to_ipa(arpa: str) -> str:
    out = []
    for ph in arpa.split():
        m = re.fullmatch(r"([A-Z]+)([012]?)", ph)
        if not m:
            continue
        base, stress = m.groups()
        sym = ARPA.get(base, "")
        if base == "AH" and stress == "0":
            sym = "ə"
        if base == "ER" and stress == "0":
            sym = "ɚ"
        out.append(("ˈ" if stress == "1" else "") + sym)
    return "".join(out)


def references(field: str) -> list[str]:
    refs: list[str] = []
    cmu = False
    for part in field.split("|"):
        if part.startswith("cmu:"):
            cmu = True
            part = part[4:]
        refs.append(arpabet_to_ipa(part) if cmu else part)
    return refs


def rows(split: str) -> list[list[str]]:
    out = []
    for line in GOLD.read_text(encoding="utf-8").splitlines():
        if line and not line.startswith("#"):
            f = line.split("\t")
            if split in ("all", f[5]):
                out.append(f)
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--split", default="heldout", choices=["dev", "heldout", "all"])
    ap.add_argument("--show", default="", help="print failures of this system")
    ap.add_argument("--origin", default="")
    ap.add_argument("--out", type=Path)
    ap.add_argument("--uccharan-bin")
    args = ap.parse_args()
    sysx = Systems(load_oracle(), args.uccharan_bin)
    systems = [s for s in sysx.available() if s != "uccharan-default"]
    data = rows(args.split)
    if args.origin:
        data = [r for r in data if r[3] == args.origin]
    res: dict = {"split": args.split, "items": len(data), "systems": {}}
    table: dict = defaultdict(lambda: defaultdict(lambda: [0, 0]))
    for s in systems:
        outs = sysx.run(s, "en-us", [r[1] for r in data])
        fails = []
        for r, out in zip(data, outs):
            want = {skeleton(x) for x in references(r[2])}
            ok = any(skeleton(g) in want for g in word_groups(out))
            for key in (r[3], "in-cmudict" if "cmudict" in r[4] else "not-in-cmudict", "all"):
                table[key][s][0] += ok
                table[key][s][1] += 1
            if not ok:
                fails.append((r[0], r[1], out, r[2]))
        res["systems"][s] = {"failed": [f[0] for f in fails]}
        if args.show == s:
            for f in fails:
                print("   FAIL", f[0], "|", f[2], "| want", f[3][:80])
    order = sorted(k for k in table if k not in ("all", "in-cmudict", "not-in-cmudict")) + ["in-cmudict", "not-in-cmudict", "all"]
    print(f"{'names (' + args.split + ')':28s} {'n':>4s}  " + "  ".join(f"{s:>20s}" for s in systems))
    res["by_group"] = {}
    for k in order:
        n = table[k][systems[0]][1]
        res["by_group"][k] = {s: {"passed": table[k][s][0], "items": n, "accuracy": table[k][s][0] / max(1, n)} for s in systems}
        print(f"{k:28s} {n:4d}  " + "  ".join(f"{table[k][s][0]:4d} ({100 * table[k][s][0] / max(1, n):5.1f}%)      " for s in systems))
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / f"names_{args.split}.json").write_text(json.dumps(res, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
