#!/usr/bin/env python3
"""Write the SHA-256 sums and sizes of the language packs
(crates/uccharan/data/*.ucd, crates/uccharan-data/data/*.ucd and the optional data/packs/*.ucd)
into the places that check them: crates/uccharan/src/packs_list.rs,
bindings/python/python/uccharan/packs.py and bindings/wasm/packs.js.

    python tools/packs.py          # update
    python tools/packs.py --check  # exit 1 when they are out of date (CI)
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "crates/uccharan/data"
PY = ROOT / "bindings/python/python/uccharan/packs.py"
JS = ROOT / "bindings/wasm/packs.js"
RS = ROOT / "crates/uccharan/src/packs_list.rs"


def packs() -> dict[str, tuple[str, int]]:
    out = {}
    # Built-in data and word tables, then the optional share-alike packs in data/packs.
    core = sorted([*DATA.glob("*.ucd"), *(ROOT / "crates/uccharan-data/data").glob("*.ucd")], key=lambda f: f.stem)
    for f in core + sorted((ROOT / "data/packs").glob("*.ucd")):
        b = f.read_bytes()
        out[f.stem] = (hashlib.sha256(b).hexdigest(), len(b))
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    p = packs()
    body = "".join(f'    "{k}": ("{h}", {n}),\n' for k, (h, n) in p.items())
    py_old = PY.read_text(encoding="utf-8")
    py_new = re.sub(r"(    # @@PACKS@@\n).*?(    # @@END@@)", lambda m: m.group(1) + body + m.group(2), py_old, flags=re.S)
    js_new = (
        "// SHA-256 sums and sizes of the language packs; written by tools/packs.py.\n"
        "export const PACKS = Object.freeze("
        + json.dumps({k: {"sha256": h, "size": n} for k, (h, n) in p.items()}, indent=1)
        + ");\n"
    )
    js_old = JS.read_text(encoding="utf-8") if JS.exists() else ""
    rs_new = (
        "// SHA-256 sums and sizes of the language packs; written by tools/packs.py.\n&[\n"
        + "".join(f'    Pack {{\n        name: "{k}",\n        sha256: "{h}",\n        size: {n},\n    }},\n' for k, (h, n) in p.items())
        + "]\n"
    )
    rs_old = RS.read_text(encoding="utf-8") if RS.exists() else ""
    if args.check:
        stale = [str(f.relative_to(ROOT)) for f, a, b in ((PY, py_old, py_new), (JS, js_old, js_new), (RS, rs_old, rs_new)) if a != b]
        if stale:
            print("pack sums out of date (run python tools/packs.py):", ", ".join(stale), file=sys.stderr)
            return 1
        return 0
    PY.write_text(py_new, encoding="utf-8")
    JS.write_text(js_new, encoding="utf-8")
    RS.write_text(rs_new, encoding="utf-8")
    for k, (h, n) in p.items():
        print(f"{k:14s} {n:9d}  {h}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
