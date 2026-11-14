#!/usr/bin/env python3
"""Score normalise-only mode against the gold sets in tools/eval/gold/norm.

usage: norm_eval.py [--split dev|heldout|all] [--bin PATH] [--json OUT] [LANG...]

Each row is normalised with the command-line tool (`-p text`); the output and
the expected forms are folded (case, punctuation, hyphens, nukta,
chandrabindu vs anusvara, spaces) and compared. Reports sentence exact match
and word error rate (against the closest accepted form) per class.
"""

import argparse
import json
import pathlib
import re
import subprocess
import sys
from collections import defaultdict

ROOT = pathlib.Path(__file__).resolve().parents[2]
GOLD = ROOT / "tools/eval/gold/norm"


def fold(s: str) -> str:
    s = s.lower().replace("़", "").replace("ँ", "ं")
    s = s.replace("-", " ")
    s = re.sub(r"[.,!?;:।॥\"'“”‘’()\[\]]", " ", s)
    return " ".join(s.split())


def wer(ref: str, hyp: str) -> tuple[int, int]:
    r, h = ref.split(), hyp.split()
    d = list(range(len(h) + 1))
    for i, x in enumerate(r, 1):
        prev, d[0] = d[0], i
        for j, y in enumerate(h, 1):
            cur = min(d[j] + 1, d[j - 1] + 1, prev + (x != y))
            prev, d[j] = d[j], cur
    return d[len(h)], len(r)


def options(spec: str) -> str:
    o: dict[str, str] = {}
    for part in filter(None, (p.strip() for p in spec.split(";"))):
        k, _, v = part.partition("=")
        o[k.strip()] = v.strip()
    return json.dumps(o, ensure_ascii=False)


def run(binary: str, lang: str, opts: str, lines: list[str]) -> list[str]:
    p = subprocess.run(
        [binary, "-l", lang, "-p", "text", "--norm-options", opts],
        input="\n".join(lines) + "\n",
        capture_output=True,
        text=True,
        check=True,
    )
    return p.stdout.split("\n")[: len(lines)]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--split", default="dev")
    ap.add_argument("--bin", default=str(ROOT / "target/release/uccharan"))
    ap.add_argument("--json")
    ap.add_argument("-v", "--verbose", action="store_true")
    ap.add_argument("langs", nargs="*")
    a = ap.parse_args()
    langs = a.langs or sorted(p.stem for p in GOLD.glob("*.tsv"))
    report: dict[str, dict] = {}
    for lang in langs:
        rows = []
        for line in (GOLD / f"{lang}.tsv").read_text(encoding="utf-8").splitlines():
            if not line.strip() or line.startswith("#"):
                continue
            f = line.split("\t") + [""] * 5
            split, cls, inp, exp, opt = f[:5]
            if a.split != "all" and split != a.split:
                continue
            rows.append((cls, inp, [e.strip() for e in exp.split("||")], opt))
        by_opt: dict[str, list[int]] = defaultdict(list)
        for i, r in enumerate(rows):
            by_opt[options(r[3])].append(i)
        outs = [""] * len(rows)
        for o, idx in by_opt.items():
            for i, out in zip(idx, run(a.bin, lang, o, [rows[i][1] for i in idx])):
                outs[i] = out
        per: dict[str, list[int]] = defaultdict(lambda: [0, 0, 0, 0])
        for (cls, inp, exp, _), out in zip(rows, outs):
            h = fold(out)
            best = min((wer(fold(e), h) for e in exp), key=lambda x: x[0] / max(x[1], 1))
            ok = any(fold(e) == h for e in exp)
            for k in (cls, "ALL"):
                per[k][0] += ok
                per[k][1] += 1
                per[k][2] += best[0]
                per[k][3] += best[1]
            if a.verbose and not ok:
                print(f"[{lang}/{cls}] {inp}\n   got: {out}\n  want: {exp[0]}")
        report[lang] = {
            k: {"exact": v[0], "n": v[1], "wer": round(100 * v[2] / max(v[3], 1), 2)}
            for k, v in sorted(per.items())
        }
        allr = report[lang]["ALL"]
        print(f"{lang} [{a.split}]: exact {allr['exact']}/{allr['n']}  WER {allr['wer']}%")
        for k, v in report[lang].items():
            if k != "ALL":
                print(f"    {k:14s} {v['exact']}/{v['n']}  WER {v['wer']}%")
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps(report, indent=1, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
