"""Label a small Hindi and Bhojpuri dataset and pick a reading script.

    python examples/train/run.py [OUTPUT_DIR]

No audio is needed: the manifest only names the files. The same steps with
the command line are in README.md next to this file.
"""

from __future__ import annotations

import sys
from pathlib import Path

from uccharan.train import coverage, label, read_manifest, read_texts, select_script

HERE = Path(__file__).resolve().parent


def main(out_dir: str = "out") -> None:
    out = Path(out_dir)

    # 1. Labelling: every row is read in its own language, improved mode.
    rows = read_manifest(HERE / "metadata.jsonl")
    done = label(rows, "hi", improved=True)
    for path in done.write(out / "labels", validation=2):
        print("wrote", path)
    for u in done.utterances:
        for w in u.warnings:
            print(f"warning: {u.id}: {w}")
    print(f"{len(done.utterances)} utterances, {len(done.symbols)} symbols")

    # 2. A reading script per language (the corpus is tiny, so a phone seen
    #    in a single sentence is not treated as rare).
    for lang in ("hi", "bho"):
        script = select_script(read_texts(HERE / f"{lang}.txt"), lang, improved=True, min_symbol_sentences=1, max_sentences=5)
        script.write(out / f"script-{lang}")
        ph, di = script.stats["phones"], script.stats["diphones"]
        print(f"{lang}: {len(script.sentences)} sentences cover {ph['covered']}/{ph['total']} phones and {di['covered']}/{di['total']} diphones")
        for s in script.sentences:
            print("   ", s.text)

    # 3. What the labelled dataset covers.
    print(coverage(done, "hi").summary(), end="")


if __name__ == "__main__":
    main(*sys.argv[1:2])
