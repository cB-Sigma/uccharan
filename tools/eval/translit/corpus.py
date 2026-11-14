#!/usr/bin/env python3
"""Word and word-pair counts of a native-script corpus, for the transliteration packs.

    XLIT_DATA=DIR python3 tools/eval/translit/corpus.py hi SENTENCES... [--extra FILE:WEIGHT]

Writes DIR/<la>/freq.tsv (word, count; count >= 2) and DIR/<la>/bigrams.tsv
(word, word, count; count >= 3). Sentence files are Leipzig style
(number TAB sentence) or Tatoeba style (id TAB sentence).
"""
import collections
import os
import re
import sys
import unicodedata

# Where the prepared data lives: DIR/<lang>/, DIR/raw/ (see data/sources/translit/README.md).
DATA = os.environ.get("XLIT_DATA") or os.path.join(os.path.dirname(os.path.abspath(__file__)), ".work")
BLOCK = {"hi": 0x0900, "mr": 0x0900, "bn": 0x0980, "pa": 0x0A00, "gu": 0x0A80,
         "ta": 0x0B80, "te": 0x0C00, "kn": 0x0C80, "ml": 0x0D00}


def main():
    la = sys.argv[1]
    base = BLOCK[la]
    files = []
    args = sys.argv[2:]
    i = 0
    while i < len(args):
        if args[i] == "--extra":
            f, w = args[i + 1].rsplit(":", 1)
            files.append((f, int(w)))
            i += 2
        else:
            files.append((args[i], 1))
            i += 1
    word = re.compile("[%s-%s‌‍]+" % (chr(base), chr(base + 0x7f)))
    digits = re.compile("[%s-%s]" % (chr(base + 0x66), chr(base + 0x6f)))
    uni = collections.Counter()
    bi = collections.Counter()
    for path, weight in files:
        for line in open(path, encoding="utf8", errors="replace"):
            p = line.rstrip("\n").split("\t")
            text = unicodedata.normalize("NFC", p[1] if len(p) > 1 else p[0])
            prev = None
            for tok in re.split(r"(\s+)", text):
                if not tok or tok.isspace():
                    continue
                m = word.search(tok)
                # A token counts when its letters are one native-script run
                # (punctuation around it allowed); anything else breaks the pair chain.
                core = tok.strip("\"'“”‘’()[]{}.,;:!?।॥-–—…|/\\*")
                if not m or m.group(0) != core or digits.search(core) or len(core) > 30:
                    prev = None
                    continue
                core = core.replace("‌", "").replace("‍", "") if la in ("hi",) else core
                uni[core] += weight
                if prev is not None:
                    bi[(prev, core)] += weight
                prev = core if tok[-1] not in ".,;:!?।" else None
    out = os.path.join(DATA, la)
    os.makedirs(out, exist_ok=True)
    total = sum(uni.values())
    with open(os.path.join(out, "freq.tsv"), "w", encoding="utf8") as f:
        f.write(f"#total\t{total}\n")
        for w, c in uni.most_common():
            if c < 2:
                break
            f.write(f"{w}\t{c}\n")
    with open(os.path.join(out, "bigrams.tsv"), "w", encoding="utf8") as f:
        for (a, b), c in bi.most_common():
            if c < 3:
                break
            f.write(f"{a}\t{b}\t{c}\n")
    print(la, "tokens", total, "types", len(uni), "pairs", len(bi))


if __name__ == "__main__":
    main()
