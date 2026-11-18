#!/usr/bin/env python3
"""Bhojpuri / Maithili gold-set evaluation: uccharan improved mode against the
Hindi-voice fallback (espeak-ng 1.52 `-v hi`, which uccharan's compatible mode
reproduces) on the hand-written gold set gold/<lang>/gold.tsv.

Comparison is on a broad transcription: stress marks removed, ʌ ɐ = ə,
ɪ = i, ʊ = u (quality of short vowels is not tested), the reference's r. = ɽ,
length after a nasal vowel ignored. Words: exact match. Sentences: phoneme
error rate (NFD code points) and exact match. Santali (Ol Chiki) and
Manipuri have no fallback row: espeak-ng reads Ol Chiki and Meetei Mayek
letters as their code points under every voice, so there is nothing to
compare with (for Manipuri in Bengali script, ``--bengali-voice`` adds the
reading of espeak-ng's Bengali voice, a different language, as a measure of
how far it is). Manipuri sentences are also scored per script.

    python tools/eval/bho_eval.py --lang bho --split dev --show improved
"""

from __future__ import annotations

import argparse
import json
import re
import unicodedata
from pathlib import Path

from common import align, load_oracle, run_uccharan

HERE = Path(__file__).resolve().parent


def norm(s: str, lang: str = "bho") -> str:
    s = unicodedata.normalize("NFD", s)
    if lang == "mag":
        # The Magahi set writes the labial approximant w (ʋ = w).
        s = s.replace("ʋ", "w")
    if lang == "doi":
        # Dogri tone: a grave or acute accent stands before the nasal mark.
        s = re.sub("(̃)([̀́])", r"\2\1", s)
    if lang == "mni":
        # Manipuri (Wiktionary notation): tone marks, syllable dots and the
        # unreleased mark go, the tie bar too; r = ɾ, ə = ɐ; ɐj ɐw are said
        # [ej] [ow] (Wikipedia "Meitei language") and a glide after a vowel
        # is written i u; a length mark after a consonant doubles it.
        s = re.sub("[̀́̚͡.‿]", "", s).replace("r", "ɾ").replace("ə", "ɐ")
        s = re.sub(r"([aeiouɐ])j(?![aeiouɐ])", r"\1i", s)
        s = re.sub(r"([aeiouɐ])w(?![aeiouɐ])", r"\1u", s)
        s = s.replace("ɐi", "ei").replace("ɐu", "ou")
        s = re.sub(r"([bdɡkmnŋplstɾ])(ʰ?)ː", r"\1\1\2", s)
    s = re.sub(r"[ˈˌ.?!,।]", lambda m: "ɽ" if m.group(0) == "." else "", s.replace("r.", "ɽ"))
    for a, b in (("ʌ", "ə"), ("ɐ", "ə"), ("ɪ", "i"), ("ʊ", "u"), ("ʱ", "ʰ"), ("̃ː", "̃"), ("ɦ", "h")):
        s = s.replace(a, b)
    s = re.sub(r"\([a-z-]+\)", "", s)
    # A geminate written with a length mark is a double consonant; aspirated
    # sonorants are consonant + h; ɕ = ʃ.
    s = re.sub(r"([bcdfɡhjklmnpqɾstʋwzɟʈɖɳʃɽ])(ʰ?)ː", r"\1\1\2", s)
    s = re.sub(r"([lmnɾ])ʰ", r"\1h", s).replace("ɕ", "ʃ")
    return " ".join(s.split())


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--lang", default="bho")
    ap.add_argument("--split", default="dev", choices=["dev", "heldout", "all"])
    ap.add_argument("--show", default="", help="print failures of this system (dev only)")
    ap.add_argument("--out", type=Path)
    ap.add_argument("--bengali-voice", action="store_true", help="Manipuri: add espeak-ng's Bengali voice (Bengali-script items)")
    args = ap.parse_args()
    rows = [l.split("\t") for l in (HERE / "gold" / args.lang / "gold.tsv").read_text(encoding="utf-8").splitlines() if l and not l.startswith("#")]
    rows = [r for r in rows if args.split in ("all", r[6])]
    heldout_words = {r[2] for r in (l.split("\t") for l in (HERE / "gold" / args.lang / "gold.tsv").read_text(encoding="utf-8").splitlines() if l and not l.startswith("#")) if r[1] == "word" and r[6] == "heldout"}
    oracle = load_oracle()
    systems = {}
    texts = [r[2] for r in rows]
    if oracle is not None and args.lang not in ("sat", "mni"):
        systems["espeak-ng hi (fallback)"] = [oracle.phonemize(t, "hi") for t in texts]
    if oracle is not None and args.lang == "mni" and args.bengali_voice:
        # Bengali-script items only: the Meetei Mayek ones are skipped (empty).
        beng = [r[4] in ("beng", "sentence-beng") for r in rows]
        systems["espeak-ng bn (Bengali voice)"] = [oracle.phonemize(t, "bn") if b else "" for t, b in zip(texts, beng)]
    systems["uccharan compatible"] = run_uccharan(texts, args.lang)
    systems["uccharan improved"] = run_uccharan(texts, args.lang, extra=("--improved",))
    res = {"lang": args.lang, "split": args.split, "systems": {}}
    print(f"{args.lang} gold ({args.split}): words {sum(r[1] == 'word' for r in rows)}, sentences {sum(r[1] == 'sentence' for r in rows)}")
    for name, outs in systems.items():
        cats: dict = {}
        wok = wn = sok = sn = 0
        err = tot = 0
        per_cat: dict = {}
        for r, o in zip(rows, outs):
            if name.startswith("espeak-ng bn") and r[4] not in ("beng", "sentence-beng"):
                continue
            got = norm(o, args.lang)
            wants = [norm(x, args.lang) for x in r[3].split("|")]
            want = min(wants, key=lambda w: align(list(w), list(got))[0])
            if r[1] == "word":
                ok = got in wants
                wok += ok
                wn += 1
                c = cats.setdefault(r[4], [0, 0])
                c[0] += ok
                c[1] += 1
            else:
                d, _ = align(list(want), list(got))
                err += d
                tot += len(want)
                pc = per_cat.setdefault(r[4], [0, 0])
                pc[0] += d
                pc[1] += len(want)
                ok = got == want
                sok += ok
                sn += 1
            if not ok and args.show and args.show in name and args.split == "dev":
                # Held-out words inside dev sentences are not shown.
                if r[1] == "sentence":
                    gw, ww = got.split(), want.split()
                    tw = r[2].replace("।", "").replace("?", "").replace(",", "").split()
                    diffs = [f"{t}: {g} want {w}" for t, g, w in zip(tw, gw, ww) if g != w and t not in heldout_words]
                    if diffs:
                        print("   FAIL", r[0], "|", "; ".join(diffs))
                else:
                    print("   FAIL", r[0], "|", got, "| want", want)
        res["systems"][name] = {"word_accuracy": wok / max(1, wn), "sentence_per": err / max(1, tot), "sentence_exact": sok / max(1, sn), "by_category": {k: v[0] / v[1] for k, v in cats.items()}}
        if len(per_cat) > 1:
            res["systems"][name]["sentence_per_by_category"] = {k: v[0] / max(1, v[1]) for k, v in per_cat.items()}
            res["systems"][name]["counts_by_category"] = {k: v[1] for k, v in cats.items()}
        print(f"  {name:28s} words {100 * wok / max(1, wn):5.1f}%  sentences PER {100 * err / max(1, tot):5.2f}%  exact {100 * sok / max(1, sn):5.1f}%")
        print("      " + "  ".join(f"{k} {100 * v[0] / v[1]:.1f}% of {v[1]}" for k, v in sorted(cats.items())))
        if len(per_cat) > 1:
            print("      " + "  ".join(f"{k} PER {100 * v[0] / max(1, v[1]):.2f}%" for k, v in sorted(per_cat.items())))
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / f"{args.lang}_gold_{args.split}.json").write_text(json.dumps(res, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
