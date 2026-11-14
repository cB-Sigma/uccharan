#!/usr/bin/env python3
"""Rebuild crates/uccharan/src/en/arabic_names.txt: Arabic given names in
English transliteration, for the improved mode's names reader.

Source: Wikidata (CC0). The English labels and English aliases of items that
are a male or female given name (Q12308941, Q11879590) and either have
"language of work or name" Arabic (P407 = Q13955) or a native label (P1705)
in Arabic or Egyptian Arabic. Left out: names that are also Western given
names on Wikidata (P407 English, French, German, Spanish or Italian), names
whose lower-case or capitalised form is in misaki's gold dictionary of
common English words (English already has a reading for those, e.g. *Said*,
*Adam*), a short hand list of Western names that came in through native
labels, and names shorter than three letters.

It also rebuilds crates/uccharan/src/en/horn_names.txt: the words of the
English labels of people (Q5) with Eritrean or Ethiopian citizenship (P27 =
Q986, Q115) on Wikidata, leaving out words CMUdict has, the Arabic list's
names, Western given names and misaki's common English words.

    python tools/arabic_names.py            # query Wikidata, write both lists
"""

from __future__ import annotations

import csv
import io
import json
import re
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "crates" / "uccharan" / "src" / "en" / "arabic_names.txt"
HORN = ROOT / "crates" / "uccharan" / "src" / "en" / "horn_names.txt"
ENDPOINT = "https://query.wikidata.org/sparql"
GIVEN = ("Q12308941", "Q11879590")
WESTERN_LANGS = ("Q1860", "Q150", "Q188", "Q1321", "Q652")
NOT_ARABIC = {
    "cynthia", "helene", "joelle", "mila", "aron", "ord", "tara", "adela", "lorine", "nalini", "ilan", "burak",
    "ons", "ferit", "bekir", "hulusi", "sevde", "senai", "kulu",
}


def query(q: str) -> list[str]:
    url = ENDPOINT + "?" + urllib.parse.urlencode({"query": q})
    req = urllib.request.Request(url, headers={"Accept": "text/csv", "User-Agent": "uccharan-data/0.2"})
    with urllib.request.urlopen(req, timeout=300) as r:
        text = r.read().decode("utf-8")
    return [row[-1] for row in csv.reader(io.StringIO(text))][1:]


def names(rows: list[str]) -> set[str]:
    return {n.lower() for n in rows if re.fullmatch(r"[A-Z][a-z]+", n)}


def main() -> None:
    arabic: set[str] = set()
    western: set[str] = set()
    for t in GIVEN:
        for prop in ("rdfs:label", "skos:altLabel"):
            arabic |= names(query(f'SELECT ?l WHERE {{ ?n wdt:P31 wd:{t}; wdt:P407 wd:Q13955 . ?n {prop} ?l FILTER(lang(?l)="en") }}'))
            arabic |= names(query(
                f'SELECT ?l WHERE {{ ?n wdt:P31 wd:{t}; wdt:P1705 ?nl FILTER(lang(?nl)="ar" || lang(?nl)="arz") . '
                f'?n {prop} ?l FILTER(lang(?l)="en") }}'))
        for lang in WESTERN_LANGS:
            western |= names(query(f'SELECT ?l WHERE {{ ?n wdt:P31 wd:{t}; wdt:P407 wd:{lang} . ?n rdfs:label ?l FILTER(lang(?l)="en") }}'))
    common = {k.lower() for k in json.loads((ROOT / "data" / "sources" / "misaki" / "us_gold.json").read_text(encoding="utf-8"))}
    keep = sorted(n for n in arabic - western - NOT_ARABIC if n not in common and len(n) >= 3)
    OUT.write_text("\n".join(keep) + "\n", encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(keep)} names")
    cmu = {line.split()[0] for line in (ROOT / "data" / "sources" / "cmudict" / "cmudict.dict").read_text(encoding="utf-8").splitlines() if line.strip()}
    words: set[str] = set()
    for country in ("Q986", "Q115"):
        for label in query(f'SELECT ?l WHERE {{ ?p wdt:P27 wd:{country}; wdt:P31 wd:Q5 . ?p rdfs:label ?l FILTER(lang(?l)="en") }}'):
            words |= {t.lower() for t in re.findall(r"\b[A-Z][a-z]+\b", label)}
    horn = sorted(w for w in words if w.isascii() and len(w) >= 3 and w not in common and w not in western and w not in set(keep) and w not in cmu)
    HORN.write_text("\n".join(horn) + "\n", encoding="utf-8")
    print(f"{HORN.relative_to(ROOT)}: {len(horn)} words")


if __name__ == "__main__":
    main()
