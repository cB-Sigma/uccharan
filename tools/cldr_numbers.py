#!/usr/bin/env python3
"""Print the cardinal number words 0-99 and the scale words of a language
from Unicode CLDR's rule-based number format data (``common/rbnf/<lang>.xml``,
Unicode License v3), as ``[numbers]`` lines for a ``.lang`` file.

    python tools/cldr_numbers.py bn > /tmp/bn_numbers.txt

The output is a starting point: the words are then checked against the
pronunciations voices expect and adjusted by hand in the ``.lang`` file.
"""

from __future__ import annotations

import re
import sys
import urllib.request

URL = "https://raw.githubusercontent.com/unicode-org/cldr/main/common/rbnf/{}.xml"


def main() -> int:
    lang = sys.argv[1]
    with urllib.request.urlopen(URL.format(lang), timeout=60) as r:
        xml = r.read().decode("utf-8")
    def section(name: str) -> str:
        return xml.split(f"%{name}:", 1)[1].split("\n%", 1)[0] if f"%{name}:" in xml else ""

    body = section("spellout-cardinal")
    if not re.search(r"^\s*1: [^=<>]+;", body, re.M):
        body = section("spellout-numbering")
    print(f"# number words from Unicode CLDR common/rbnf/{lang}.xml (Unicode License v3)")
    for line in body.splitlines():
        m = re.fullmatch(r"(\d+): (.*);", line.strip())
        if not m:
            continue
        n, rule = int(m.group(1)), m.group(2)
        # Sub-rule references (">%%name>", "=%name=", "<<", ">>") and the "|" before a suffix are dropped.
        word = re.sub(r"\[ ?>>\]|>?%%?[-a-z]+>?|>>|<<|=%[-a-z]+=|[\[\]=|]", "", rule).strip()
        if n < 100 and word:
            print(f"{n} {word}")
        elif word:
            # Larger values need a person to decide (scale word, combining
            # forms): printed as comments.
            print(f"# {n}: {rule}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
