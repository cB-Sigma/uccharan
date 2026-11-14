#!/usr/bin/env python3
"""Held-out scores and speed of the transliteration packs.

    heldout.py --data DIR [--langs hi,mr,...] [--out tools/eval/results/translit_heldout.json]

DIR is the directory `prep.py` wrote (`<lang>/test_dak.tsv`, `test_ak.tsv`,
`test_ak_native.tsv`, `test_ak_ne.tsv`) with the Dakshina tarball's
`<lang>/romanized/*.rejoined.test.*` files under
`DIR/raw/dakshina/dakshina_dataset_v1.0/`. Scores come from `score.py`;
speed is wall time of the `uccharan` command over the same files, one thread.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def score(args: list[str]) -> dict[str, str]:
    p = subprocess.run([sys.executable, str(HERE / "score.py"), *args], capture_output=True, text=True, encoding="utf8")
    if p.returncode != 0:
        sys.exit(p.stderr)
    return dict(re.findall(r"(\w+)=(\S+)", p.stdout))


def timed(cmd: list[str], text: str) -> float:
    t = time.perf_counter()
    subprocess.run(cmd, input=text, capture_output=True, text=True, encoding="utf8", check=True)
    return time.perf_counter() - t


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--data", required=True)
    ap.add_argument("--langs", default="hi,mr,bn,ta,te,gu,pa,kn,ml")
    ap.add_argument("--bin", default=str(ROOT / "target/release/uccharan"))
    ap.add_argument("--out", default=str(ROOT / "tools/eval/results/translit_heldout.json"))
    ap.add_argument("--speed-only", action="store_true", help="measure speed again and keep the scores of --out")
    ap.add_argument("--merge", action="store_true", help="keep the other languages of --out")
    a = ap.parse_args()
    data = Path(a.data)
    out: dict[str, dict[str, object]] = {}
    if (a.speed_only or a.merge) and Path(a.out).exists():
        out = json.loads(Path(a.out).read_text(encoding="utf8"))
    for la in a.langs.split(","):
        pack = ROOT / f"data/packs/xlit-{la}.ucd"
        base = ["--bin", a.bin, "--lang", la, "--pack", str(pack)]
        r: dict[str, object] = {"pack_bytes": pack.stat().st_size}
        d = data / f"raw/dakshina/dakshina_dataset_v1.0/{la}/romanized/{la}.romanized.rejoined.test"
        sent = [f"{d}.roman.txt", f"{d}.native.txt"]
        if a.speed_only:
            r = dict(out.get(la, r))
        for name in () if a.speed_only else ("test_dak", "test_ak", "test_ak_native", "test_ak_ne"):
            r[name] = score(["words", str(data / la / f"{name}.tsv"), *base])
        if not a.speed_only:
            r["sentences"] = score(["sentences", *sent, *base])
            r["sentences_no_context"] = score(["sentences", *sent, *base, "--no-context"])
            r["sentences_no_pack"] = score(["sentences", *sent, "--bin", a.bin, "--lang", la, "--no-pack"])
            if la == "hi":
                gold = str(ROOT / "tools/eval/gold/hinglish/test.tsv")
                r["hinglish"] = score(["hinglish", gold, *base])
                r["hinglish_no_context"] = score(["hinglish", gold, *base, "--no-context"])
                r["hinglish_no_pack"] = score(["hinglish", gold, "--bin", a.bin, "--lang", la, "--no-pack"])
        # Speed: running text from a cold start, the same text again in the
        # same process (words known), and words that all need the model.
        cmd = [a.bin, "-l", la, "-p", "native", "--pack", str(pack), "--translit-options", '{"english":"convert"}']
        text = open(sent[0], encoding="utf8").read()
        n = len(text.split())
        once = min(timed(cmd, text) for _ in range(3))
        # Five more passes over the same text in the same process: the words are known by then.
        six = min(timed(cmd, text * 6) for _ in range(3))
        words = "".join(l.split("\t")[0] + "\n" for l in open(data / la / "test_dak.tsv", encoding="utf8"))
        new = min(timed(cmd, words) for _ in range(2))
        r["speed_words_per_s"] = {
            "running_text_first_pass": round(n / once),
            "running_text_again": round(5 * n / max(six - once, 1e-6)),
            "new_words": round(len(words.split()) / new),
        }
        out[la] = r
        print(la, json.dumps(r, ensure_ascii=False), flush=True)
    Path(a.out).write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n", encoding="utf8")


if __name__ == "__main__":
    main()
