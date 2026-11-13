#!/usr/bin/env python3
"""Agreement between uccharan and the reference phonemizer (an espeak-ng the
user installed; see tools/oracle).

For each language this measures, on the sets in tools/eval/data:

* sentence level, phonemizer shape (words separated by spaces, no punctuation):
  phoneme error rate = edit distance over NFD codepoints / reference length,
  with and without stress marks, exact-sentence match;
* sentence level, Piper shape (one string per sentence with clause
  terminators): PER and exact match of the codepoint stream;
* a breakdown of edit operations by class, of word errors by word class
  (function word / lexicon word / out-of-lexicon word) and by whether the
  sentence contains digits;
* word lists: PER, exact match and stress-pattern errors for frequent,
  rare-but-in-lexicon and out-of-lexicon words.

Reference output is produced at run time and is never written to the
repository; only aggregate numbers and a few example lines go to the result
file.

    python tools/eval/run_eval.py --lang en-us --set heldout --out tools/eval/results
"""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter
from pathlib import Path

from common import ROOT, VOICES, align, load_oracle, nfd, op_class, read_tsv, run_uccharan, stress_pattern, strip_stress

DATA = Path(__file__).resolve().parent / "data"

# Closed-class English words (hand-written for the breakdown only).
FUNCTION_WORDS = set(
    """a an the and or but nor so yet if as than that which who whom whose what when where why how
    i me my mine you your yours he him his she her hers it its we us our ours they them their theirs
    this these those there here am is are was were be been being have has had do does did will would
    shall should can could may might must to of in on at by for from with into onto upon about over
    under after before between through during without within against among not no n't some any each
    every all both either neither such own same too very just only also then once""".split()
)


def sentence_piper_ref(oracle, text: str, voice: str) -> str:
    out = []
    for ipa, term in oracle.clauses(text, voice):
        out.append(ipa + term)
    s = " ".join(out)
    return re.sub(r"\([a-z-]+\)", "", s)


def source_words(text: str) -> list[str]:
    return [w for w in re.split(r"\s+", text) if re.search(r"\w", w)]


def evaluate_sentences(rows, lang, oracle, binary, examples=12):
    voice = VOICES[lang]
    texts = [t for _, t in rows]
    hyp = run_uccharan(texts, lang, binary=binary)
    hyp_piper = run_uccharan(texts, lang, "piper", punctuation=True, binary=binary)
    cmu = set()
    if lang != "hi":
        for line in (ROOT / "data/sources/cmudict/cmudict.dict").read_text(encoding="utf-8").splitlines():
            cmu.add(line.split(" ", 1)[0].split("(")[0])
    tot = Counter()
    ops_by_class = Counter()
    confusions = Counter()
    word_class = {k: Counter() for k in ("function", "lexicon", "oov")}
    subset = {k: Counter() for k in ("with-digits", "no-digits")}
    worst = []
    for (sid, text), h, hp in zip(rows, hyp, hyp_piper):
        ref = re.sub(r"\([a-z-]+\)", "", oracle.phonemize(text, voice))
        ref = " ".join(ref.split())
        r, hh = nfd(ref), nfd(h)
        d, ops = align(r, hh)
        tot["ref"] += len(r)
        tot["err"] += d
        tot["sent"] += 1
        tot["exact"] += d == 0
        rs, hs = nfd(strip_stress(ref)), nfd(strip_stress(h))
        ds, _ = align(rs, hs)
        tot["ref_nostress"] += len(rs)
        tot["err_nostress"] += ds
        tot["exact_nostress"] += ds == 0
        for op in ops:
            ops_by_class[op_class(op, lang)] += 1
            confusions[f"{op[1] or '∅'}→{op[2] or '∅'}"] += 1
        sub = subset["with-digits" if re.search(r"[0-9०-९]", text) else "no-digits"]
        sub["ref"] += len(r)
        sub["err"] += d
        sub["sent"] += 1
        sub["exact"] += d == 0
        # Piper shape
        rp = nfd(sentence_piper_ref(oracle, text, voice))
        hpl = nfd(hp)
        dp, _ = align(rp, hpl)
        tot["piper_ref"] += len(rp)
        tot["piper_err"] += dp
        tot["piper_exact"] += dp == 0
        # word classes (only when the word counts line up)
        rw, hw, sw = ref.split(), h.split(), source_words(text)
        if len(rw) == len(hw) == len(sw):
            tot["aligned_sent"] += 1
            for a, b, s in zip(rw, hw, sw):
                key = re.sub(r"[^a-z']", "", s.lower().replace("’", "'"))
                if lang == "hi":
                    cls = "lexicon"
                elif key in FUNCTION_WORDS:
                    cls = "function"
                elif key in cmu:
                    cls = "lexicon"
                else:
                    cls = "oov"
                word_class[cls]["words"] += 1
                word_class[cls]["wrong"] += a != b
        else:
            tot["word_count_mismatch"] += 1
        if d:
            worst.append((d / max(1, len(r)), sid, text, ref, h))
    worst.sort(reverse=True)
    res = {
        "sentences": tot["sent"],
        "reference_symbols": tot["ref"],
        "per": tot["err"] / tot["ref"],
        "per_without_stress": tot["err_nostress"] / tot["ref_nostress"],
        "exact_sentence": tot["exact"] / tot["sent"],
        "exact_sentence_without_stress": tot["exact_nostress"] / tot["sent"],
        "piper_shape_per": tot["piper_err"] / tot["piper_ref"],
        "piper_shape_exact_sentence": tot["piper_exact"] / tot["sent"],
        "word_count_mismatch_sentences": tot["word_count_mismatch"],
        "errors_by_class": {k: {"count": v, "share": v / max(1, tot["err"]), "per_contribution": v / tot["ref"]} for k, v in ops_by_class.most_common()},
        "top_confusions": [{"edit": k, "count": v} for k, v in confusions.most_common(25)],
        "word_error_by_class": {k: {"words": v["words"], "wrong": v["wrong"], "rate": v["wrong"] / v["words"]} for k, v in word_class.items() if v["words"]},
        "by_subset": {k: {"sentences": v["sent"], "per": v["err"] / v["ref"], "exact_sentence": v["exact"] / v["sent"]} for k, v in subset.items() if v["sent"]},
        "examples_of_disagreement": [{"id": sid, "text": text, "reference": ref, "uccharan": h} for _, sid, text, ref, h in worst[len(worst) // 3 :][:examples]],
    }
    return res, worst


def evaluate_words(words, lang, oracle, binary):
    voice = VOICES[lang]
    hyp = run_uccharan(words, lang, binary=binary)
    tot = Counter()
    wrong = []
    for w, h in zip(words, hyp):
        ref = " ".join(re.sub(r"\([a-z-]+\)", "", oracle.phonemize(w, voice)).split())
        r, hh = nfd(ref), nfd(h)
        d, _ = align(r, hh)
        tot["ref"] += len(r)
        tot["err"] += d
        tot["n"] += 1
        tot["exact"] += d == 0
        tot["exact_nostress"] += strip_stress(ref) == strip_stress(h)
        tot["stress_wrong"] += stress_pattern(ref) != stress_pattern(h)
        if d:
            wrong.append((w, ref, h))
    return {
        "words": tot["n"],
        "per": tot["err"] / max(1, tot["ref"]),
        "exact_word": tot["exact"] / max(1, tot["n"]),
        "exact_word_without_stress": tot["exact_nostress"] / max(1, tot["n"]),
        "stress_pattern_error": tot["stress_wrong"] / max(1, tot["n"]),
    }, wrong


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--lang", choices=sorted(VOICES), required=True)
    ap.add_argument("--set", default="heldout", help="heldout | dev | prose (English only)")
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--words", action="store_true", help="also score the word lists")
    ap.add_argument("--out", type=Path, help="directory for the JSON result")
    ap.add_argument("--show", type=int, default=0, help="print this many disagreeing sentences (development only)")
    ap.add_argument("--show-words", type=int, default=0)
    ap.add_argument("--uccharan-bin")
    args = ap.parse_args()
    oracle = load_oracle()
    if oracle is None:
        print("reference phonemizer not installed; nothing to compare against")
        return 2
    base = "hi" if args.lang == "hi" else "en"
    rows = read_tsv(DATA / f"{base}_{args.set}.tsv")
    if args.limit:
        rows = rows[: args.limit]
    res, worst = evaluate_sentences(rows, args.lang, oracle, args.uccharan_bin)
    result = {"language": args.lang, "reference": f"espeak-ng {oracle.version()} voice {VOICES[args.lang]} (library API)", "set": args.set, "sentence_level": res}
    print(f"[{args.lang} {args.set}] n={res['sentences']} PER {100 * res['per']:.2f}%  (no stress {100 * res['per_without_stress']:.2f}%)  "
          f"exact {100 * res['exact_sentence']:.1f}%  piper-shape PER {100 * res['piper_shape_per']:.2f}% exact {100 * res['piper_shape_exact_sentence']:.1f}%")
    for k, v in res["errors_by_class"].items():
        print(f"   {k:14s} {v['count']:6d}  {100 * v['share']:5.1f}% of errors")
    print("   word error by class:", {k: f"{100 * v['rate']:.2f}% of {v['words']}" for k, v in res["word_error_by_class"].items()})
    print("   by subset:", {k: f"PER {100 * v['per']:.2f}% n={v['sentences']}" for k, v in res["by_subset"].items()})
    print("   top confusions:", " ".join(f"{c['edit']}×{c['count']}" for c in res["top_confusions"][:18]))
    for _, sid, text, ref, h in worst[: args.show]:
        print(f"   {sid}\t{text}\n      ref {ref}\n      got {h}")
    if args.words:
        lists = {"hi": {"heldout-sentence-words": "hi_words_heldout.txt"}}.get(
            args.lang, {"frequent-in-lexicon": "en_words_frequent.txt", "rare-in-lexicon": "en_words_rare.txt", "out-of-lexicon": "en_words_oov.txt"}
        )
        result["word_level"] = {}
        for name, fn in lists.items():
            words = [w for w in (DATA / fn).read_text(encoding="utf-8").split("\n") if w]
            if args.limit:
                words = words[: args.limit]
            wres, wrong = evaluate_words(words, args.lang, oracle, args.uccharan_bin)
            result["word_level"][name] = wres
            print(f"   words {name}: n={wres['words']} PER {100 * wres['per']:.2f}% exact {100 * wres['exact_word']:.1f}% "
                  f"(no stress {100 * wres['exact_word_without_stress']:.1f}%) stress-pattern errors {100 * wres['stress_pattern_error']:.1f}%")
            for w, ref, h in wrong[: args.show_words]:
                print(f"      {w}\t{ref}\t{h}")
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        path = args.out / f"agreement_{args.lang}_{args.set}.json"
        path.write_text(json.dumps(result, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
        print("wrote", path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
