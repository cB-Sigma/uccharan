#!/usr/bin/env python3
"""Print `@word` lines for a `.lang` file's [words] section: the reference
reading of each number word 0-99 whose own reading differs.

    python tools/oracle_numbers.py pa [--max 99]

Needs the reference phonemizer (tools/oracle) and a release build of the
`uccharan` command-line program. The lines are pasted under [words].
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools" / "eval"))
sys.path.insert(0, str(ROOT / "tools" / "oracle"))
from common import VOICES, load_oracle  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("lang")
    ap.add_argument("--max", type=int, default=99)
    ap.add_argument("--extra", nargs="*", default=["100", "1000", "100000", "10000000"])
    args = ap.parse_args()
    text = (ROOT / "crates" / "uccharan" / "lang" / f"{args.lang}.lang").read_text(encoding="utf-8")
    sec = text.split("[numbers]", 1)[1].split("\n[", 1)[0]
    words: dict[str, str] = {}
    for line in sec.splitlines():
        m = re.match(r"^(\d+|scale\d+)\s+(\S+)\s*$", line.strip())
        if m:
            words[m.group(1)] = m.group(2)
    o = load_oracle()
    nums = [str(n) for n in range(args.max + 1)] + args.extra
    binary = ROOT / "target" / "release" / "uccharan"
    ours = subprocess.run([str(binary), "-l", args.lang, "--no-punctuation"], input="\n".join(nums) + "\n",
                          capture_output=True, text=True, check=True).stdout.split("\n")
    for n, mine in zip(nums, ours):
        ref = " ".join(re.sub(r"\([a-z-]+\)", "", o.phonemize(n, VOICES[args.lang])).split())
        word = words.get(n)
        if ref != mine.strip() and word and len(ref.split()) == 1:
            print(f"@{word} {ref}")
        elif ref != mine.strip():
            print(f"# {n}: reference {ref!r}, ours {mine.strip()!r}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
