#!/usr/bin/env python3
"""Size on disk of uccharan's language packs and code next to espeak-ng's,
raw and with the same general-purpose compressors applied to both sides.

    cargo build --release -p uccharan-capi --no-default-features --target-dir target/nodata
    python tools/bench/sizes.py --code target/nodata/release/libuccharan.so \\
        --espeak-lib /path/to/libespeak-ng.so.1.52.0 --espeak-data /path/to/espeak-ng-data \\
        --out tools/bench/results/sizes.json

What counts as "the files a language needs":

* uccharan: the language pack (`.ucd`). Packs are self-contained.
* espeak-ng: `<lang>_dict` plus the phoneme files every language loads at
  start-up (`phontab`, `phonindex`, `phondata`, `intonations`). espeak-ng
  does not start without them, even when it only produces phonemes. Their
  sizes are also reported on their own, so the dictionary alone can be
  compared too.

Code is the shared library without embedded data (uccharan built with
`--no-default-features`) and espeak-ng's shared library. Compression: gzip -9,
xz -9e and zstd -19 (zstd only when the `zstandard` module is installed).
"""

from __future__ import annotations

import argparse
import gzip
import json
import lzma
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SHARED = ["phontab", "phonindex", "phondata", "intonations"]


def sizes(blob: bytes) -> dict:
    out = {
        "raw": len(blob),
        "gzip": len(gzip.compress(blob, 9, mtime=0)),
        "xz": len(lzma.compress(blob, preset=9 | lzma.PRESET_EXTREME)),
    }
    try:
        import zstandard  # type: ignore[import-not-found]

        out["zstd"] = len(zstandard.ZstdCompressor(level=19).compress(blob))
    except ImportError:
        pass
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--data", type=Path, default=ROOT / "crates/uccharan/data")
    ap.add_argument("--code", type=Path, help="uccharan shared library built without embedded data")
    ap.add_argument("--espeak-lib", type=Path)
    ap.add_argument("--espeak-data", type=Path)
    ap.add_argument("--out", type=Path)
    args = ap.parse_args()
    res: dict = {"uccharan": {}, "espeak-ng": {}}
    tables = ROOT / "crates/uccharan-data/data"
    for f in sorted([*args.data.glob("*.ucd"), *(tables.glob("*.ucd") if args.data == ROOT / "crates/uccharan/data" else [])]):
        res["uccharan"][f.stem] = sizes(f.read_bytes())
    if args.code:
        res["uccharan"]["code"] = sizes(args.code.read_bytes())
    if args.espeak_lib:
        res["espeak-ng"]["code"] = sizes(args.espeak_lib.read_bytes())
    if args.espeak_data:
        d = args.espeak_data
        shared = b"".join((d / f).read_bytes() for f in SHARED)
        res["espeak-ng"]["shared phoneme files"] = sizes(shared)
        for lang, dic in (("en", "en_dict"), ("hi", "hi_dict")):
            res["espeak-ng"][dic] = sizes((d / dic).read_bytes())
            # Each language as one concatenated stream, as a pack would be.
            res["espeak-ng"][lang] = sizes((d / dic).read_bytes() + shared)
    cols = ["raw", "gzip", "xz", "zstd"]
    print(f"{'':28s}" + "".join(f"{c:>10s}" for c in cols))
    for side, rows in res.items():
        for k, v in rows.items():
            print(f"{side + ' ' + k:28s}" + "".join(f"{v.get(c, 0):10d}" for c in cols))
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(res, indent=1) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
