#!/usr/bin/env python3
"""Score the transliterator through the `uccharan` command line.

    score.py words     --lang hi --pack data/packs/xlit-hi.ucd FILE.tsv
    score.py sentences --lang hi --pack PACK ROMAN.txt NATIVE.txt [--lines A:B]
    score.py hinglish  --lang hi --pack PACK tools/eval/gold/hinglish/test.tsv

`words`: FILE has a romanised word and its native references (`a|b`) per
line; prints top-1 and top-3 accuracy of each word alone. `sentences`: two
files of parallel lines, every Latin word converted, word accuracy by
position. `hinglish`: romanised sentence, reference with English left in
Latin; prints Indic words right, English words wrongly converted and Indic
words wrongly left in Latin.

Options: --bin PATH (default target/release/uccharan), --no-pack (the
behaviour without a pack), --options JSON (merged into --translit-options),
--errors (list the wrong tokens).

Strict = exact match of a token after stripping punctuation. Relaxed
(Devanagari) folds spelling variants of one word: chandrabindu = anusvara,
nukta ignored, nasal + virama before a consonant = anusvara (हिन्दी = हिंदी),
and the verb endings यी/ई, ये/ए after a written vowel (लिये = लिए; गयी / गई stay
different).
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PUNCT = "\"'“”‘’()[]{}.,;:!?।॥-–—…|/\\*"


def relax(w: str) -> str:
    w = unicodedata.normalize("NFC", w).replace("‌", "").replace("‍", "")
    w = w.replace("़", "").replace("ँ", "ं")
    w = unicodedata.normalize("NFD", w).replace("़", "")
    w = re.sub("[ङञणनम]्(?=[क-ह])", "ं", w)
    w = re.sub("(?<=[ा-ौअ-औ])यी", "ई", w)
    w = re.sub("(?<=[ा-ौअ-औ])ये", "ए", w)
    return w.replace("ऊँ", "ऊं")


def run(args: argparse.Namespace, lines: list[str], profile: str, options: dict) -> list[str]:
    options = {**options, **json.loads(args.options or "{}")}
    cmd = [args.bin, "-l", args.lang, "-p", profile, "--translit-options", json.dumps(options)]
    if not args.no_pack:
        cmd += ["--pack", args.pack]
    p = subprocess.run(cmd, input="\n".join(lines) + "\n", capture_output=True, text=True, encoding="utf8")
    if p.returncode != 0:
        sys.exit(p.stderr)
    out = p.stdout.split("\n")
    if out and out[-1] == "":
        out.pop()
    if len(out) != len(lines):
        sys.exit(f"{len(out)} output lines for {len(lines)} input lines")
    return out


def words(args: argparse.Namespace) -> None:
    rows = [l.rstrip("\n").split("\t") for l in open(args.files[0], encoding="utf8") if "\t" in l]
    out = run(args, [r[0] for r in rows], "native-json", {"english": "convert", "context": False, "alternatives": 3})
    top1 = top3 = 0
    for (roman, refs), line in zip(rows, out):
        spans = [s for s in json.loads(line)["spans"] if s["kind"] != "other"]
        alts = [a["text"] for a in spans[0].get("alternatives", [])] if spans else []
        refs = refs.split("|")
        top1 += bool(alts) and alts[0] in refs
        top3 += any(a in refs for a in alts[:3])
        if args.errors and not (alts and alts[0] in refs):
            print(roman, "ref", "|".join(refs), "got", " ".join(alts))
    n = len(rows)
    print(f"words={n} top1={100 * top1 / n:.2f} top3={100 * top3 / n:.2f}")


def sentences(args: argparse.Namespace) -> None:
    roman = open(args.files[0], encoding="utf8").read().split("\n")
    native = open(args.files[1], encoding="utf8").read().split("\n")
    if args.lines:
        a, b = args.lines.split(":")
        roman, native = roman[int(a):int(b)], native[int(a):int(b)]
    pairs = [(r, n) for r, n in zip(roman, native) if r.strip() and len(r.split()) == len(n.split())]
    out = run(args, [p[0] for p in pairs], "native", {"english": "convert", "context": not args.no_context})
    tot = ok = rel = 0
    for (src, ref), hyp in zip(pairs, out):
        r, h = ref.split(), hyp.split()
        h = (h + [""] * len(r))[: len(r)]
        for a, b, c in zip(src.split(), r, h):
            b, c = b.strip(PUNCT), c.strip(PUNCT)
            if not b or not any(ch.isalpha() for ch in a):
                continue
            tot += 1
            ok += b == c
            good = relax(b) == relax(c)
            rel += good
            if args.errors and not good:
                print(a, "ref", b, "got", c)
    print(f"sentences={len(pairs)} words={tot} strict={100 * ok / tot:.2f} relaxed={100 * rel / tot:.2f}")


def hinglish(args: argparse.Namespace) -> None:
    rows = [l.rstrip("\n").split("\t") for l in open(args.files[0], encoding="utf8") if l.strip() and not l.startswith("#")]
    out = run(args, [r[0] for r in rows], "native", {"context": not args.no_context})
    ind = ind_ok = ind_rel = ind_left = en = en_conv = sent_ok = 0
    for (src, ref), hyp in zip(rows, out):
        s, r, h = src.split(), ref.split(), hyp.split()
        h = (h + [""] * len(s))[: len(s)]
        good = True
        for a, b, c in zip(s, r, h):
            a, b, c = a.strip(PUNCT), b.strip(PUNCT), c.strip(PUNCT)
            if a == b:
                en += 1
                if c != a:
                    en_conv += 1
                    good = False
                    if args.errors:
                        print("english converted:", a, "->", c, "|", src)
            else:
                ind += 1
                ind_ok += c == b
                rel = relax(c) == relax(b)
                ind_rel += rel
                ind_left += c == a
                if not rel:
                    good = False
                    if args.errors:
                        print("indic:", a, "ref", b, "got", c, "|", src)
        sent_ok += good
    print(f"sentences={len(rows)} indic_words={ind} english_words={en}")
    print(f"indic_strict={100 * ind_ok / ind:.1f} indic_relaxed={100 * ind_rel / ind:.1f}")
    print(f"english_converted={en_conv}/{en}={100 * en_conv / max(en, 1):.1f}")
    print(f"indic_left_latin={ind_left}/{ind}={100 * ind_left / ind:.1f}")
    print(f"sentences_right={sent_ok}/{len(rows)}={100 * sent_ok / len(rows):.1f}")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("mode", choices=["words", "sentences", "hinglish"])
    ap.add_argument("files", nargs="+")
    ap.add_argument("--lang", default="hi")
    ap.add_argument("--pack", default="")
    ap.add_argument("--bin", default=str(ROOT / "target/release/uccharan"))
    ap.add_argument("--options", default="")
    ap.add_argument("--lines", default="")
    ap.add_argument("--no-pack", action="store_true")
    ap.add_argument("--no-context", action="store_true")
    ap.add_argument("--errors", action="store_true")
    args = ap.parse_args()
    if not args.pack and not args.no_pack:
        args.pack = str(ROOT / f"data/packs/xlit-{args.lang}.ucd")
    {"words": words, "sentences": sentences, "hinglish": hinglish}[args.mode](args)


if __name__ == "__main__":
    main()
