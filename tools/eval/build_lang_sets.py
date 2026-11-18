#!/usr/bin/env python3
"""Build development and held-out sentence sets for one language.

    python tools/eval/build_lang_sets.py bn mr ta

Sentences come from the Tatoeba per-language export (CC BY 2.0 FR),
downloaded into ``--work``. When Tatoeba has fewer than ``--min`` usable
sentences, the FLORES-200 benchmark (CC BY-SA 4.0; ``--flores DIR`` pointing
at an unpacked ``flores200_dataset``) is used instead: its ``dev`` split
becomes our dev set and its ``devtest`` split our held-out set. Languages
in neither (Dogri, Bodo, Santali) take their sentences from AI4Bharat's
IndicCorp v2 monolingual text (CC0, downloaded into ``--work``; ids
``indiccorp-``), with lines in Hindi (Santali: lines with wiki markup) left
out. A language in Google's IndicGenBench
(Flores-IN, CC BY-SA 4.0: the FLORES-200 sentences in languages FLORES-200
itself lacks) also gets a second held-out set ``<lang>_flores.tsv`` from
its ``devtest`` split (ids ``flores-devtest-``), never used for tuning; a
language in ``FLORES_DEV`` (Manipuri, whose IndicGenBench text is in Bengali
script while its IndicCorp text is in Meetei Mayek) also gets
``<lang>_flores_dev.tsv`` from the ``dev`` split, which rules may be tuned on.
A language in ``WIKISET`` (Santali, Manipuri, Kashmiri) takes its dev and held-out sets from
IndicCorp v2 even where Tatoeba has enough sentences, and gets two more
held-out sets, never used for tuning: ``<lang>_wiki.tsv``, sentences from
random articles of its Wikipedia (CC BY-SA 4.0, ids ``wp-``) that are not
in the IndicCorp text, and ``<lang>_tatoeba.tsv``, its Tatoeba sentences
(CC BY 2.0 FR) whose id hashes into the held-out half (not written when
Tatoeba has fewer than 50 sentences of the language: Manipuri has 7).
Without FLORES, Wikipedia article extracts
(CC BY-SA 4.0, ids ``wp-``) top the set up. Share-alike sentences are
evaluation input only (``tools/eval`` is not part of any package).

The split is fixed by a hash of the sentence id, so it never depends on
which sentences were looked at: ids hashing below 500/1000 are ``dev`` (rules
may be tuned on them), the rest are ``heldout`` (never used for tuning).
Outputs ``tools/eval/data/<lang>_dev.tsv`` and ``<lang>_heldout.tsv``
(``id<TAB>text``). Nothing here comes from a phonemizer.
"""

from __future__ import annotations

import argparse
import bz2
import hashlib
import json
import random
import time
import re
import urllib.parse
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
TATOEBA = "https://downloads.tatoeba.org/exports/per_language/{0}/{0}_sentences.tsv.bz2"
WIKI = "https://{0}.wikipedia.org/w/api.php?action=query&format=json&generator=random&grnnamespace=0&grnlimit=1&prop=extracts&explaintext=1&exlimit=1"

FLORES = {"bn": "ben_Beng", "mr": "mar_Deva", "ne": "npi_Deva", "gu": "guj_Gujr", "pa": "pan_Guru", "or": "ory_Orya",
          "as": "asm_Beng", "ta": "tam_Taml", "te": "tel_Telu", "kn": "kan_Knda", "ml": "mal_Mlym", "ur": "urd_Arab",
          "sd": "snd_Arab", "si": "sin_Sinh", "es": "spa_Latn", "fr": "fra_Latn", "de": "deu_Latn", "it": "ita_Latn",
          "pt": "por_Latn", "nl": "nld_Latn", "pl": "pol_Latn", "ru": "rus_Cyrl", "tr": "tur_Latn", "sa": "san_Deva",
          "mai": "mai_Deva", "bho": "bho_Deva", "mag": "mag_Deva"}

# IndicCorp v2 (AI4Bharat, CC0): uccharan tag -> file name, and a pattern
# of Hindi function words that marks a line as Hindi rather than the language.
INDICCORP_URL = "https://huggingface.co/datasets/ai4bharat/IndicCorpV2/resolve/main/data/{}.txt"
INDICCORP = {"doi": ("dg", r"(?:^|\s)(?:है|हैं|में|का|को|से|था|थी|थे|नहीं|और|लिए|भी|यह|वह)(?=[\s।,?!]|$)"),
             # Bodo: से is also the Bodo word for one, so it does not mark Hindi.
             "brx": ("bd", r"(?:^|\s)(?:है|हैं|में|का|को|की|के|था|थी|थे|नहीं|और|लिए|भी|यह|वह|ने)(?=[\s।,?!]|$)"),
             # Santali (Ol Chiki): the text is Ol Chiki throughout, but some
             # lines keep Wikipedia markup.
             "sat": ("sat", r"\[\[|\]\]|\{\{|\}\}|[<>|=]"),
             # Manipuri: the IndicCorp text is Meetei Mayek (Wikipedia-like,
             # some markup kept).
             "mni": ("mni", r"\[\[|\]\]|\{\{|\}\}|[<>|=]"),
             # Kashmiri: the IndicCorp text is Perso-Arabic (Wikipedia-like);
             # lines with Urdu function words are left out.
             "ks": ("ks", r"\[\[|\]\]|\{\{|\}\}|[<>|=]|(?:^|\s)(?:ہے|ہیں|میں|تھا|تھی|تھے|اور)(?=[\s۔،؟!.]|$)")}

# IndicGenBench Flores-IN (Google, CC BY-SA 4.0): uccharan tag -> language code
# of flores_in/flores_<code>_en_test.json (the FLORES-200 devtest sentences).
INDICGENBENCH_URL = "https://raw.githubusercontent.com/google-research-datasets/indic-gen-bench/main/flores_in/flores_{}_en_test.json"
INDICGENBENCH = {"brx": "brx", "sat": "sat", "mni": "mni"}
INDICGENBENCH_DEV_URL = INDICGENBENCH_URL.replace("_test.json", "_dev.json")
# Languages whose IndicGenBench dev split is written as a tuning set too.
FLORES_DEV = {"mni"}

# Languages with a third held-out set from their Wikipedia (uccharan tag ->
# Wikipedia code, number of sentences).
WIKISET = {"sat": ("sat", 500), "mni": ("mni", 500), "ks": ("ks", 500)}

# uccharan tag -> (Tatoeba code, Wikipedia code, letter regex of the language's script)
LANGS: dict[str, tuple[str, str, str]] = {
    "bn": ("ben", "bn", r"[ঀ-৿]"),
    "mr": ("mar", "mr", r"[ऀ-ॿ]"),
    "ne": ("nep", "ne", r"[ऀ-ॿ]"),
    "gu": ("guj", "gu", r"[઀-૿]"),
    "pa": ("pan", "pa", r"[਀-੿]"),
    "or": ("ori", "or", r"[଀-୿]"),
    "as": ("asm", "as", r"[ঀ-৿]"),
    "ta": ("tam", "ta", r"[஀-௿]"),
    "te": ("tel", "te", r"[ఀ-౿]"),
    "kn": ("kan", "kn", r"[ಀ-೿]"),
    "ml": ("mal", "ml", r"[ഀ-ൿ]"),
    "kok": ("kok", "gom", r"[ऀ-ॿ]"),
    "bho": ("bho", "bh", r"[ऀ-ॿ]"),
    "mai": ("mai", "mai", r"[ऀ-ॿ]"),
    "ur": ("urd", "ur", r"[؀-ۿ]"),
    "sd": ("snd", "sd", r"[؀-ۿ]"),
    "si": ("sin", "si", r"[඀-෿]"),
    "es": ("spa", "es", r"[a-záéíóúñü]"),
    "fr": ("fra", "fr", r"[a-zàâçéèêëîïôûùüÿœ]"),
    "de": ("deu", "de", r"[a-zäöüß]"),
    "it": ("ita", "it", r"[a-zàèéìòù]"),
    "pt": ("por", "pt", r"[a-záâãàçéêíóôõú]"),
    "nl": ("nld", "nl", r"[a-z]"),
    "pl": ("pol", "pl", r"[a-ząćęłńóśźż]"),
    "ru": ("rus", "ru", r"[а-яё]"),
    "tr": ("tur", "tr", r"[a-zçğıöşü]"),
    "sa": ("san", "sa", r"[ऀ-ॿ]"),
    "mag": ("mag", "mag", r"[ऀ-ॿ]"),
    "doi": ("doi", "dgo", r"[ऀ-ॿ]"),
    "brx": ("brx", "brx", r"[ऀ-ॿ]"),
    "sat": ("sat", "sat", r"[ᱚ-ᱽ]"),
    # Manipuri (Meitei): Meetei Mayek and its extensions.
    "mni": ("mni", "mni", r"[ꯀ-꯭ꫠ-꫶]"),
    # Kashmiri in Perso-Arabic script (with the Kashmiri vowel signs).
    "ks": ("kas", "ks", r"[؀-ۿ]"),
}


def fetch(url: str, dest: Path) -> Path:
    if not dest.exists():
        req = urllib.request.Request(url, headers={"User-Agent": "uccharan-eval/0.2"})
        with urllib.request.urlopen(req, timeout=300) as r:
            dest.write_bytes(r.read())
    return dest


def bucket(key: str) -> int:
    return int(hashlib.sha1(key.encode()).hexdigest(), 16) % 1000


def tatoeba(work: Path, code: str) -> list[tuple[str, str]]:
    try:
        path = fetch(TATOEBA.format(code), work / f"{code}_sentences.tsv.bz2")
    except OSError as e:
        print(f"tatoeba {code}: {e}")
        return []
    rows = []
    with bz2.open(path, "rt", encoding="utf-8") as fh:
        for line in fh:
            f = line.rstrip("\n").split("\t")
            if len(f) >= 3:
                rows.append((f[0], f[2]))
    return rows


def wikipedia(work: Path, code: str, want: int, letters: str) -> list[tuple[str, str]]:
    out: dict[str, str] = {}
    for k in range(300):
        if len(out) >= want:
            break
        dest = work / f"wiki_{code}_{k}.json"
        if not dest.exists():
            time.sleep(1.5)  # be polite to the API
        data = None
        for attempt in range(4):
            try:
                data = json.loads(fetch(WIKI.format(code), dest).read_text(encoding="utf-8"))
                break
            except OSError as e:
                print(f"wikipedia {code}: {e}")
                if "429" not in str(e) or attempt == 3:
                    break
                time.sleep(60)  # rate limited: wait and try again
        if data is None:
            break
        for page in data.get("query", {}).get("pages", {}).values():
            for s in re.split(r"(?<=[.!?।॥᱾᱿꯫۔؟])\s+", page.get("extract", "")):
                s = s.strip()
                alpha = [c for c in s if c.isalpha()]
                own = sum(1 for c in alpha if re.match(letters, c.lower()))
                # Prose of the language only: no other alphabets, few digits.
                clean = alpha and own / len(alpha) >= 0.97 and sum(c.isdigit() for c in s) <= 4
                if 15 <= len(s) <= 160 and clean and "\n" not in s and "==" not in s:
                    out.setdefault(f"wp-{code}-{hashlib.sha1(s.encode()).hexdigest()[:12]}", s)
    return list(out.items())


def indiccorp(work: Path, lang: str, letters: str) -> list[tuple[str, str]]:
    """Sentences of the language from its IndicCorp v2 file, Hindi lines left out."""
    name, hindi = INDICCORP[lang]
    try:
        path = fetch(INDICCORP_URL.format(name), work / f"indiccorp_{name}.txt")
    except OSError as e:
        print(f"indiccorp {name}: {e}")
        return []
    out: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        for s in re.split(r"(?<=[।?!|᱾᱿꯫۔؟])\s+", line.strip()):
            s = " ".join(s.split())
            alpha = [c for c in s if c.isalpha()]
            own = sum(1 for c in alpha if re.match(letters, c))
            if 15 <= len(s) <= 160 and alpha and own / len(alpha) >= 0.97 and not re.search(hindi, s):
                out.setdefault(f"indiccorp-{hashlib.sha1(s.encode()).hexdigest()[:12]}", s)
    return list(out.items())


def indicgenbench(work: Path, lang: str) -> None:
    """Write the second held-out set from IndicGenBench's Flores-IN devtest split."""
    code = INDICGENBENCH[lang]
    path = fetch(INDICGENBENCH_URL.format(code), work / f"indicgenbench_{code}_test.json")
    rows = [e["source"] for e in json.loads(path.read_text(encoding="utf-8"))["examples"]]
    out = HERE / "data" / f"{lang}_flores.tsv"
    out.write_text("".join(f"flores-devtest-{k}\t{' '.join(t.split())}\n" for k, t in enumerate(rows)), encoding="utf-8")
    print(f"{out.name}: {len(rows)} sentences (IndicGenBench Flores-IN devtest, CC BY-SA 4.0; evaluation only)")
    if lang in FLORES_DEV:
        path = fetch(INDICGENBENCH_DEV_URL.format(code), work / f"indicgenbench_{code}_dev.json")
        rows = [e["source"] for e in json.loads(path.read_text(encoding="utf-8"))["examples"]]
        out = HERE / "data" / f"{lang}_flores_dev.tsv"
        out.write_text("".join(f"flores-dev-{k}\t{' '.join(t.split())}\n" for k, t in enumerate(rows)), encoding="utf-8")
        print(f"{out.name}: {len(rows)} sentences (IndicGenBench Flores-IN dev, CC BY-SA 4.0; evaluation only)")


def tatoeba_set(lang: str, rows: list[tuple[str, str]], want: int = 1000) -> None:
    """Write a held-out set of the language's Tatoeba sentences (held-out half of the hash only)."""
    held = sorted((r for r in rows if bucket(r[0]) >= 500), key=lambda r: r[0])
    rng = random.Random(20261118)
    chosen = sorted(rng.sample(held, min(want, len(held))), key=lambda r: r[0])
    out = HERE / "data" / f"{lang}_tatoeba.tsv"
    out.write_text("".join(f"{i}\t{' '.join(t.split())}\n" for i, t in chosen), encoding="utf-8")
    print(f"{out.name}: {len(chosen)} sentences (Tatoeba, CC BY 2.0 FR; evaluation only)")


def wiki_set(work: Path, lang: str, avoid: set[str]) -> None:
    """Write the third held-out set from random articles of the language's Wikipedia."""
    code, want = WIKISET[lang]
    letters = LANGS[lang][2]
    rows = [(i, t) for i, t in wikipedia(work, code, want * 3, letters) if " ".join(t.split()) not in avoid]
    rng = random.Random(20261118)
    rng.shuffle(rows)
    chosen = sorted(rows[:want], key=lambda r: r[0])
    out = HERE / "data" / f"{lang}_wiki.tsv"
    out.write_text("".join(f"{i}\t{' '.join(t.split())}\n" for i, t in chosen), encoding="utf-8")
    print(f"{out.name}: {len(chosen)} sentences ({code}.wikipedia.org, CC BY-SA 4.0; evaluation only)")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("langs", nargs="+")
    ap.add_argument("--work", type=Path, default=HERE / ".work")
    ap.add_argument("--min", type=int, default=1500, help="top up from Wikipedia below this many sentences")
    ap.add_argument("--dev", type=int, default=1000)
    ap.add_argument("--heldout", type=int, default=2000)
    ap.add_argument("--flores", type=Path, help="unpacked flores200_dataset directory")
    args = ap.parse_args()
    args.work.mkdir(parents=True, exist_ok=True)
    for lang in args.langs:
        if lang in INDICGENBENCH:
            indicgenbench(args.work, lang)
        tcode, wcode, letters = LANGS[lang]
        rows = [(i, t) for i, t in tatoeba(args.work, tcode) if 8 <= len(t) <= 160 and re.search(letters, t.lower())]
        source = "Tatoeba (CC BY 2.0 FR)"
        if lang in WIKISET:
            # Santali: the IndicCorp text is the main set; the Tatoeba
            # sentences of the held-out half become a held-out set of their own.
            if len(rows) >= 50:
                tatoeba_set(lang, rows)
            rows = indiccorp(args.work, lang, letters)
            source = "IndicCorp v2 (CC0)"
            wiki_set(args.work, lang, {t for _, t in rows})
        if len(rows) < args.min and lang in INDICCORP:
            rows += indiccorp(args.work, lang, letters)
            source = "IndicCorp v2 (CC0)"
        if len(rows) < args.min and args.flores and lang in FLORES:
            code = FLORES[lang]
            for name, split in (("dev", "dev"), ("heldout", "devtest")):
                lines = (args.flores / split / f"{code}.{split}").read_text(encoding="utf-8").splitlines()
                path = HERE / "data" / f"{lang}_{name}.tsv"
                path.write_text("".join(f"flores-{split}-{k}\t{t}\n" for k, t in enumerate(lines)), encoding="utf-8")
                print(f"{path.name}: {len(lines)} sentences (FLORES-200 {split}, CC BY-SA 4.0; evaluation only)")
            continue
        if len(rows) < args.min:
            rows += wikipedia(args.work, wcode, args.min - len(rows), letters)
            source += " + Wikipedia extracts (CC BY-SA 4.0; evaluation only)"
        rng = random.Random(20261114)
        dev = [r for r in rows if bucket(r[0]) < 500]
        held = [r for r in rows if bucket(r[0]) >= 500]
        rng.shuffle(dev)
        rng.shuffle(held)
        for name, part, n in (("dev", dev, args.dev), ("heldout", held, args.heldout)):
            chosen = sorted(part[:n], key=lambda r: r[0])
            path = HERE / "data" / f"{lang}_{name}.tsv"
            path.write_text("".join(f"{i}\t{t}\n" for i, t in chosen), encoding="utf-8")
            print(f"{path.name}: {len(chosen)} sentences ({source})")


if __name__ == "__main__":
    main()
