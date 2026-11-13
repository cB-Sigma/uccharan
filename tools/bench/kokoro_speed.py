#!/usr/bin/env python3
"""Speed of uccharan's Kokoro English front end next to misaki, measured the
same way: one call per sentence from one Python process, same sentences,
same output (Kokoro token strings). uccharan is called through its C library
with ctypes (a few µs of call overhead included).

Run with a Python that has misaki and spaCy's en_core_web_sm installed:

    cargo build --release -p uccharan-capi
    python tools/bench/kokoro_speed.py --out tools/bench/results/kokoro_speed.json
"""

from __future__ import annotations

import argparse
import ctypes
import json
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def sentences(limit: int, part: str = "heldout") -> list[str]:
    rows = (ROOT / f"tools/eval/data/en_{part}.tsv").read_text(encoding="utf-8").splitlines()
    return [r.split("\t", 1)[1] for r in rows if "\t" in r][:limit]


class Ours:
    def __init__(self, lib: str, data: Path | None, accent: str) -> None:
        self.lib = ctypes.CDLL(lib)
        self.lib.uccharan_new.restype = ctypes.c_void_p
        self.lib.uccharan_phonemize.restype = ctypes.c_void_p
        self.lib.uccharan_phonemize.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint32]
        self.lib.uccharan_load_misaki.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p]
        self.lib.uccharan_string_free.argtypes = [ctypes.c_void_p]
        self.h = self.lib.uccharan_new()
        self.lang = b"en-gb" if accent == "gb" else b"en-us"
        if data is not None:
            gold = (data / f"{accent}_gold.json").read_bytes()
            silver = (data / f"{accent}_silver.json").read_bytes()
            assert self.lib.uccharan_load_misaki(self.h, self.lang, gold, silver) == 0

    def __call__(self, text: str) -> str:
        p = self.lib.uccharan_phonemize(self.h, text.encode(), self.lang, b"kokoro", 0)
        s = ctypes.string_at(p).decode()
        self.lib.uccharan_string_free(p)
        return s


def make(which: str, lib: str, accent: str):
    if which == "uccharan":
        return Ours(lib, None, accent)  # built-in copy of misaki's dictionaries
    from misaki import en  # type: ignore[import-not-found]

    g = en.G2P(trf=False, british=accent == "gb", fallback=None, unk="")
    return lambda t: g(t)[0]


def child(which: str, lib: str, accent: str) -> None:
    def rss() -> int:
        for line in open("/proc/self/status"):
            if line.startswith("VmRSS:"):
                return int(line.split()[1]) * 1024
        return 0

    r0 = rss()
    t0 = time.perf_counter()
    f = make(which, lib, accent)
    f("Hello there, this is the first sentence.")
    print(json.dumps({"init_ms": 1e3 * (time.perf_counter() - t0), "rss_mb": (rss() - r0) / 2**20}))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--lib", default=str(ROOT / "target/release/libuccharan.so"))
    ap.add_argument("--limit", type=int, default=3000)
    ap.add_argument("--out", type=Path)
    ap.add_argument("--child")
    ap.add_argument("--accent", default="us")
    args = ap.parse_args()
    if args.child:
        child(args.child, args.lib, args.accent)
        return 0
    texts = sentences(args.limit)
    words = sum(len(t.split()) for t in texts)
    res: dict = {"sentences": len(texts), "words": words, "systems": {}}
    for which in ("misaki", "uccharan"):
        f = make(which, args.lib, "us")
        # Warm up on other text (uccharan keeps a small cache of recent words).
        for t in sentences(300, "dev"):
            f(t)
        lat = []
        for t in texts:
            a = time.perf_counter()
            f(t)
            lat.append(time.perf_counter() - a)
        lat.sort()
        init = [json.loads(subprocess.run([sys.executable, __file__, "--child", which, "--lib", args.lib], capture_output=True, text=True).stdout) for _ in range(3)]
        e = {
            "latency_us_p50": 1e6 * lat[len(lat) // 2],
            "latency_us_p99": 1e6 * lat[int(0.99 * len(lat))],
            "words_per_second": words / sum(lat),
            "init_plus_first_call_ms": statistics.median(i["init_ms"] for i in init),
            "rss_after_init_mb": statistics.median(i["rss_mb"] for i in init),
        }
        res["systems"][which] = e
        print(f"{which:9s} p50 {e['latency_us_p50']:8.1f} µs  p99 {e['latency_us_p99']:8.1f} µs  {e['words_per_second']:9.0f} words/s  "
              f"init {e['init_plus_first_call_ms']:7.1f} ms  rss {e['rss_after_init_mb']:6.1f} MB")
    m, u = res["systems"]["misaki"], res["systems"]["uccharan"]
    res["speedup"] = {"latency_p50": m["latency_us_p50"] / u["latency_us_p50"], "throughput": u["words_per_second"] / m["words_per_second"],
                      "init": m["init_plus_first_call_ms"] / u["init_plus_first_call_ms"]}
    print("speed-up", {k: round(v, 1) for k, v in res["speedup"].items()})
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(res, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
