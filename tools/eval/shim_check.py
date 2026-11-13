#!/usr/bin/env python3
"""Compare uccharan.compat.EspeakBackend with phonemizer's EspeakBackend
(both installed by you; phonemizer and espeak-ng are GPL and not dependencies
of uccharan). Prints the share of utterances with identical output.

    python tools/eval/shim_check.py --set heldout --limit 1000
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from common import read_tsv

DATA = Path(__file__).resolve().parent / "data"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--set", default="heldout")
    ap.add_argument("--limit", type=int, default=1000)
    ap.add_argument("--out", type=Path)
    args = ap.parse_args()
    from uccharan.compat import EspeakBackend as Ours

    try:
        import espeakng_loader  # type: ignore[import-not-found]
        from phonemizer.backend.espeak.wrapper import EspeakWrapper  # type: ignore[import-not-found]

        EspeakWrapper.set_library(espeakng_loader.get_library_path())
        EspeakWrapper.set_data_path(espeakng_loader.get_data_path())
    except ImportError:
        pass
    from phonemizer.backend import EspeakBackend as Theirs  # type: ignore[import-not-found]

    res = {}
    for lang in ("en-us", "en-gb-x-rp", "hi"):
        rows = read_tsv(DATA / f"{'hi' if lang == 'hi' else 'en'}_{args.set}.tsv")[: args.limit]
        texts = [t for _, t in rows]
        for name, kw in (("stress", {"with_stress": True}), ("stress+punctuation", {"with_stress": True, "preserve_punctuation": True}),
                         ("stress+tie", {"with_stress": True, "tie": True}), ("plain", {})):
            a = Theirs(lang, **kw).phonemize(texts, strip=True)
            b = Ours(lang, **kw).phonemize(texts, strip=True)
            share = sum(x.strip() == y.strip() for x, y in zip(a, b)) / len(texts)
            res[f"{lang} {name}"] = share
            print(f"{lang:11s} {name:20s} identical {100 * share:5.1f}%")
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / "shim.json").write_text(json.dumps({"set": args.set, "utterances": args.limit, "identical": res}, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
