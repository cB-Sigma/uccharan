#!/usr/bin/env python3
"""Expand the hand-written Bhojpuri (or Maithili, Magahi, Dogri, Bodo, Santali, Manipuri) gold sources in
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
# Bodo: sources of words.src (see its header).
SOURCES_BRX = {
    "K": "Wiktionary Bodo (India) entry (CC BY-SA 4.0), read 2026-11-18; pronunciation hand-written",
    "U": "example word of Unicode document L2/08-197 (Government of India, 2008); pronunciation hand-written",
    "N": "Bodo numeral, Wikipedia \"Boro language (India)\" (CC BY-SA 4.0), spelt as in IndicCorp v2; pronunciation hand-written",
    "C": "Unicode CLDR common/main/brx.xml (Unicode License v3); pronunciation hand-written",
    "I": "word attested in IndicCorp v2 Bodo text (CC0); pronunciation hand-written",
}
# Santali: sources of words.src (see its header).
SOURCES_SAT = {
    "U": "Ol Chiki description (Unicode L2/05-243R; Wikipedia \"Ol Chiki script\", \"Santali language\", CC BY-SA 4.0); pronunciation hand-written",
    "N": "Santali numeral, Wikipedia \"Santali language\" (CC BY-SA 4.0); pronunciation hand-written",
    "C": "Unicode CLDR common/main/sat.xml (Unicode License v3); pronunciation hand-written",
    "I": "word attested in IndicCorp v2 Santali text (CC0); pronunciation hand-written",
}
# Manipuri: sources of words.src (see its header).
SOURCES_MNI = {
    "W": "Wiktionary Manipuri entry with a pronunciation (CC BY-SA 4.0), read 2026-11-18; pronunciation from Wiktionary",
    "B": "Wiktionary Manipuri entry (CC BY-SA 4.0), Bengali-script spelling attested in IndicGenBench Flores-IN mni (CC BY-SA 4.0); pronunciation from Wiktionary",
    "N": "Meitei numeral, Wikipedia \"Meitei language\" (CC BY-SA 4.0); pronunciation hand-written from its romanisation",
}
SOURCES_BY_LANG = {"doi": SOURCES_DOI, "brx": SOURCES_BRX, "sat": SOURCES_SAT, "mni": SOURCES_MNI}
DATE = {"doi": "2026-11-18", "brx": "2026-11-18", "sat": "2026-11-18", "mni": "2026-11-18"}
SENTENCE_SOURCE = {
    "brx": lambda k: "Wikipedia \"Boro language (India)\", UDHR article 1 (CC BY-SA 4.0); pronunciation from its Latin version" if k == 0 else "IndicCorp v2 bd (CC0), not in the brx_dev/brx_heldout/brx_flores sets; pronunciation hand-written",
    "sat": lambda k: "Wikipedia \"Santali language\", UDHR article 1 (CC BY-SA 4.0); pronunciation hand-written" if k < 2 else "IndicCorp v2 sat (CC0), not in the sat_dev/heldout/flores/tatoeba/wiki sets; pronunciation hand-written",
    "doi": lambda k: "IndicCorp v2 doi (CC0), not in the doi_dev/doi_heldout sets; pronunciation hand-written",
    "mni": lambda k: "IndicCorp v2 mni (CC0), Bengali-script line; pronunciation hand-written" if k < 17 else "Tatoeba (CC BY 2.0 FR); pronunciation hand-written" if k < 25 else "IndicCorp v2 mni (CC0), not in the mni_dev/heldout/wiki sets; pronunciation hand-written",
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
        source = SOURCES_BY_LANG[lang][src] if lang in SOURCES_BY_LANG else SOURCES[src].format(lang=lang)
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
        cat = "sentence"
        if lang == "mni":
            # Manipuri: the script of the sentence, so each script is scored on its own.
            cat = "sentence-beng" if any("ঀ" <= c <= "৿" for c in text) else "sentence-mtei"
        rows.append((f"s-{k:03d}", "sentence", text, ipa, cat, sentence_source(n), split("s" + text)))
    out = d / "gold.tsv"
    head = "\n".join(header) + "\n# Columns: id, kind, text, expected, category, source (with licence), split, date written.\n"
    date = DATE.get(lang, "2026-11-14")
    out.write_text(head + "".join("\t".join(r) + f"\t{date}\n" for r in rows), encoding="utf-8")
    for kind in ("word", "sentence"):
        n = [r for r in rows if r[1] == kind]
        print(f"{out.relative_to(HERE)}: {kind}s {len(n)}, dev {sum(r[6] == 'dev' for r in n)}, heldout {sum(r[6] == 'heldout' for r in n)}")


if __name__ == "__main__":
    main()
