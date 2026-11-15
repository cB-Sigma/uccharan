#!/usr/bin/env python3
"""Write ``crates/uccharan/lang/numbers/ord-<lang>.txt``: the ordinal rule sets
of a language from Unicode CLDR's rule-based number formats
(``common/rbnf/<lang>.xml``, Unicode License v3), for the ordinal words of
normalise-only mode and improved mode (``uccharan::rbnf``, ``[ordinals]`` in
the language file).

    python tools/cldr_ordinals.py de:spellout-ordinal-r,spellout-ordinal-n ...

Kept: the named public sets and every set they use (cardinal sets
included). Left out: rules for negative numbers, fractions and numbers
written with digits (``-x``, ``x.x``, ``=#,##0=``), which the engine does
not read. Norwegian Bokmål reads ``no.xml``.
"""

from __future__ import annotations

import re
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
URL = "https://raw.githubusercontent.com/unicode-org/cldr/main/common/rbnf/{}.xml"
FILE = {"nb": "no"}


def sets_of(rules: str) -> dict[str, list[str]]:
    sets: dict[str, list[str]] = {}
    cur = None
    for line in rules.splitlines():
        line = line.strip()
        if not line:
            continue
        m = re.match(r"^(%%?[\w-]+):$", line)
        if m:
            cur = m.group(1)
            sets[cur] = []
        elif cur:
            sets[cur].append(line)
    return sets


def main() -> int:
    for arg in sys.argv[1:]:
        lang, names = arg.split(":")
        with urllib.request.urlopen(URL.format(FILE.get(lang, lang)), timeout=60) as r:
            xml = r.read().decode("utf-8")
        rules = re.search(r"<rbnfRules><!\[CDATA\[(.*?)\]\]></rbnfRules>", xml, re.S).group(1)
        sets = sets_of(rules)
        want = ["%" + n for n in names.split(",")]
        keep: list[str] = []
        todo = list(want)
        while todo:
            s = todo.pop()
            if s in keep or s not in sets:
                continue
            keep.append(s)
            for line in sets[s]:
                todo.extend(re.findall(r"(%%?[\w-]+)", line.split(":", 1)[-1]))
        out = [
            f"# Ordinal number words of '{lang}': rule sets from Unicode CLDR",
            f"# common/rbnf/{FILE.get(lang, lang)}.xml (Unicode License v3, see NOTICE), written out by",
            "# tools/cldr_ordinals.py.",
        ]
        for s in sorted(keep, key=lambda x: (x.startswith("%%"), x)):
            out.append(f"{s}:")
            for line in sets[s]:
                head = line.split(":", 1)[0]
                if head in ("-x", "x.x", "Inf", "NaN") or "#,##0" in line:
                    continue
                out.append(line)
        path = ROOT / "crates/uccharan/lang/numbers" / f"ord-{lang}.txt"
        path.write_text("\n".join(out) + "\n", encoding="utf-8")
        print(path, len(keep), "sets")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
