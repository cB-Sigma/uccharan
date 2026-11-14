#!/usr/bin/env python3
"""Speed, start-up and memory of uccharan next to a reference phonemizer,
measured the same way: both are shared libraries called through ctypes from
one Python process, on the same sentences, producing the same kind of output
(an IPA string per sentence).

    cargo build --release -p uccharan-capi
    python tools/bench/compare.py --out tools/bench/results/compare.json

The reference side is optional: it needs an espeak-ng shared library that you
installed yourself (for example `pip install espeakng-loader`). Without it
only the uccharan columns are filled in.

What is measured, per language:

* single-sentence latency: wall time of one call per sentence (p50, p99, mean);
* throughput, one thread: words of input text per second over the whole set;
* throughput, N workers: for uccharan one call of the batch function
  (`uccharan_phonemize_lines`) on N native threads that share one handle.
  The reference library keeps global state and cannot be called from several
  threads, so it is run in N separate *processes* instead, which is the most
  favourable way to parallelise it;
* cold start: a fresh process that loads the library, initialises it and
  phonemizes one sentence (median of several runs), minus the same process
  doing nothing;
* memory: resident set size added by loading + initialising the library and
  by processing the whole set, in a fresh process;
* size on disk of the library and its data.
"""

from __future__ import annotations

import argparse
import ctypes
import json
import os
import platform
import statistics
import subprocess
import sys
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VOICES = {"bho": "hi", "mai": "hi", "en-us": "en-us", "en-gb": "en-gb-x-rp", "hi": "hi", "bn": "bn", "mr": "mr", "te": "te", "ne": "ne", "kn": "kn", "ta": "ta", "ml": "ml", "as": "as", "gu": "gu", "pa": "pa", "or": "or", "kok": "kok", "si": "si", "sa": "hi"}
FLAGS = 4 | 16  # reference-style number reading, no punctuation: same output shape as the reference call


def sentences(lang: str, limit: int, part: str = "heldout") -> list[str]:
    name = f"{'en' if lang.startswith('en') else lang}_{part}.tsv"
    rows = [line.split("\t", 1)[1] for line in (ROOT / "tools/eval/data" / name).read_text(encoding="utf-8").splitlines() if "\t" in line]
    return rows[:limit] if limit else rows


class Uccharan:
    name = "uccharan"

    def __init__(self, lib_path: str) -> None:
        self.path = lib_path
        lib = ctypes.CDLL(lib_path)
        lib.uccharan_new.restype = ctypes.c_void_p
        lib.uccharan_phonemize.restype = ctypes.c_void_p
        lib.uccharan_phonemize.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint32]
        lib.uccharan_string_free.argtypes = [ctypes.c_void_p]
        self.lib = lib
        self.h = lib.uccharan_new()

    def set_lang(self, lang: str) -> None:
        self.lang = lang.encode()

    def call(self, text: bytes) -> bytes:
        p = self.lib.uccharan_phonemize(self.h, text, self.lang, b"espeak-ipa", FLAGS)
        s = ctypes.string_at(p)
        self.lib.uccharan_string_free(p)
        return s

    def disk(self) -> dict:
        return {"library_bytes": os.path.getsize(self.path), "data_bytes": 0, "note": "language data for en-US, en-GB and Hindi is inside the library"}


class Reference:
    name = "reference"

    def __init__(self) -> None:
        import espeakng_loader  # type: ignore[import-not-found]

        self.path = espeakng_loader.get_library_path()
        self.data = espeakng_loader.get_data_path()
        lib = ctypes.CDLL(self.path)
        lib.espeak_Initialize.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_char_p, ctypes.c_int]
        lib.espeak_SetVoiceByName.argtypes = [ctypes.c_char_p]
        lib.espeak_TextToPhonemes.argtypes = [ctypes.POINTER(ctypes.c_char_p), ctypes.c_int, ctypes.c_int]
        lib.espeak_TextToPhonemes.restype = ctypes.c_char_p
        lib.espeak_Info.restype = ctypes.c_char_p
        lib.espeak_Info.argtypes = [ctypes.c_void_p]
        if lib.espeak_Initialize(1, 0, self.data.encode(), 0x8000) <= 0:
            raise OSError("initialisation failed")
        self.lib = lib
        self.version = lib.espeak_Info(None).decode().split()[0]

    def set_lang(self, lang: str) -> None:
        self.lib.espeak_SetVoiceByName(VOICES[lang].encode())

    def call(self, text: bytes) -> bytes:
        buf = ctypes.c_char_p(text)
        out = []
        f = self.lib.espeak_TextToPhonemes
        while buf.value:
            r = f(ctypes.byref(buf), 1, 2)
            if r:
                out.append(r)
        return b" ".join(out)

    def disk(self) -> dict:
        data = sum(p.stat().st_size for p in Path(self.data).rglob("*") if p.is_file())
        return {"library_bytes": os.path.getsize(self.path), "data_bytes": data, "note": "all languages; the data directory is required at run time"}


def make(which: str, lib_path: str):
    return Uccharan(lib_path) if which == "uccharan" else Reference()


def pct(xs: list[float], q: float) -> float:
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(q * len(xs)))]


def bench_latency(engines: list[str], lib_path: str, lang: str, limit: int, rounds: int) -> dict:
    """Each round runs in a fresh process: initialise, warm up on 1 000
    development sentences (other text), then time one call per held-out
    sentence once. uccharan keeps a small per-thread cache of recent words, so
    timing the same sentences repeatedly in one process would flatter it.
    The engines take turns round by round, so that a change in machine load
    affects both alike; the best round of each is kept."""
    best: dict = {}
    for _ in range(rounds):
        for which in engines:
            r = run_child(which, lib_path, lang, "latency", limit)
            if which not in best or sum(r["lat"]) < sum(best[which]["lat"]):
                best[which] = r
    out = {}
    for which, b in best.items():
        lat, words = b["lat"], b["words"]
        out[which] = {
            "sentences": len(lat),
            "words": words,
            "latency_us_p50": 1e6 * pct(lat, 0.50),
            "latency_us_p99": 1e6 * pct(lat, 0.99),
            "latency_us_mean": 1e6 * statistics.fmean(lat),
            "words_per_second_1_thread": words / sum(lat),
            "sentences_per_second_1_thread": len(lat) / sum(lat),
            "method": "best of N fresh processes, engines alternating; warm-up on 1 000 development sentences; each held-out sentence timed once",
        }
    return out


def bench_threads(eng, lang: str, texts: list[bytes], n: int, repeat: int) -> float:
    """Words per second for one batch call on n native threads that share the
    handle (`uccharan_phonemize_lines`)."""
    work = texts * repeat
    blob = b"\n".join(t.replace(b"\n", b" ") for t in work)
    f = eng.lib.uccharan_phonemize_lines
    f.restype = ctypes.c_void_p
    f.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint32, ctypes.c_uint32]
    t0 = time.perf_counter()
    p = f(eng.h, blob, lang.encode(), b"espeak-ipa", FLAGS, n)
    dt = time.perf_counter() - t0
    eng.lib.uccharan_string_free(p)
    return sum(len(t.split()) for t in work) / dt


def child(which: str, lib_path: str, lang: str, mode: str, limit: int, repeat: int) -> None:
    """Runs in a fresh process; prints one JSON object."""

    def rss() -> int:
        for line in open("/proc/self/status"):
            if line.startswith("VmRSS:"):
                return int(line.split()[1]) * 1024
        return 0

    if mode == "noop":
        print(json.dumps({"rss": rss()}))
        return
    if mode == "latency":
        eng = make(which, lib_path)
        eng.set_lang(lang)
        for t in sentences(lang, 1000, "dev"):
            eng.call(t.encode())
        texts = [s.encode() for s in sentences(lang, limit)]
        clock = time.perf_counter
        lat = []
        for t in texts:
            t0 = clock()
            eng.call(t)
            lat.append(clock() - t0)
        print(json.dumps({"lat": lat, "words": sum(len(t.split()) for t in texts)}))
        return
    r0 = rss()
    t0 = time.perf_counter()
    eng = make(which, lib_path)
    eng.set_lang(lang)
    first = eng.call("Hello there, this is the first sentence.".encode() if lang != "hi" else "नमस्ते, यह पहला वाक्य है।".encode())
    t1 = time.perf_counter()
    r1 = rss()
    out = {"init_plus_first_call_s": t1 - t0, "rss_after_init": r1 - r0, "first": first.decode()}
    if mode == "full":
        texts = [s.encode() for s in sentences(lang, limit)] * repeat
        t2 = time.perf_counter()
        for t in texts:
            eng.call(t)
        out["seconds"] = time.perf_counter() - t2
        out["words"] = sum(len(t.split()) for t in texts)
        out["rss_after_run"] = rss() - r0
    print(json.dumps(out))


def spawn(which: str, lib_path: str, lang: str, mode: str, limit: int = 0, repeat: int = 1) -> subprocess.Popen:
    return subprocess.Popen([sys.executable, __file__, "--child", which, "--lib", lib_path, "--lang", lang, "--mode", mode, "--limit", str(limit), "--repeat", str(repeat)],
                            stdout=subprocess.PIPE, text=True)


def run_child(*a, **k) -> dict:
    p = spawn(*a, **k)
    out = p.communicate()[0]
    return json.loads(out)


def process_time(mode: str, which: str, lib_path: str, lang: str, runs: int) -> float:
    ts = []
    for _ in range(runs):
        t0 = time.perf_counter()
        spawn(which, lib_path, lang, mode).communicate()
        ts.append(time.perf_counter() - t0)
    return statistics.median(ts)


def machine_scaling(workers: list[int]) -> dict:
    """Control measurement: how much total work N busy processes get done on
    this machine right now, relative to one. On an idle N-core machine this
    is N; on shared or throttled hardware it is less, and no library can
    scale better than this."""
    code = "import time\nt=time.perf_counter();n=0\nwhile time.perf_counter()-t<1.5:\n    for _ in range(20000): n+=1\nprint(n)"
    out = {}
    base = None
    for n in workers:
        ps = [subprocess.Popen([sys.executable, "-c", code], stdout=subprocess.PIPE, text=True) for _ in range(n)]
        total = sum(int(p.communicate()[0]) for p in ps)
        base = base or total
        out[str(n)] = total / base
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--lib", default=str(ROOT / "target/release/libuccharan.so"))
    ap.add_argument("--langs", default="en-us,hi")
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--rounds", type=int, default=5)
    ap.add_argument("--threads", default="1,2,4")
    ap.add_argument("--out", type=Path)
    ap.add_argument("--no-reference", action="store_true")
    ap.add_argument("--child")
    ap.add_argument("--lang")
    ap.add_argument("--mode")
    ap.add_argument("--repeat", type=int, default=1)
    args = ap.parse_args()
    if args.child:
        child(args.child, args.lib, args.lang, args.mode, args.limit, args.repeat)
        return 0

    engines = ["uccharan"]
    ref_version = None
    if not args.no_reference:
        try:
            ref_version = Reference().version
            engines.append("reference")
        except Exception as exc:
            print(f"reference phonemizer not available ({exc}); measuring uccharan only")
    result: dict = {
        "machine": {"cpu": platform.processor() or platform.machine(), "logical_cpus": os.cpu_count(), "python": platform.python_version(), "system": platform.platform()},
        "reference": f"espeak-ng {ref_version} shared library via ctypes" if ref_version else None,
        "languages": {},
    }
    threads = [int(x) for x in args.threads.split(",")]
    result["machine_scaling_control"] = machine_scaling(threads)
    print("machine scaling control (total work of N busy processes / 1):", {k: round(v, 2) for k, v in result["machine_scaling_control"].items()})
    for lang in args.langs.split(","):
        texts = [s.encode() for s in sentences(lang, args.limit)]
        entry: dict = {}
        latency = bench_latency(engines, args.lib, lang, args.limit, args.rounds)
        for which in engines:
            eng = make(which, args.lib)
            e = latency[which]
            e["disk"] = eng.disk()
            scaling = {}
            for n in threads:
                if which == "uccharan":
                    # one pass over the held-out set: the worker threads start
                    # with empty caches, as in the single-thread timing
                    scaling[str(n)] = max(bench_threads(eng, lang, texts, n, 1) for _ in range(7))
                else:
                    # separate processes: the library is not thread-safe
                    best = 0.0
                    for _ in range(2):
                        ps = [spawn(which, args.lib, lang, "full", args.limit, 4) for _ in range(n)]
                        outs = [json.loads(p.communicate()[0]) for p in ps]
                        best = max(best, sum(o["words"] / o["seconds"] for o in outs))
                    scaling[str(n)] = best
            e["words_per_second_by_workers"] = scaling
            e["parallel_method"] = "native threads sharing one handle (batch call)" if which == "uccharan" else "separate processes (library is not thread-safe)"
            noop = process_time("noop", which, args.lib, lang, 15)
            e["cold_start_ms"] = 1e3 * max(0.0, process_time("init", which, args.lib, lang, 15) - noop)
            inproc = [run_child(which, args.lib, lang, "init")["init_plus_first_call_s"] for _ in range(15)]
            e["init_plus_first_call_ms"] = 1e3 * statistics.median(inproc)
            mem = run_child(which, args.lib, lang, "full", args.limit)
            e["rss_after_init_mb"] = mem["rss_after_init"] / 2**20
            e["rss_after_run_mb"] = mem["rss_after_run"] / 2**20
            entry[which] = e
            print(f"[{lang}] {which:9s} p50 {e['latency_us_p50']:7.1f} µs  p99 {e['latency_us_p99']:7.1f} µs  {e['words_per_second_1_thread']:10.0f} words/s  "
                  f"workers {{{', '.join(f'{k}: {v:.0f}' for k, v in scaling.items())}}}  init+first {e['init_plus_first_call_ms']:.2f} ms  "
                  f"rss init {e['rss_after_init_mb']:.1f} MB run {e['rss_after_run_mb']:.1f} MB")
        u_scale = entry["uccharan"]["words_per_second_by_workers"]
        entry["uccharan"]["scaling_vs_1_worker"] = {k: v / u_scale["1"] for k, v in u_scale.items()} if "1" in u_scale else {}
        if "reference" in entry:
            u, r = entry["uccharan"], entry["reference"]
            entry["speedup"] = {
                "latency_p50": r["latency_us_p50"] / u["latency_us_p50"],
                "latency_p99": r["latency_us_p99"] / u["latency_us_p99"],
                "throughput_1_thread": u["words_per_second_1_thread"] / r["words_per_second_1_thread"],
                "throughput_max_workers": u["words_per_second_by_workers"][str(threads[-1])] / r["words_per_second_by_workers"][str(threads[-1])],
                "init_plus_first_call": r["init_plus_first_call_ms"] / u["init_plus_first_call_ms"],
            }
            print(f"[{lang}] speed-up:", {k: round(v, 2) for k, v in entry["speedup"].items()})
        result["languages"][lang] = entry
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(result, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
        print("wrote", args.out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
