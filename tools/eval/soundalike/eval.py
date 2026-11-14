#!/usr/bin/env python3
"""Evaluate the sound-alike matcher.

    python tools/eval/soundalike/eval.py --split dev
    python tools/eval/soundalike/eval.py --split heldout --thresholds uccharan=0.80,chars=0.75,metaphone=0.9
    python tools/eval/soundalike/eval.py --speed

Test cases are recogniser transcripts of sentences that contain a vocabulary
term (``data/confusions.jsonl``, made by ``make_confusions.py``), hand-written
mishearings (``terms/mishearings.tsv``) and sentences without any term. The
whole vocabulary (both splits) is loaded for every case. Three systems are
scored with the same span search: uccharan's phoneme matcher (through the
command-line program), plain character edit distance, and Metaphone codes.
`uccharan-plain` is the matcher without the spoken-form hints of the term
sets; `uccharan-words` is the matcher given the words that occur twice or
more in the development sentences as ordinary words (meaningful on the
held-out split only, whose sentences are disjoint from those).

For a threshold t a system *corrects* every non-overlapping candidate with a
score of at least t (for uccharan, a candidate whose span is of the
`uncertain` or `ordinary` kind also needs `--uncertain` / `--ordinary`).

* recovery: of the transcripts in which the term is not written correctly
  (its words, compared without case, are not in the transcript), the share
  in which it is put back;
* wrong corrections: corrections to any other term, in transcripts with a
  term;
* false-correction rate: the share of transcripts without a term that are
  changed at all.

On the development split the script sweeps t and reports, for each system,
the lowest t whose false-correction rate is at most ``--max-false`` (1 %;
the suggested default). On the held-out split it scores the thresholds it is
given. The baselines need ``rapidfuzz`` and ``jellyfish``.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
# The word tables are packs; inside the repository they are read from the source tree.
os.environ.setdefault("UCCHARAN_DATA_DIR", os.pathsep.join(str(ROOT / "crates" / d / "data") for d in ("uccharan", "uccharan-data")))
MAX_WORDS = 3
FLOOR = 0.5
PUNCT = set("।॥،؛؟۔")


def word_char(ch: str) -> bool:
    return (ch.isalnum() or "\u0300" <= ch <= "\u036f" or "\u0900" <= ch <= "\u097f" or "\u0600" <= ch <= "\u06ff") and ch not in PUNCT


def tokens(text: str) -> list[tuple[int, int]]:
    """Words as (start, end): letters, digits and combining signs; an
    apostrophe between letters stays in the word, a final 's does not."""
    out = []
    start = None
    n = len(text)
    skip = -1
    for i, ch in enumerate(text):
        if i == skip:
            continue
        inner = ch in "'’" and start is not None and i + 1 < n and text[i + 1].isalpha()
        if inner and text[i + 1] in "sS" and (i + 2 >= n or not word_char(text[i + 2])):
            inner = False
            skip = i + 1
        if word_char(ch) or inner:
            if start is None:
                start = i
        elif start is not None:
            out.append((start, i))
            start = None
    if start is not None:
        out.append((start, n))
    return out


def find_binary(explicit: str | None) -> str:
    for c in (explicit, os.environ.get("UCCHARAN_BIN"), str(ROOT / "target" / "release" / "uccharan"), shutil.which("uccharan")):
        if c and Path(c).exists():
            return c
    raise SystemExit("uccharan binary not found: run `cargo build --release` or set UCCHARAN_BIN")


def split_of(term: str) -> str:
    return "dev" if hashlib.sha1(term.lower().encode()).digest()[0] % 2 == 0 else "heldout"


def read_terms() -> list[dict]:
    out = []
    for kind, name in (("name", "names.tsv"), ("brand", "brands.tsv"), ("jargon", "jargon.tsv")):
        for line in (HERE / "terms" / name).read_text(encoding="utf-8").splitlines():
            if not line.strip() or line.startswith("#"):
                continue
            cols = line.split("\t")
            t = {"term": cols[0], "kind": kind}
            if kind == "name":
                t["hi"] = cols[2]
            elif len(cols) > 1 and cols[1].strip():
                t["say"] = cols[1].strip()
            out.append(t)
    return out


def script_of(text: str) -> str:
    for c in text:
        if "ऀ" <= c <= "ॿ":
            return "deva"
        if "؀" <= c <= "ۿ":
            return "arab"
    return "latin"


def load_cases(split: str, terms: list[dict]) -> list[dict]:
    """Test cases of a split. `vocab` says which vocabulary a case is scored
    against: `latin` (all terms as the term sets spell them) or `deva`
    (personal names in Devanagari)."""
    kind_of = {t["term"]: t["kind"] for t in terms}
    hi_of = {t["term"]: t["hi"] for t in terms if "hi" in t}
    has_say = {t["term"] for t in terms if "say" in t}
    cases: list[dict] = []
    seen = set()

    def add(**c):
        key = (c["text"], c.get("term"), c["lang"], c["vocab"])
        if c["text"].strip() and key not in seen:
            seen.add(key)
            cases.append(c)

    path = HERE / "data" / "confusions.jsonl"
    rows = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()] if path.exists() else []
    # Sentences without a term, as written.
    for line in (HERE / "data" / "items.jsonl").read_text(encoding="utf-8").splitlines():
        it = json.loads(line)
        if it["kind"] == "none" and it["split"] == split:
            add(text=it["ref"], lang="hi" if it["ctx"] != "en" else "en-us", ctx=it["ctx"], kind="none", source="clean", vocab="latin")
    for r in rows:
        if r["split"] != split:
            continue
        ctx = r["ctx"]
        for asr, hyp in r["hyp"].items():
            lang = "en-us" if ctx == "en" or asr.endswith("-en") else "hi"
            common = {"text": hyp, "lang": lang, "ctx": ctx, "kind": r["kind"], "source": asr}
            if r["kind"] == "none":
                add(**common, vocab="latin")
                continue
            add(**common, term=r["term"], vocab="latin", hint=r["term"] in has_say)
            # The other direction: a vocabulary of names in Devanagari
            # against Latin-script transcripts.
            if r["kind"] == "name" and script_of(hyp) == "latin":
                add(**common, term=hi_of[r["term"]], vocab="deva", hint=False)
    for line in (HERE / "terms" / "mishearings.tsv").read_text(encoding="utf-8").splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        heard, term, lang = line.split("\t")
        if split_of(term) != split:
            continue
        text = f"I think they said {heard} at the end." if lang == "en" else f"{heard} के बारे में बताइए।"
        add(text=text, lang="en-us" if lang == "en" else "hi", ctx=lang, kind=kind_of[term], source="hand", term=term, vocab="latin", hint=term in has_say)
    for c in cases:
        c["script"] = script_of(c["text"])
    return cases


def write_vocab(terms: list[dict], directory: Path) -> dict[str, Path]:
    latin = directory / "vocab.tsv"
    rows = ["# uccharan wordlist 1", "word\tlang\tipa\tsay"]
    rows += [f"{t['term']}\t\t\t{t.get('say', '')}" for t in terms]
    latin.write_text("\n".join(rows) + "\n", encoding="utf-8")
    plain = directory / "vocab-plain.txt"
    plain.write_text("\n".join(t["term"] for t in terms) + "\n", encoding="utf-8")
    deva = directory / "vocab-deva.txt"
    deva.write_text("\n".join(t["hi"] for t in terms if "hi" in t) + "\n", encoding="utf-8")
    return {"latin": latin, "plain": plain, "deva": deva}


# ---------------------------------------------------------------------------
# Systems: each returns, per case, a list of (start, end, term, score, exact,
# least score at which the candidate may be applied).


def frequent_words(min_count: int = 2) -> list[str]:
    """Words that occur at least `min_count` times in the development
    sentences (`tools/eval/data/{en,hi}_dev.tsv`): what a deployment would
    pass as `ordinary_words` for its own domain. The held-out sentences are
    disjoint from these."""
    counts: dict[str, int] = {}
    for name in ("en_dev.tsv", "hi_dev.tsv"):
        for line in (HERE.parent / "data" / name).read_text(encoding="utf-8").splitlines():
            sentence = line.split("\t", 1)[-1]
            for a, b in tokens(sentence):
                w = sentence[a:b].lower()
                counts[w] = counts.get(w, 0) + 1
    return sorted(w for w, n in counts.items() if n >= min_count and not w.isdigit())


def run_uccharan(binary: str, cases: list[dict], vocab: Path, options: dict | None = None, uncertain: float = 0.9, ordinary: float = 0.92,
                 index_options: dict | None = None) -> list[list[tuple]]:
    out: list[list[tuple]] = [[] for _ in cases]
    opts = {"min_score": FLOOR, "max_words": MAX_WORDS, **(options or {})}
    for lang in sorted({c["lang"] for c in cases}):
        idx = [i for i, c in enumerate(cases) if c["lang"] == lang]
        text = "\n".join(cases[i]["text"].replace("\n", " ") for i in idx) + "\n"
        res = subprocess.run([binary, "-l", lang, "--match-terms", str(vocab), "--match-options", json.dumps(opts),
                              "--index-options", json.dumps(index_options or {}, ensure_ascii=False)],
                             input=text, capture_output=True, text=True, check=True).stdout.split("\n")
        for i, line in zip(idx, res):
            for c in json.loads(line)["candidates"]:
                # Spans whose words do not look misheard need a higher score.
                need = {"unknown": 0.0, "uncertain": uncertain, "ordinary": ordinary}[c["kind"]]
                out[i].append((c["src"]["char"][0], c["src"]["char"][1], c["term"], c["score"], c["exact"], need))
    return out


def spans(text: str):
    toks = tokens(text)
    for i in range(len(toks)):
        for n in range(1, MAX_WORDS + 1):
            if i + n > len(toks):
                break
            if n > 1:
                gap = text[toks[i + n - 2][1]:toks[i + n - 1][0]]
                if len(gap) > 3 or any(not (ch.isspace() or ch in "-,.'’\"") for ch in gap):
                    break
            yield toks[i][0], toks[i + n - 1][1]


def word_key(text: str) -> str:
    """The words of a text in lower case, joined by single spaces."""
    return " ".join(text[a:b].lower() for a, b in tokens(text))


def squash(s: str) -> str:
    return "".join(ch for ch in s.lower() if ch.isalnum() or "ऀ" <= ch <= "ॿ")


def resolve(found: list[tuple]) -> list[tuple]:
    found.sort(key=lambda c: (not c[4], -c[3], -(c[1] - c[0]), c[0]))
    kept: list[tuple] = []
    for c in found:
        if all(c[1] <= k[0] or c[0] >= k[1] for k in kept):
            kept.append(c)
    return sorted(kept)


def run_baseline(kind: str, cases: list[dict], terms: list[dict], vocab: str) -> list[list[tuple]]:
    from rapidfuzz import process
    from rapidfuzz.distance import Levenshtein

    if kind == "metaphone":
        import jellyfish

        def code(s: str) -> str:
            s = re.sub(r"[^A-Za-z ]", " ", s)
            return jellyfish.metaphone(s).replace(" ", "") if s.strip() else ""
    else:
        code = squash
    keys: list[str] = []
    owner: list[str] = []
    for t in terms:
        if vocab == "deva":
            if "hi" not in t:
                continue
            forms, word = [t["hi"]], t["hi"]
        else:
            forms, word = [t["term"]] + ([t["say"]] if "say" in t else []), t["term"]
        for f in forms:
            k = code(f)
            if k:
                keys.append(k)
                owner.append(word)
    exact = {word_key(w): w for w in owner}
    out = []
    for c in cases:
        found = []
        text = c["text"]
        for s, e in spans(text):
            piece = text[s:e]
            if word_key(piece) in exact:
                found.append((s, e, exact[word_key(piece)], 1.0, True, 0.0))
                continue
            k = code(piece)
            if not k or not keys:
                continue
            hit = process.extractOne(k, keys, scorer=Levenshtein.normalized_similarity, score_cutoff=FLOOR)
            if hit:
                found.append((s, e, owner[hit[2]], float(hit[1]), False, 0.0))
        out.append(resolve(found))
    return out


# ---------------------------------------------------------------------------
# Scoring


def score(cases: list[dict], cands: list[list[tuple]], t: float, where=lambda c: True) -> dict:
    pos = corrupted = recovered = wrong = neg = neg_changed = neg_changes = 0
    for c, cs in zip(cases, cands):
        if not where(c):
            continue
        applied = [x for x in cs if not x[4] and x[3] >= max(t, x[5])]
        if c.get("term") is None:
            neg += 1
            neg_changed += bool(applied)
            neg_changes += len(applied)
            continue
        pos += 1
        target = c["term"]
        # The term is already there when its words are, in any case.
        present = f" {word_key(target)} " in f" {word_key(c['text'])} "
        wrong += sum(1 for x in applied if x[2] != target)
        if not present:
            corrupted += 1
            recovered += any(x[2] == target for x in applied)
    made = recovered + wrong + neg_changes
    return {
        "with_term": pos, "term_already_right": pos - corrupted, "corrupted": corrupted, "recovered": recovered,
        "recovery": round(recovered / corrupted, 4) if corrupted else None,
        "wrong_corrections": wrong,
        "without_term": neg, "false_corrected": neg_changed,
        "false_correction_rate": round(neg_changed / neg, 4) if neg else None,
        "precision": round(recovered / made, 4) if made else None,
    }


GROUPS = {
    "English transcripts, Latin vocabulary": lambda c: c["vocab"] == "latin" and c["script"] == "latin" and c["ctx"] == "en",
    "Hindi sentences, Devanagari transcript, Latin vocabulary": lambda c: c["vocab"] == "latin" and c["script"] == "deva",
    "Hindi sentences, Arabic-script transcript, Latin vocabulary": lambda c: c["vocab"] == "latin" and c["script"] == "arab",
    "Hindi and Hinglish sentences, Latin transcript, Latin vocabulary": lambda c: c["vocab"] == "latin" and c["script"] == "latin" and c["ctx"] != "en",
    "Latin transcripts, Devanagari vocabulary (names)": lambda c: c["vocab"] == "deva",
    "names": lambda c: c["vocab"] == "latin" and c["kind"] == "name",
    "brands": lambda c: c["vocab"] == "latin" and c["kind"] == "brand",
    "jargon": lambda c: c["vocab"] == "latin" and c["kind"] == "jargon",
    "terms with a spoken-form hint": lambda c: c["vocab"] == "latin" and c.get("hint") is True,
    "terms without a hint": lambda c: c["vocab"] == "latin" and c.get("hint") is False,
    "hand-written mishearings": lambda c: c["source"] == "hand",
    "recogniser transcripts": lambda c: c["source"] not in ("hand", "clean"),
    "sentences as written (no term)": lambda c: c["source"] == "clean",
    "English sentences without a term": lambda c: c["kind"] == "none" and c["ctx"] == "en",
    "Hindi and Hinglish sentences without a term": lambda c: c["kind"] == "none" and c["ctx"] != "en",
}


def evaluate(name: str, cases: list[dict], cands: list[list[tuple]], threshold: float | None, max_false: float) -> dict:
    grid = [round(0.50 + 0.01 * k, 2) for k in range(51)]
    curve = {str(t): score(cases, cands, t) for t in grid}
    if threshold is None:
        ok = [t for t in grid if (curve[str(t)]["false_correction_rate"] or 0) <= max_false]
        threshold = ok[0] if ok else 1.0
    res = {"system": name, "threshold": threshold, "overall": score(cases, cands, threshold),
           "groups": {g: score(cases, cands, threshold, f) for g, f in GROUPS.items()},
           "curve": {t: {k: v[k] for k in ("recovery", "precision", "false_correction_rate", "wrong_corrections")} for t, v in curve.items() if int(round(float(t) * 100)) % 5 == 0 and float(t) >= 0.7}}
    return res


def show(res: dict) -> None:
    o = res["overall"]
    print(f"\n== {res['system']} at threshold {res['threshold']}")
    print(f"   recovery {o['recovery']} ({o['recovered']}/{o['corrupted']}), precision {o['precision']}, wrong corrections {o['wrong_corrections']}, "
          f"false-correction rate {o['false_correction_rate']} ({o['false_corrected']}/{o['without_term']})")
    for g, s in res["groups"].items():
        if s["with_term"] or s["without_term"]:
            print(f"   {g:64s} recovery {s['recovery']} ({s['recovered']}/{s['corrupted']}; already right {s['term_already_right']}) wrong {s['wrong_corrections']}"
                  + (f" false {s['false_corrected']}/{s['without_term']}" if s["without_term"] else ""))
    print("   threshold: recovery / precision / false-correction rate")
    for t, v in res["curve"].items():
        print(f"     {t}: {v['recovery']} / {v['precision']} / {v['false_correction_rate']} (wrong corrections {v['wrong_corrections']})")


def speed(binary: str, terms: list[dict], directory: Path) -> dict:
    """Throughput against vocabularies of 1k, 10k and 50k terms: the real
    terms plus given name + surname pairs and English words."""
    given = [t["term"] for t in terms if t["kind"] == "name"]
    pairs = [f"{a} {b}" for i, a in enumerate(given) for j, b in enumerate(given) if (i * 31 + j * 17) % 2 == 0 and a != b]
    words = []
    for f in ("en_words_rare.txt", "en_words_oov.txt", "en_words_frequent.txt"):
        p = HERE.parent / "data" / f
        if p.exists():
            words += [w.strip() for w in p.read_text(encoding="utf-8").split() if len(w.strip()) > 4]
    base = [t["term"] for t in terms]
    pool = list(dict.fromkeys(base + [x for pair in zip(pairs, words * (len(pairs) // max(len(words), 1) + 1)) for x in pair] + pairs))
    lines = [c["text"] for c in load_cases("heldout", terms) if c["vocab"] == "latin"]
    lines = (lines * (3000 // max(len(lines), 1) + 1))[:3000]
    out = {}
    for n in (len(base), 1000, 10000, 50000):
        vocab = directory / f"speed-{n}.txt"
        vocab.write_text("\n".join(pool[:n]) + "\n", encoding="utf-8")
        best = None
        for lang in ("en-us",):
            for _ in range(3):
                r = subprocess.run([binary, "-l", lang, "--match-terms", str(vocab), "--match-stats"], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True)
                stats = dict(zip(r.stderr.split()[::2], r.stderr.split()[1::2]))
                if best is None or float(stats["seconds"]) < float(best["seconds"]):
                    best = stats
        assert best is not None
        out[str(n)] = {"terms": int(best["terms"]), "sequences": int(best["sequences"]), "build_ms": float(best["build_ms"]),
                       "transcripts_per_s": float(best["lines_per_s"]), "words_per_s": float(best["words_per_s"]),
                       "span_lookups_per_s": round(float(best["words_per_s"]) * MAX_WORDS)}
        print(n, json.dumps(out[str(n)]))
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--split", choices=("dev", "heldout"), default="dev")
    ap.add_argument("--systems", default="uccharan,uccharan-plain,chars,metaphone")
    ap.add_argument("--thresholds", default="", help="system=threshold,... (required for the held-out split)")
    ap.add_argument("--max-false", type=float, default=0.01)
    ap.add_argument("--uncertain", type=float, default=0.9, help="score needed by a span of the `uncertain` kind (uccharan)")
    ap.add_argument("--ordinary", type=float, default=0.92, help="score needed by a span of ordinary words (uccharan)")
    ap.add_argument("--options", default="", help="JSON matching options for the uccharan systems")
    ap.add_argument("--speed", action="store_true")
    ap.add_argument("--uccharan-bin")
    ap.add_argument("--out", type=Path)
    ap.add_argument("--dump", type=Path, help="write every case with its candidates (uccharan) as JSON lines")
    args = ap.parse_args()
    binary = find_binary(args.uccharan_bin)
    terms = read_terms()
    with tempfile.TemporaryDirectory() as d:
        directory = Path(d)
        if args.speed:
            res = speed(binary, terms, directory)
            if args.out:
                args.out.write_text(json.dumps(res, indent=1) + "\n", encoding="utf-8")
            return 0
        vocab = write_vocab(terms, directory)
        cases = load_cases(args.split, terms)
        given = dict(x.split("=") for x in args.thresholds.split(",") if x)
        if args.split == "heldout" and not given:
            raise SystemExit("--thresholds is required for the held-out split (choose them on the development split)")
        options = json.loads(args.options) if args.options else None
        results = []
        for name in args.systems.split(","):
            t0 = time.time()
            cands: list[list[tuple]] = [[] for _ in cases]
            for v in ("latin", "deva"):
                idx = [i for i, c in enumerate(cases) if c["vocab"] == v]
                sub = [cases[i] for i in idx]
                if name == "uccharan":
                    got = run_uccharan(binary, sub, vocab[v], options, args.uncertain, args.ordinary)
                elif name == "uccharan-words":
                    got = run_uccharan(binary, sub, vocab[v], options, args.uncertain, args.ordinary, {"ordinary_words": frequent_words()})
                elif name == "uccharan-plain":
                    got = run_uccharan(binary, sub, vocab["plain" if v == "latin" else v], options, args.uncertain, args.ordinary)
                else:
                    got = run_baseline(name, sub, terms, v)
                for i, g in zip(idx, got):
                    cands[i] = g
            res = evaluate(name, cases, cands, float(given[name]) if name in given else None, args.max_false)
            res["seconds"] = round(time.time() - t0, 1)
            show(res)
            results.append(res)
            if args.dump and name == "uccharan":
                with args.dump.open("w", encoding="utf-8") as f:
                    for c, cs in zip(cases, cands):
                        f.write(json.dumps({**c, "candidates": cs}, ensure_ascii=False) + "\n")
        if args.out:
            args.out.write_text(json.dumps({"split": args.split, "cases": len(cases), "results": results}, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
