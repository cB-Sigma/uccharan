#!/usr/bin/env python3
"""Build the evaluation sets in tools/eval/data from public sources.

Sources (downloaded into ``--work``; nothing here comes from a phonemizer):

* Tatoeba per-language sentence exports (CC BY 2.0 FR): English and Hindi.
* Project Gutenberg #1661, "The Adventures of Sherlock Holmes" (public domain
  in the USA): running English prose.
* Hindi Wikipedia article extracts (CC BY-SA 4.0), fetched at build time into
  the work directory only: an independent Hindi word list. These are **not**
  written to the repository.

Outputs (tab-separated ``id<TAB>text``):

* ``en_dev.tsv`` / ``hi_dev.tsv``      — sentences the rules were tuned on (a sample);
* ``en_heldout.tsv`` / ``hi_heldout.tsv`` — sentences never looked at during development;
* ``en_prose.tsv``                     — Sherlock Holmes sentences (never looked at);
* ``en_words_*.txt`` / ``hi_words_*.txt`` — word lists derived from the above.

The ids of every sentence used during development are read from
``--dev-ids`` files so that the held-out sets stay disjoint from them.
"""

from __future__ import annotations

import argparse
import bz2
import hashlib
import json
import random
import re
import urllib.request
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
TATOEBA = "https://downloads.tatoeba.org/exports/per_language/{0}/{0}_sentences.tsv.bz2"
GUTENBERG = "https://www.gutenberg.org/cache/epub/1661/pg1661.txt"
WIKI_API = "https://hi.wikipedia.org/w/api.php?action=query&format=json&prop=extracts&explaintext=1&exlimit=1&titles={}"
WIKI_TITLES = ["भारत", "हिन्दी", "दिल्ली", "गंगा नदी", "महात्मा गांधी", "क्रिकेट", "सूर्य", "पृथ्वी", "विज्ञान", "संगीत", "रेल परिवहन", "कृषि"]


def fetch(url: str, dest: Path) -> Path:
    if not dest.exists():
        req = urllib.request.Request(url, headers={"User-Agent": "uccharan-eval/0.1"})
        with urllib.request.urlopen(req, timeout=300) as r:
            dest.write_bytes(r.read())
    return dest


def bucket(key: str) -> int:
    return int(hashlib.sha1(key.encode()).hexdigest(), 16) % 1000


def tatoeba(work: Path, code: str) -> list[tuple[str, str]]:
    path = fetch(TATOEBA.format(code), work / f"{code}_sentences.tsv.bz2")
    rows = []
    with bz2.open(path, "rt", encoding="utf-8") as fh:
        for line in fh:
            f = line.rstrip("\n").split("\t")
            if len(f) >= 3:
                rows.append((f[0], f[2]))
    return rows


def write(name: str, rows: list[tuple[str, str]]) -> None:
    (HERE / "data" / name).write_text("".join(f"{i}\t{t}\n" for i, t in rows), encoding="utf-8")
    print(f"{name}: {len(rows)}")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--work", type=Path, default=HERE / ".work")
    ap.add_argument("--dev-ids-en", type=Path, help="ids of English sentences seen in development")
    ap.add_argument("--dev-ids-hi", type=Path, help="ids of Hindi sentences seen in development")
    ap.add_argument("--cmudict", type=Path, default=HERE.parents[1] / "data/sources/cmudict/cmudict.dict")
    args = ap.parse_args()
    args.work.mkdir(parents=True, exist_ok=True)
    rng = random.Random(20261113)

    # ---- English sentences -------------------------------------------------
    en = [(i, t) for i, t in tatoeba(args.work, "eng") if 25 <= len(t) <= 160]
    dev_en = set(args.dev_ids_en.read_text().split()) if args.dev_ids_en else set()
    dev_rows = [r for r in en if r[0] in dev_en]
    held_rows = [r for r in en if r[0] not in dev_en and bucket(r[0]) < 40]
    rng.shuffle(dev_rows)
    rng.shuffle(held_rows)
    write("en_dev.tsv", sorted(dev_rows[:2000], key=lambda r: int(r[0])))
    held_en = sorted(held_rows[:3000], key=lambda r: int(r[0]))
    write("en_heldout.tsv", held_en)

    # ---- English prose -----------------------------------------------------
    text = fetch(GUTENBERG, args.work / "pg1661.txt").read_text(encoding="utf-8-sig")
    body = text.split("*** START OF", 1)[-1].split("*** END OF", 1)[0]
    body = body.split("\n", 1)[-1].replace("\r", "")
    body = re.sub(r"_", "", body)
    paras = [re.sub(r"\s+", " ", p).strip() for p in body.split("\n\n")]
    sents: list[str] = []
    for p in paras:
        for s in re.split(r"(?<=[.!?])[\"”’]?\s+(?=[\"“‘A-Z])", p):
            s = s.strip().strip("\"“”")
            if 40 <= len(s) <= 200 and s[0].isupper() and s[-1] in ".!?" and not re.search(r"[IVXL]{2,}\.", s):
                sents.append(s)
    rng.shuffle(sents)
    write("en_prose.tsv", [(f"pg1661-{k}", s) for k, s in enumerate(sents[:1000])])

    # ---- Hindi sentences ---------------------------------------------------
    hi = [(i, t) for i, t in tatoeba(args.work, "hin") if 10 <= len(t) <= 160 and re.search(r"[ऀ-ॿ]", t)]
    dev_hi = set(args.dev_ids_hi.read_text().split()) if args.dev_ids_hi else set()
    dev_rows = [r for r in hi if r[0] in dev_hi]
    held_rows = [r for r in hi if r[0] not in dev_hi]
    rng.shuffle(dev_rows)
    rng.shuffle(held_rows)
    write("hi_dev.tsv", sorted(dev_rows[:2000], key=lambda r: int(r[0])))
    held_hi = sorted(held_rows[:3000], key=lambda r: int(r[0]))
    write("hi_heldout.tsv", held_hi)

    # ---- English word lists --------------------------------------------------
    lexicon = set()
    for line in args.cmudict.read_text(encoding="utf-8").splitlines():
        w = line.split(" ", 1)[0]
        if "(" not in w:
            lexicon.add(w)
    counts: Counter[str] = Counter()
    for _, t in en:
        for w in re.findall(r"[A-Za-z]+(?:'[a-z]+)?", t):
            counts[w.lower()] += 1
    frequent = [w for w, _ in counts.most_common() if w in lexicon and len(w) > 1][:5000]
    (HERE / "data" / "en_words_frequent.txt").write_text("\n".join(frequent) + "\n")
    seen = set(counts)
    rare = sorted(w for w in lexicon if w.isalpha() and w not in seen and len(w) >= 4)
    rng.shuffle(rare)
    (HERE / "data" / "en_words_rare.txt").write_text("\n".join(sorted(rare[:3000])) + "\n")
    oov = [w for w, c in counts.most_common() if w not in lexicon and w.isalpha() and len(w) >= 4 and c >= 2]
    # Three product names were removed from the published list by hand afterwards.
    (HERE / "data" / "en_words_oov.txt").write_text("\n".join(oov[:2000]) + "\n")
    print(f"en words: frequent {len(frequent)}, rare {min(len(rare), 3000)}, oov {min(len(oov), 2000)}")

    # ---- Hindi word lists ----------------------------------------------------
    hcounts: Counter[str] = Counter()
    for _, t in held_hi:
        for w in re.findall(r"[ऀ-ॣ॰-ॿ]+", t):
            hcounts[w] += 1
    words = [w for w, _ in hcounts.most_common() if len(w) > 1]
    (HERE / "data" / "hi_words_heldout.txt").write_text("\n".join(words) + "\n", encoding="utf-8")
    print(f"hi words (held-out sentences): {len(words)}")

    # Independent word list from Hindi Wikipedia: written to the work directory only.
    wiki: Counter[str] = Counter()
    for title in WIKI_TITLES:
        try:
            dest = args.work / f"hiwiki_{hashlib.sha1(title.encode()).hexdigest()[:10]}.json"
            data = json.loads(fetch(WIKI_API.format(urllib.request.quote(title)), dest).read_text(encoding="utf-8"))
            for page in data.get("query", {}).get("pages", {}).values():
                for w in re.findall(r"[ऀ-ॣ॰-ॿ]+", page.get("extract", "")):
                    wiki[w] += 1
        except Exception as exc:  # network trouble: the set is optional
            print(f"wikipedia {title}: {exc}")
    wwords = [w for w, c in wiki.most_common() if len(w) > 1 and "़़" not in w][:5000]
    (args.work / "hi_words_wikipedia.txt").write_text("\n".join(wwords) + "\n", encoding="utf-8")
    print(f"hi words (wikipedia, work dir only): {len(wwords)}")


if __name__ == "__main__":
    main()
