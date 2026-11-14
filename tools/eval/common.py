"""Shared helpers for the evaluation tools: running uccharan, loading the
optional oracle, alignment and error classification."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "oracle"))

VOICES = {"bho": "hi", "mai": "hi", "en-us": "en-us", "en-gb": "en-gb-x-rp", "hi": "hi", "bn": "bn", "mr": "mr", "te": "te", "ne": "ne", "kn": "kn", "ta": "ta", "ml": "ml", "as": "as", "gu": "gu", "pa": "pa", "or": "or", "kok": "kok", "si": "si", "sa": "hi"}
STRESS = "ˈˌ"
VOWELS = set("aeiouyæɑɒɐɔəɚɛɜɝɪʊʌᵻɨøœɵãẽĩõũ")
MARKS = set("ː̩̪̯̃")


def find_binary(explicit: str | None = None) -> str:
    """Path of the `uccharan` command-line program."""
    candidates = [explicit, os.environ.get("UCCHARAN_BIN"), str(ROOT / "target" / "release" / "uccharan"), shutil.which("uccharan")]
    for c in candidates:
        if c and Path(c).exists():
            return c
    raise SystemExit("uccharan binary not found: run `cargo build --release` or set UCCHARAN_BIN")


def run_uccharan(lines: list[str], lang: str, profile: str = "espeak-ipa", *, normalize: str = "espeak",
                 punctuation: bool = False, binary: str | None = None, extra: tuple[str, ...] = ()) -> list[str]:
    """One output line per input line."""
    args = [find_binary(binary), "-l", lang, "-p", profile, "--normalize", normalize, *extra]
    if not punctuation:
        args.append("--no-punctuation")
    text = "\n".join(line.replace("\n", " ") for line in lines) + "\n"
    out = subprocess.run(args, input=text, capture_output=True, text=True, check=True).stdout.split("\n")
    return out[: len(lines)]


def load_oracle():
    """The reference phonemizer, or None when it is not installed."""
    try:
        import espeak_oracle  # type: ignore[import-not-found]

        return espeak_oracle.find_oracle()
    except Exception:
        return None


def nfd(s: str) -> list[str]:
    return list(unicodedata.normalize("NFD", s))


def read_tsv(path: Path) -> list[tuple[str, str]]:
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if "\t" in line:
            i, t = line.split("\t", 1)
            rows.append((i, t))
    return rows


def align(ref: list[str], hyp: list[str]) -> tuple[int, list[tuple[str, str, str]]]:
    """Levenshtein distance and the edit script as (op, ref_symbol, hyp_symbol)
    with op in 'sub', 'del' (missing from hyp), 'ins' (extra in hyp)."""
    n, m = len(ref), len(hyp)
    if ref == hyp:
        return 0, []
    prev = list(range(m + 1))
    rows = [prev]
    for i in range(1, n + 1):
        cur = [i] + [0] * m
        ri = ref[i - 1]
        for j in range(1, m + 1):
            c = prev[j - 1] + (ri != hyp[j - 1])
            d = prev[j] + 1
            e = cur[j - 1] + 1
            cur[j] = c if c <= d and c <= e else (d if d <= e else e)
        rows.append(cur)
        prev = cur
    ops: list[tuple[str, str, str]] = []
    i, j = n, m
    while i > 0 or j > 0:
        if i > 0 and j > 0 and rows[i][j] == rows[i - 1][j - 1] + (ref[i - 1] != hyp[j - 1]):
            if ref[i - 1] != hyp[j - 1]:
                ops.append(("sub", ref[i - 1], hyp[j - 1]))
            i, j = i - 1, j - 1
        elif i > 0 and rows[i][j] == rows[i - 1][j] + 1:
            ops.append(("del", ref[i - 1], ""))
            i -= 1
        else:
            ops.append(("ins", "", hyp[j - 1]))
            j -= 1
    ops.reverse()
    return rows[n][m], ops


def op_class(op: tuple[str, str, str], lang: str) -> str:
    """Coarse class of one edit operation."""
    _, r, h = op
    syms = [s for s in (r, h) if s]
    if all(s in STRESS for s in syms):
        return "stress"
    if any(s in STRESS for s in syms):
        return "stress"
    if all(s == " " for s in syms):
        return "word-boundary"
    if all(s == "ː" for s in syms):
        return "length"
    if lang == "hi" and all(s in "əʌ" for s in syms):
        return "schwa" if len(syms) == 1 else "vowel"
    if all(s in MARKS for s in syms):
        return "diacritic"
    if all(s in VOWELS for s in syms):
        return "vowel"
    if not any(s in VOWELS for s in syms) and not any(s in MARKS or s == " " for s in syms):
        return "consonant"
    return "other"


def stress_pattern(word: str) -> str:
    """Stress level per vowel nucleus, e.g. 'həlˈoʊ' -> '01'."""
    out = []
    level = "0"
    in_vowel = False
    for c in unicodedata.normalize("NFD", word):
        if c == "ˈ":
            level = "1"
            in_vowel = False
        elif c == "ˌ":
            level = "2"
            in_vowel = False
        elif c in VOWELS:
            if not in_vowel:
                out.append(level)
                level = "0"
            in_vowel = True
        elif c in MARKS:
            pass
        else:
            in_vowel = False
    return "".join(out)


def strip_stress(s: str) -> str:
    return s.replace("ˈ", "").replace("ˌ", "")
