#!/usr/bin/env python3
"""Write ``crates/uccharan/lang/numbers/<lang>.txt``: the spellout rule sets of
a language from Unicode CLDR's rule-based number formats
(``common/rbnf/<lang>.xml``, Unicode License v3), for the written number
words of normalise-only mode (``uccharan::rbnf``).

    python tools/cldr_rbnf.py ta te kn ...

Kept: ``%spellout-numbering``, ``%spellout-cardinal`` and the private sets
they use. Added: three header lines with the words for the decimal sign and
the minus sign (from the CLDR rules) and for the percent sign (below).
Nepali is written with the traditional long vowels (तीन, बीस, पच्चीस), which
the dictionaries of the Nepal Academy use. Malayalam gets a space before a
tens word after -ത്തി/-റ്റി, where CLDR writes one long word. Telugu exact
thousands, lakhs and crores end in -లు (ఐదు వేలు), the oblique form only
before more.
"""

from __future__ import annotations

import re
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
URL = "https://raw.githubusercontent.com/unicode-org/cldr/main/common/rbnf/{}.xml"
PERCENT = {
    "bn": "শতাংশ", "gu": "ટકા", "kn": "ಪ್ರತಿಶತ", "ml": "ശതമാനം", "mr": "टक्के", "ne": "प्रतिशत",
    "or": "ପ୍ରତିଶତ", "pa": "ਪ੍ਰਤੀਸ਼ਤ", "ta": "சதவீதம்", "te": "శాతం", "ur": "فیصد",
}
POINT = {"bn": "দশমিক", "gu": "દશાંશ", "mr": "दशांश", "or": "ଦଶମିକ", "pa": "ਦਸ਼ਮਲਵ", "te": "దశాంశం", "kn": "ದಶಮಾಂಶ", "ur": "اعشاریہ"}


def main() -> int:
    for lang in sys.argv[1:]:
        with urllib.request.urlopen(URL.format(lang), timeout=60) as r:
            xml = r.read().decode("utf-8")
        rules = re.search(r"<rbnfRules><!\[CDATA\[(.*?)\]\]></rbnfRules>", xml, re.S).group(1)
        sets: dict[str, list[str]] = {}
        cur = None
        for line in rules.split("\n"):
            s = line.strip()
            m = re.match(r"^(%%?[\w-]+):$", s)
            if m:
                cur = m.group(1)
                sets[cur] = []
            elif cur and s:
                sets[cur].append(s)
        need = ["%spellout-numbering", "%spellout-cardinal"]
        i = 0
        while i < len(need):
            for ln in sets[need[i]]:
                for n in re.findall(r"%%?[\w-]+", ln):
                    if n in sets and n not in need:
                        need.append(n)
            i += 1
        everything = "\n".join(ln for n in need for ln in sets[n])
        point = re.search(r"^x\.x: << (.*?) >>;", everything, re.M)
        minus = re.search(r"^-x: (.*?) >>;", everything, re.M)
        out = [
            f"# Number words of '{lang}': rule sets from Unicode CLDR common/rbnf/{lang}.xml",
            "# (Unicode License v3, see NOTICE), written out by tools/cldr_rbnf.py.",
            f"point: {POINT.get(lang) or (point.group(1) if point else '')}",
            f"minus: {minus.group(1) if minus else ''}",
            f"percent: {PERCENT.get(lang, '')}",
        ]
        for n in need:
            out.append(f"{n}:")
            for ln in sets[n]:
                if not re.match(r"^\d", ln):
                    continue
                if lang == "ne":
                    ln = re.sub(r"िस(?=[;\[ ]|$)", "ीस", ln).replace("तिन", "तीन").replace("शुन्य", "शून्य")
                if lang == "ml" and n == "%%spellout-cardinal-compound" and ln.endswith(": =%spellout-cardinal=;"):
                    ln = ln.replace(": =", ": ' =")
                if lang == "te":
                    ln = re.sub(r"(వేల|లక్షల|కోట్ల)\[ >>\]", r"\1[ >>|ు]", ln)
                out.append(ln)
        path = ROOT / "crates/uccharan/lang/numbers" / f"{lang}.txt"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("\n".join(out) + "\n", encoding="utf-8")
        print(path, len(out), "lines")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
