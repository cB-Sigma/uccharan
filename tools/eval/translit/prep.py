#!/usr/bin/env python3
"""Prepare the word pairs of one language for the transliteration packs.

    XLIT_DATA=DIR python3 tools/eval/translit/prep.py hi

Reads   DIR/raw/dakshina/dakshina_dataset_v1.0/<la>/lexicons/*.tsv
        DIR/raw/aksharantar/<iso3>.zip   (downloaded if missing, deleted after)
Writes  DIR/<la>/train.tsv            native, roman, source, count
        DIR/<la>/{dev,test}_dak.tsv   roman, native|native..   (Dakshina dev / test lexicon)
        DIR/<la>/{dev,test}_ak.tsv    roman, native|native..   (Aksharantar valid / test)
        DIR/<la>/test_ak_{native,ne}.tsv  the manually made parts of the Aksharantar test set:
                                      frequent / uniformly sampled words, and named entities
        DIR/<la>/split.json           counts and the rule used

The split is fixed here: a training pair is dropped when its native word OR
its romanised word occurs in any of the four dev / test files, so every dev /
test input and every reference is unseen by the model and the lexicon.
"""
import collections
import io
import json
import os
import subprocess
import sys
import unicodedata
import zipfile

# Where the prepared data lives: DIR/<lang>/, DIR/raw/ (see data/sources/translit/README.md).
DATA = os.environ.get("XLIT_DATA") or os.path.join(os.path.dirname(os.path.abspath(__file__)), ".work")
ISO3 = {"hi": "hin", "mr": "mar", "bn": "ben", "ta": "tam", "te": "tel",
        "gu": "guj", "pa": "pan", "kn": "kan", "ml": "mal"}


def norm_native(w):
    w = unicodedata.normalize("NFC", w.strip())
    return w.replace("‌", "").replace("‍", "") if False else w


def norm_roman(w):
    return w.strip().lower()


def ok_roman(w):
    return 0 < len(w) <= 40 and all("a" <= c <= "z" for c in w)


def read_dak(path):
    rows = []
    for line in open(path, encoding="utf8"):
        p = line.rstrip("\n").split("\t")
        if len(p) < 3:
            continue
        rows.append((norm_native(p[0]), norm_roman(p[1]), int(p[2])))
    return rows


def main():
    la = sys.argv[1]
    iso = ISO3[la]
    out = os.path.join(DATA, la)
    os.makedirs(out, exist_ok=True)
    os.makedirs(os.path.join(DATA, "raw/aksharantar"), exist_ok=True)
    dak = os.path.join(DATA, "raw/dakshina/dakshina_dataset_v1.0", la, "lexicons")
    d_train = read_dak(os.path.join(dak, f"{la}.translit.sampled.train.tsv"))
    d_dev = read_dak(os.path.join(dak, f"{la}.translit.sampled.dev.tsv"))
    d_test = read_dak(os.path.join(dak, f"{la}.translit.sampled.test.tsv"))

    zpath = os.path.join(DATA, "raw/aksharantar", iso + ".zip")
    if not os.path.exists(zpath):
        subprocess.check_call([
            "curl", "-sS", "-L", "-m", "1200",
            f"https://huggingface.co/datasets/ai4bharat/Aksharantar/resolve/main/{iso}.zip",
            "-o", zpath])
    z = zipfile.ZipFile(zpath)
    ak = {}
    for part in ("train", "valid", "test"):
        name = [n for n in z.namelist() if n.endswith(f"{iso}_{part}.json")][0]
        rows = []
        with z.open(name) as f:
            for line in io.TextIOWrapper(f, encoding="utf8"):
                d = json.loads(line)
                rows.append((norm_native(d["native word"]), norm_roman(d["english word"]),
                             d.get("source", "?")))
        ak[part] = rows

    held_native, held_roman = set(), set()
    for rows in (d_dev, d_test):
        for n, r, _ in rows:
            held_native.add(n)
            held_roman.add(r)
    for part in ("valid", "test"):
        for n, r, _ in ak[part]:
            held_native.add(n)
            held_roman.add(r)

    def write_eval(name, pairs):
        by = collections.OrderedDict()
        for n, r in pairs:
            if not ok_roman(r):
                continue
            by.setdefault(r, [])
            if n not in by[r]:
                by[r].append(n)
        with open(os.path.join(out, name), "w", encoding="utf8") as f:
            for r, ns in by.items():
                f.write(r + "\t" + "|".join(ns) + "\n")
        return len(by)

    stats = {"lang": la, "rule": "train pair dropped if its native or roman word is in any dev/test file"}
    stats["dev_dak"] = write_eval("dev_dak.tsv", [(n, r) for n, r, _ in d_dev])
    stats["test_dak"] = write_eval("test_dak.tsv", [(n, r) for n, r, _ in d_test])
    stats["dev_ak"] = write_eval("dev_ak.tsv", [(n, r) for n, r, _ in ak["valid"]])
    stats["test_ak"] = write_eval("test_ak.tsv", [(n, r) for n, r, _ in ak["test"]])
    # The two parts of the Aksharantar test set that its authors report.
    stats["test_ak_native"] = write_eval("test_ak_native.tsv", [(n, r) for n, r, s in ak["test"] if s in ("AK-Freq", "AK-Uni")])
    stats["test_ak_ne"] = write_eval("test_ak_ne.tsv", [(n, r) for n, r, s in ak["test"] if s in ("AK-NEI", "AK-NEF")])

    seen = {}
    dropped = collections.Counter()
    src_count = collections.Counter()
    for n, r, c in d_train:
        if n in held_native or r in held_roman:
            dropped["dakshina"] += 1
            continue
        if not ok_roman(r) or not n:
            continue
        seen[(n, r)] = ["Dakshina", c]
    for n, r, s in ak["train"]:
        if n in held_native or r in held_roman:
            dropped[s] += 1
            continue
        if not ok_roman(r) or not n or " " in n:
            continue
        if (n, r) in seen:
            continue
        seen[(n, r)] = [s, 1]
    with open(os.path.join(out, "train.tsv"), "w", encoding="utf8") as f:
        for (n, r), (s, c) in seen.items():
            src_count[s] += 1
            f.write(f"{n}\t{r}\t{s}\t{c}\n")
    stats["train"] = dict(src_count)
    stats["dropped_for_overlap"] = dict(dropped)
    json.dump(stats, open(os.path.join(out, "split.json"), "w"), indent=1, ensure_ascii=False)
    print(json.dumps(stats, ensure_ascii=False))
    z.close()
    os.remove(zpath)


if __name__ == "__main__":
    main()
