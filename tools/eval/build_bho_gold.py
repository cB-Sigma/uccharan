#!/usr/bin/env python3
"""Expand the hand-written Bhojpuri (or Maithili, Magahi, Dogri) gold sources in
gold/<lang>/{words,sentences}.src into gold/<lang>/gold.tsv with a fixed
dev/held-out split (by a hash of the item, decided before any tuning).

    python tools/eval/build_bho_gold.py bho
"""

from __future__ import annotations

import hashlib
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SOURCES = {
    "F": "FLORES-200 {lang}_Deva (CC BY-SA 4.0), word attested; pronunciation hand-written",
    "L": "Grierson, Linguistic Survey of India vol. V part II (1903, public domain), or common word; pronunciation hand-written",
    "S": "language description example word; pronunciation hand-written",
    "G": "grammar description (see header); pronunciation hand-written",
    "W": "WikiPron {lang}_deva_narrow (Wiktionary, CC BY-SA 4.0); pronunciation from Wiktionary",
    "K": "Wiktionary {lang} entry via kaikki.org (CC BY-SA 4.0), fetched 2026-11-14; pronunciation from the romanisation, checked by hand",
}
# Dogri: sources of words.src (see its header).
SOURCES_DOI = {
    "R": "example word of the Dogri phonology descriptions (Bahri 2001; Ghai 1991); pronunciation hand-written",
    "N": "Dogri numeral, Bahri (2002) in E. Chan's numeral tables; pronunciation hand-written",
    "C": "Unicode CLDR common/main/doi.xml (Unicode License v3); pronunciation hand-written",
    "I": "word attested in IndicCorp v2 Dogri text (CC0); pronunciation hand-written",
}
DATE = {"doi": "2026-11-18"}
SENTENCE_SOURCE = {
    "doi": lambda k: "IndicCorp v2 doi (CC0), not in the doi_dev/doi_heldout sets; pronunciation hand-written",
    "mag": lambda k: "FLORES-200 mag_Deva (CC BY-SA 4.0); pronunciation hand-written" if 108 <= k < 132 else "written for this set (see header)",
}


def split(key: str) -> str:
    return "dev" if int(hashlib.sha1(key.encode()).hexdigest(), 16) % 2 == 0 else "heldout"


def main() -> None:
    lang = sys.argv[1] if len(sys.argv) > 1 else "bho"
    d = HERE / "gold" / lang
    rows = []
    header = []
    for line in (d / "words.src").read_text(encoding="utf-8").splitlines():
        if line.startswith("#"):
            header.append(line)
            continue
        if not line.strip():
            continue
        word, ipa, cat, src = line.split()
        source = SOURCES_DOI[src] if lang == "doi" else SOURCES[src].format(lang=lang)
        rows.append((f"w-{word}", "word", word, ipa, cat, source, split("w" + word)))
    sentence_source = SENTENCE_SOURCE.get(lang, lambda k: "written for this set (see header)")
    n = -1
    for k, line in enumerate((d / "sentences.src").read_text(encoding="utf-8").splitlines()):
        if line.startswith("#"):
            header.append(line)
            continue
        if not line.strip():
            continue
        n += 1
        text, ipa = line.split("\t")
        rows.append((f"s-{k:03d}", "sentence", text, ipa, "sentence", sentence_source(n), split("s" + text)))
    out = d / "gold.tsv"
    head = "\n".join(header) + "\n# Columns: id, kind, text, expected, category, source (with licence), split, date written.\n"
    date = DATE.get(lang, "2026-11-14")
    out.write_text(head + "".join("\t".join(r) + f"\t{date}\n" for r in rows), encoding="utf-8")
    for kind in ("word", "sentence"):
        n = [r for r in rows if r[1] == kind]
        print(f"{out.relative_to(HERE)}: {kind}s {len(n)}, dev {sum(r[6] == 'dev' for r in n)}, heldout {sum(r[6] == 'heldout' for r in n)}")


if __name__ == "__main__":
    main()
