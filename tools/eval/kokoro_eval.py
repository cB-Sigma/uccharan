#!/usr/bin/env python3
"""Agreement and speed of uccharan's Kokoro English front end against misaki,
the G2P Kokoro's English voices were trained on.

Run it with a Python that has misaki installed (`pip install misaki` plus
spaCy's `en_core_web_sm`, `phonemizer-fork` and `espeakng-loader` for misaki's
own out-of-dictionary fallback). misaki is the reference here; nothing of its
output is stored except aggregate numbers and a few examples.

    python tools/eval/kokoro_eval.py --set heldout --out tools/eval/results

Reported per accent:

* sentence exact match and token error rate (edit distance over Kokoro tokens
  / reference length);
* word-level agreement, split into words misaki finds in its dictionaries
  (gold, silver, or their suffix rules), words whose dictionary entry depends on
  the part of speech, and words misaki sends to its espeak-ng fallback;
* time per sentence for both (rough: the uccharan side is timed through its
  command-line program; tools/bench/kokoro_speed.py measures properly).

uccharan uses its built-in copy of misaki 0.9.4's dictionaries unless
`--use-installed-dictionaries` is given.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import time
import unicodedata
from collections import Counter
from pathlib import Path

from common import align, find_binary, read_tsv

DATA = Path(__file__).resolve().parent / "data"


def misaki_g2p(british: bool, fallback: bool):
    from misaki import en  # type: ignore[import-not-found]

    fb = None
    if fallback:
        from misaki import espeak  # type: ignore[import-not-found]

        fb = espeak.EspeakFallback(british=british)
    return en.G2P(trf=False, british=british, fallback=fb, unk="❓" if not fallback else "")


def ours(texts: list[str], lang: str, misaki_dir: Path | None, binary: str | None) -> tuple[list[str], float]:
    args = [find_binary(binary), "-l", lang, "-p", "kokoro"] + (["--misaki", str(misaki_dir)] if misaki_dir else [])
    payload = "\n".join(t.replace("\n", " ") for t in texts) + "\n"
    t0 = time.perf_counter()
    out = subprocess.run(args, input=payload, capture_output=True, text=True, check=True).stdout.split("\n")[: len(texts)]
    total = time.perf_counter() - t0
    t0 = time.perf_counter()
    subprocess.run(args, input=texts[0] + "\n", capture_output=True, text=True, check=True)
    startup = time.perf_counter() - t0
    return out, max(0.0, total - startup)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--set", default="heldout", help="heldout | dev | prose")
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--accents", default="en-us,en-gb")
    ap.add_argument("--out", type=Path)
    ap.add_argument("--show", type=int, default=0)
    ap.add_argument("--uccharan-bin")
    ap.add_argument("--use-installed-dictionaries", action="store_true", help="make uccharan read the installed misaki's JSON instead of its built-in copy")
    ap.add_argument("--dump", type=Path, help="write misaki's output for the set (JSON lines) for kokoro_e2e.py; keep it out of the repository")
    args = ap.parse_args()
    import misaki  # type: ignore[import-not-found]

    misaki_dir = Path(misaki.__file__).parent / "data"
    version = getattr(misaki, "__version__", None)
    if version is None:
        from importlib.metadata import version as v

        version = v("misaki")
    rows = read_tsv(DATA / f"en_{args.set}.tsv")
    if args.limit:
        rows = rows[: args.limit]
    texts = [t for _, t in rows]
    result: dict = {"reference": f"misaki {version} (spaCy en_core_web_sm tagger, espeak-ng fallback)", "set": args.set, "sentences": len(texts), "accents": {}}
    for lang in args.accents.split(","):
        british = lang == "en-gb"
        g_ref = misaki_g2p(british, True)
        g_cls = misaki_g2p(british, False)
        lex = g_cls.lexicon
        tagged = {k.lower() for k, v in lex.golds.items() if isinstance(v, dict)}
        for t in texts[:20]:
            g_ref(t)
        t0 = time.perf_counter()
        ref = [g_ref(t)[0] for t in texts]
        t_ref = time.perf_counter() - t0
        if args.dump:
            with open(f"{args.dump}.{lang}.jsonl", "w", encoding="utf-8") as fh:
                for (sid, text), r in zip(rows, ref):
                    fh.write(json.dumps({"id": sid, "text": text, "misaki": r}, ensure_ascii=False) + "\n")
        cls = [g_cls(t)[0] for t in texts]
        hyp, t_ours = ours(texts, lang, misaki_dir if args.use_installed_dictionaries else None, args.uccharan_bin)
        tot = Counter()
        cats = {k: Counter() for k in ("in-dictionary", "pos-dependent", "out-of-dictionary")}
        worst = []
        conf = Counter()
        for text, r, c, h in zip(texts, ref, cls, hyp):
            r = unicodedata.normalize("NFD", r.strip())
            h = unicodedata.normalize("NFD", h.strip())
            d, ops = align(list(r), list(h))
            tot["n"] += 1
            tot["ref"] += len(r)
            tot["err"] += d
            tot["exact"] += d == 0
            rw, hw, cw = r.split(), h.split(), c.split()
            src = text.split()
            if len(rw) == len(hw) == len(cw):
                tot["aligned"] += 1
                for k, (a, b, x) in enumerate(zip(rw, hw, cw)):
                    word = src[k].strip(".,;:!?\"“”()").lower() if len(src) == len(rw) else ""
                    if "❓" in x:
                        cat = "out-of-dictionary"
                    elif word in tagged:
                        cat = "pos-dependent"
                    else:
                        cat = "in-dictionary"
                    cats[cat]["words"] += 1
                    cats[cat]["same"] += a == b
                    if a != b and cat != "out-of-dictionary":
                        conf[(word or x, a, b)] += 1
            if d:
                worst.append((d, text, r, h))
        entry = {
            "sentences": tot["n"],
            "exact_sentence": tot["exact"] / tot["n"],
            "token_error_rate": tot["err"] / tot["ref"],
            "sentences_with_aligned_words": tot["aligned"],
            "word_agreement": {k: {"words": v["words"], "identical": v["same"] / max(1, v["words"])} for k, v in cats.items()},
            "seconds_per_sentence": {"misaki": t_ref / len(texts), "uccharan": t_ours / len(texts)},
            "speedup": (t_ref / t_ours) if t_ours else None,
            "top_in_dictionary_disagreements": [{"word": w, "misaki": a, "uccharan": b, "count": n} for (w, a, b), n in conf.most_common(15)],
        }
        result["accents"][lang] = entry
        wa = entry["word_agreement"]
        print(f"[{lang} {args.set}] n={tot['n']} exact {100 * entry['exact_sentence']:.1f}%  token error {100 * entry['token_error_rate']:.2f}%  "
              f"words: in-dict {100 * wa['in-dictionary']['identical']:.2f}% of {wa['in-dictionary']['words']}, "
              f"POS-dependent {100 * wa['pos-dependent']['identical']:.1f}% of {wa['pos-dependent']['words']}, "
              f"OOV {100 * wa['out-of-dictionary']['identical']:.1f}% of {wa['out-of-dictionary']['words']}  "
              f"time/sentence misaki {1e3 * t_ref / len(texts):.2f} ms, uccharan {1e6 * t_ours / len(texts):.1f} µs")
        for (w, a, b), n in conf.most_common(args.show):
            print(f"   {n:3d} {w}\tmisaki {a}\tuccharan {b}")
        worst.sort(reverse=True)
        for d, text, r, h in worst[: args.show // 3]:
            print(f"   {text}\n      misaki   {r}\n      uccharan {h}")
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / f"kokoro_{args.set}.json").write_text(json.dumps(result, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
