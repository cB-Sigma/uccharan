#!/usr/bin/env python3
"""Expand the hand-written African names list (gold/african_names.src) into
the gold set gold/african_names.tsv used by names_eval.py --gold.

Each source line is ``Name origin kind pattern[|pattern…]``. A pattern is the
English-context pronunciation written by hand from the language's documented
phonology and spelling (sources in the .tsv header): ``.`` separates
syllables, ``*`` marks every syllable whose vowel may carry the English
primary stress (names from tone languages have no fixed English stress), and
the plain letters a e i o u stand for the language's vowels, which an English
speaker renders with either of two English vowels (a: ɑ/æ, e: eɪ/ɛ, i: i/ɪ,
o: oʊ/ɔ, u: u/ʊ). Every combination is an accepted pronunciation. The split
is fixed by a hash of the id: about half dev, half held-out.
"""

from __future__ import annotations

import hashlib
import itertools
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
SRC = HERE / "gold" / "african_names.src"
OUT = HERE / "gold" / "african_names.tsv"

SINGLE_GEMINATES = False
STRESSED = {"a": ("ɑ", "æ"), "e": ("eɪ", "ɛ"), "i": ("i", "ɪ"), "o": ("oʊ", "ɔ"), "u": ("u", "ʊ")}
PLAIN = {"a": "ɑ", "e": "ɛ", "i": "i", "o": "oʊ", "u": "u"}
GLIDES = "ɪʊə"

HEADER = """# African names gold set (built before any African-names work). Columns: id, text, expected, origin, source, split.
# expected: hand-written English-context pronunciations, alternatives separated by |; generated from gold/african_names.src
# by tools/eval/build_african_names.py. source: phonology = written by hand from the descriptions below; never from a phonemizer.
# Check: skeleton (consonants + primary-stressed vowel), as names.tsv. split: dev (may be used to adjust rules) or heldout (never).
# Phonology and spelling sources by language:
#  Yoruba: Bamgbose 1966, A Grammar of Yoruba; Akinlabi & Liberman 2001 (tone, orthography: s/ṣ, gb, p=kp).
#  Igbo: Emenanjo 2015, A Grammar of Contemporary Igbo; Ikekeonwu 1991 JIPA "Igbo" (ch, gb, kp, nw, ny, syllabic n/m).
#  Hausa: Newman 2000, The Hausa Language (c=ch, ts, j, sh; no lexical stress, tone).
#  Akan/Twi: Dolphyne 1988, The Akan (Twi-Fante) Language; dw/gy/ky/hy/tw/nw as affricates/glides.
#  Ewe: Ameka 1991; Westermann 1930 (gb, kp, dz, ts, ny, ŋ).  Swahili: Mohammed 2001, Modern Swahili Grammar (penultimate stress).
#  Kikuyu: Mugane 1997 (ng'/nj/mb prenasalised).  Luo: Tucker 1994, A Grammatical Sketch of Dholuo (dh = ð, ch, ny, ng').
#  Luganda: Ashton et al. 1954, A Luganda Grammar (doubled consonants, ky/gy).
#  Zulu/Xhosa/Ndebele: Doke 1926/1954; Poulos & Msimang 1998 (hl = ɬ, dl = ɮ, c/q/x clicks, said k/kl by most English speakers).
#  Sotho/Tswana: Cole 1955, An Introduction to Tswana Grammar; Doke & Mofokeng 1957 (Tswana g = x/h, th = tʰ, tsh, o/u).
#  Shona: Fortune 1955; Mkanganwi 1972 (sv/zv whistled sibilants, said s/z or sw/zw; dz, tsv, ny).
#  Amharic/Tigrinya: Leslau 1995, Reference Grammar of Amharic; Ethiopian romanisation conventions (ts = ejective tsʼ).
#  Somali: Saeed 1999, Somali (x = ħ, c = ʕ, dh = ɖ, doubled vowels long; official Latin orthography 1972).
#  Oromo: Owens 1985; Qubee orthography (c = tʃʼ, ch = tʃ, dh = ɗ, x = tʼ, doubled letters long).
#  Wolof: Ka 1994, Wolof Phonology and Morphology; French-based spellings (ou = u, Ndiaye, Mb-).
#  Fula: Arnott 1970 (French spellings: Diallo = Jallo, ou = u).  Lingala: Meeuwis 1998; French spellings (tsh, ou).
#  Malagasy: Keenan & Polinsky 1998 (o = u, final vowels weak, penultimate stress; tr/dr affricates, j = dz).
#  North African names: Arabic and Berber (Tamazight) names in their French-based Latin spellings (ou = u/w, ch = ʃ, kh = x).
#  Other names (Kalenjin, Kinyarwanda, Kirundi, Chichewa, Bemba, Oshiwambo, Kimbundu, Tsonga, Swati, Mende, Temne, Bambara,
#  Moore, Zarma, Sara, Ewondo, Duala): the same Bantu / West African spelling conventions; place names as usually said in English.
"""


def expand(pattern: str) -> list[str]:
    out: list[str] = []
    stars = [i for i, c in enumerate(pattern) if c == "*"]
    for s in stars:
        before = pattern[:s].replace("*", "").replace(".", "")
        after = pattern[s + 1 :].replace("*", "").replace(".", "")
        v, rest = after[0], after[1:]
        if v in STRESSED and not (rest[:1] and rest[0] in GLIDES):
            choices = STRESSED[v]
        else:
            choices = (v,)

        def plain(t: str) -> str:
            return "".join(PLAIN.get(c, c) if c in PLAIN and not (k + 1 < len(t) and t[k + 1] in GLIDES) else c for k, c in enumerate(t))

        for ch in choices:
            out.append(plain(before) + "ˈ" + ch + plain(rest))
    if SINGLE_GEMINATES:
        # English speakers say a written double consonant once.
        out = [re.sub(r"([^aeiouɑæɛɪʊəɔˈ])(ˈ?)\1", r"\2\1", x) for x in out]
    return out


NA_HEADER = """# North African names gold set (written before any work on these names; the 20 North African names of
# african_names.tsv stay there as a regression check). Columns: id, text, expected, origin, source, split.
# expected: hand-written English-context pronunciations generated from gold/north_african_names.src by
# tools/eval/build_african_names.py --north-african; never from a phonemizer. Check: skeleton. split: by hash.
# Sources: Ryding 2005, A Reference Grammar of Modern Standard Arabic (weight-based stress: a final superheavy
#  syllable, else a heavy penult, else the antepenult); Watson 2002, The Phonology and Morphology of Arabic;
#  Harrell 1962, A Short Reference Grammar of Moroccan Arabic; Woidich 2006, Das Kairenisch-Arabische (Egyptian
#  g for jim, stress); Kossmann 2013, The Arabic Influence on Northern Berber, and Chaker's Tamazight
#  descriptions (Berber names and places). Spelling: French-based Maghrebi transliteration (ou = u/w, ch = sh,
#  dj = j, kh = x, gh = gh/ɣ, e = schwa or silent) and English-based Egyptian spellings; English speakers' x and ɣ
#  are accepted as k/h and ɡ.
"""


EG_HEADER = """# Egyptian names gold set, second set (written 2026-11-14 before any work on these names, because the 38 Egyptian
# names of north_african_names.tsv were too few and the improved reader still trailed the reference on them).
# Columns: id, text, expected, origin, source, split. expected: hand-written English-context pronunciations generated
# from gold/egyptian_names.src by tools/eval/build_african_names.py --egyptian; never from a phonemizer. Check: skeleton.
# split: by hash, fixed before tuning (dev may be used to adjust rules; heldout is scored once).
# Sources: Woidich 2006, Das Kairenisch-Arabische (stress: a final superheavy syllable, else a heavy penult, else the
#  penult or antepenult by syllable count; jim = g; qaf = glottal stop, said k by English speakers from the spelling);
#  Watson 2002, The Phonology and Morphology of Arabic (Cairene stress); Badawi & Hinds 1986, A Dictionary of Egyptian
#  Arabic (vowel length of names); Gamal-Eldin 1967. Spelling: the English-based transliteration Egyptians use in
#  passports and the press (final -y = i, kh = x said k/h, gh = ɣ said g, doubled vowels mark ʕ or length,
#  e/o for short i/u; a written double consonant is said once). Coptic Christian names (Bishoy, Kirollos, Tawadros, Shenouda) as said in Egyptian Arabic.
"""

SE_HEADER = """# Names from four groups that were too small in african_names.tsv (Chichewa, Shona, Tsonga, Tigrinya), written
# 2026-11-14 before any work on them. Columns: id, text, expected, origin, source, split. expected: hand-written
# English-context pronunciations generated from gold/se_african_names.src by build_african_names.py --se-african;
# never from a phonemizer. Check: skeleton. split: by hash, fixed before tuning.
# Sources: Chichewa: Mchombo 2004, The Syntax of Chichewa (phonology chapter: penultimate lengthening and stress);
#  Kanerva 1990, Focus and Phrasing in Chichewa Phonology; Chichewa orthography (ch = tʃ, ph/th/kh aspirated stops,
#  said p/t/k; ng' = ŋ; r/l one phoneme; syllabic m/n before a consonant, often dropped by English speakers).
#  Shona: Fortune 1955, An Analytical Grammar of Shona; Mkanganwi 1972; 1967 standard orthography (sv/zv whistled
#  sibilants said s/z or sw/zw, pf, dz, ny, breathy mh/bh/vh, final -ai said a.i or aɪ; English penultimate stress).
#  Tsonga: Baumbach 1987, Analytical Tsonga Grammar (x = ʃ, hl = ɬ said ʃl/l, rh = breathy r, ts/tsw/dz, penultimate
#  stress). Tigrinya: Leslau 1941, Documents tigrigna; Kogan 1997, Tigrinya, in Hetzron (ed.), The Semitic Languages
#  (weak stress, usually on a closed or the penultimate syllable; final -ay = aɪ; ts = ejective tsʼ said ts; gh = ɣ
#  said g; place names as usually said in English).
"""


def main() -> None:
    import sys

    global SRC, OUT, HEADER, SINGLE_GEMINATES
    SINGLE_GEMINATES = "--egyptian" in sys.argv or "--se-african" in sys.argv
    for flag, stem, header in (("--north-african", "north_african_names", NA_HEADER), ("--egyptian", "egyptian_names", EG_HEADER), ("--se-african", "se_african_names", SE_HEADER)):
        if flag in sys.argv:
            SRC = HERE / "gold" / f"{stem}.src"
            OUT = HERE / "gold" / f"{stem}.tsv"
            HEADER = header
    rows = []
    for line in SRC.read_text(encoding="utf-8").splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        name, origin, kind, patterns = line.split(None, 3)
        alts = list(dict.fromkeys(itertools.chain.from_iterable(expand(p) for p in patterns.split("|"))))
        rid = f"{origin}-{name.lower()}"
        split = "dev" if int(hashlib.sha1(rid.encode()).hexdigest(), 16) % 2 == 0 else "heldout"
        if kind.endswith("!"):  # added later to grow a development half: always dev
            kind, split = kind[:-1], "dev"
        text = f"We drove to {name}." if kind == "place" else f"Have you met {name}?"
        rows.append((rid, text, "|".join(alts), origin, "phonology", split))
    OUT.write_text(HEADER + "".join("\t".join(r) + "\n" for r in rows), encoding="utf-8")
    print(f"{OUT.name}: {len(rows)} names, dev {sum(r[5] == 'dev' for r in rows)}, heldout {sum(r[5] == 'heldout' for r in rows)}")


if __name__ == "__main__":
    main()
