"""Reading-script selection: the few sentences to record for a new voice.

>>> from uccharan.train import select_script
>>> s = select_script(["यह घर है।", "वह घर है।", "खाना अच्छा है।"], "hi", min_symbol_sentences=1)
>>> [x.text for x in s.sentences]
['खाना अच्छा है।', 'वह घर है।', 'यह घर है।']
>>> s.stats["phones"]["covered"] == s.stats["phones"]["total"]
True

A greedy set cover: at each step the sentence that adds the most uncovered
phones (first) and diphones (second) is taken, until nothing is left to
gain or the limit is reached.
"""

from __future__ import annotations

import heapq
import json
import os
import re
from collections import Counter
from collections.abc import Iterable, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import uccharan

from ..textnorm import WordList
from ._phones import diphone_name, diphones, foreign_scripts, split_phones
from .symbols import SymbolTable

__all__ = ["Script", "Selected", "select_script"]

_DIGIT = re.compile(r"\d")
# A new phone outweighs any number of new diphones.
_PHONE_WEIGHT = 1_000_000


@dataclass(frozen=True)
class Selected:
    """One sentence of the script."""

    index: int
    """Position in the corpus (after duplicates were dropped)."""
    text: str
    phonemes: str
    new_phones: tuple[str, ...]
    new_diphones: int
    phones_covered: int
    """Phones covered by the script up to this sentence."""
    diphones_covered: int


@dataclass
class Script:
    sentences: list[Selected]
    stats: dict[str, Any] = field(default_factory=dict)
    """``candidates`` (sentences that passed the filters), ``rejected``
    (count per reason), ``phones`` and ``diphones`` (``covered``, ``total``
    among the candidates, ``missing``), ``corpus_only_phones`` (phones that
    occur only in rejected sentences), ``phone_counts``."""

    def text(self) -> str:
        return "".join(s.text + "\n" for s in self.sentences)

    def tsv(self) -> str:
        rows = ["n\ttext\tphonemes\tnew_phones\tnew_diphones\tphones_covered\tdiphones_covered\n"]
        for n, s in enumerate(self.sentences, start=1):
            rows.append(f"{n}\t{s.text}\t{s.phonemes}\t{' '.join(s.new_phones)}\t{s.new_diphones}\t{s.phones_covered}\t{s.diphones_covered}\n")
        return "".join(rows)

    def write(self, out_dir: str | os.PathLike[str]) -> list[Path]:
        """``script.txt`` (one sentence per line), ``script.tsv`` (with
        phonemes and what each sentence adds) and ``coverage.json``."""
        out = Path(out_dir)
        out.mkdir(parents=True, exist_ok=True)
        files = {"script.txt": self.text(), "script.tsv": self.tsv(), "coverage.json": json.dumps(self.stats, ensure_ascii=False, indent=1) + "\n"}
        for name, body in files.items():
            (out / name).write_text(body, encoding="utf-8", newline="\n")
        return [out / name for name in files]


def select_script(
    sentences: Iterable[str],
    lang: str = "en-us",
    *,
    improved: bool = False,
    normalize: str = "natural",
    wordlists: Sequence[WordList | str] = (),
    max_sentences: int | None = None,
    min_words: int = 3,
    max_words: int = 20,
    max_chars: int | None = None,
    allow_digits: bool = False,
    allow_mixed_script: bool = False,
    min_symbol_sentences: int = 2,
    symbols: SymbolTable | None = None,
    stress: bool = False,
    prefer: str = "sentences",
) -> Script:
    """Pick sentences that cover every phone and as many diphones as
    possible.

    Filters: ``min_words`` to ``max_words`` words (and at most ``max_chars``
    characters), no digits unless ``allow_digits``, no letters of another
    script unless ``allow_mixed_script``, nothing uccharan drops as unknown,
    no phone that occurs in fewer than ``min_symbol_sentences`` sentences of
    the corpus (rare symbols are mostly foreign words and typing errors),
    and, with ``symbols``, no symbol outside that table.

    ``prefer="sentences"`` takes the sentence with the largest gain (fewest
    sentences); ``prefer="length"`` divides the gain by the sentence's
    length in phones (least to read). ``stress=True`` counts stressed and
    unstressed vowels as different phones."""
    if prefer not in ("sentences", "length"):
        raise ValueError(f"unknown prefer {prefer!r} (expected sentences or length)")
    texts = list(dict.fromkeys(" ".join(s.split()) for s in sentences if s.strip()))
    rejected: Counter[str] = Counter()
    pool: list[int] = []
    for i, t in enumerate(texts):
        words = len(t.split())
        if words < min_words:
            rejected["too-short"] += 1
        elif words > max_words or (max_chars is not None and len(t) > max_chars):
            rejected["too-long"] += 1
        elif not allow_digits and _DIGIT.search(t):
            rejected["digits"] += 1
        elif not allow_mixed_script and foreign_scripts(t, lang):
            rejected["mixed-script"] += 1
        else:
            pool.append(i)

    if wordlists:
        rendered = [str(uccharan.phonemize_wordlists(texts[i], lang, wordlists=wordlists, normalize=normalize, improved=improved)) for i in pool]
    else:
        rendered = uccharan.phonemize_batch([texts[i] for i in pool], lang, "espeak-ipa", normalize=normalize, improved=improved)

    # Phones per sentence, and in how many sentences each phone occurs.
    clauses = [split_phones(p, stress=stress) for p in rendered]
    in_sentences: Counter[str] = Counter()
    for cl in clauses:
        in_sentences.update({p for c in cl for p in c})
    all_phones = set(in_sentences)

    cand: list[int] = []  # positions in pool
    for k, cl in enumerate(clauses):
        t = texts[pool[k]]
        flat = [p for c in cl for p in c]
        if not flat:
            rejected["no-phonemes"] += 1
        elif any(in_sentences[p] < min_symbol_sentences for p in flat):
            rejected["rare-symbol"] += 1
        elif symbols is not None and symbols.outside(rendered[k].replace("ˈ", "").replace("ˌ", "") if not stress else rendered[k]):
            rejected["outside-symbols"] += 1
        elif any(s.kind == "dropped" and s.text.strip() for s in uccharan.normalize(t, lang, improved=improved).spans):
            rejected["unknown-character"] += 1
        else:
            cand.append(k)

    phone_sets = {k: frozenset(p for c in clauses[k] for p in c) for k in cand}
    diphone_sets = {k: frozenset(diphones(clauses[k])) for k in cand}
    lengths = {k: sum(len(c) for c in clauses[k]) for k in cand}
    total_phones: set[str] = set().union(*phone_sets.values()) if cand else set()
    total_diphones: set[tuple[str, str]] = set().union(*diphone_sets.values()) if cand else set()
    phone_counts: Counter[str] = Counter()
    for k in cand:
        phone_counts.update(p for c in clauses[k] for p in c)

    covered_p: set[str] = set()
    covered_d: set[tuple[str, str]] = set()

    def gain(k: int) -> float:
        g = len(phone_sets[k] - covered_p) * _PHONE_WEIGHT + len(diphone_sets[k] - covered_d)
        if g == 0:
            return 0.0
        return g / lengths[k] if prefer == "length" else float(g)

    # Gains only shrink as the cover grows, so a stale heap entry is an
    # upper bound: re-score the top until it stays on top.
    heap = [(-gain(k), lengths[k], k) for k in cand]
    heapq.heapify(heap)
    chosen: list[Selected] = []
    while heap and (max_sentences is None or len(chosen) < max_sentences):
        _stale, length, k = heapq.heappop(heap)
        g = gain(k)
        if g <= 0:
            continue
        if heap and -g > heap[0][0]:
            heapq.heappush(heap, (-g, length, k))
            continue
        new_p = phone_sets[k] - covered_p
        new_d = diphone_sets[k] - covered_d
        covered_p |= new_p
        covered_d |= new_d
        chosen.append(
            Selected(
                index=pool[k],
                text=texts[pool[k]],
                phonemes=rendered[k],
                new_phones=tuple(sorted(new_p)),
                new_diphones=len(new_d),
                phones_covered=len(covered_p),
                diphones_covered=len(covered_d),
            )
        )

    script_counts: Counter[str] = Counter()
    for s in chosen:
        script_counts.update(p for c in split_phones(s.phonemes, stress=stress) for p in c)
    stats: dict[str, Any] = {
        "language": lang,
        "mode": "improved" if improved else "compatible",
        "uccharan": uccharan.__version__,
        "corpus_sentences": len(texts),
        "candidates": len(cand),
        "rejected": dict(sorted(rejected.items())),
        "sentences": len(chosen),
        "words": sum(len(s.text.split()) for s in chosen),
        "phones_in_script": sum(script_counts.values()),
        "phones": {
            "covered": len(covered_p),
            "total": len(total_phones),
            "missing": sorted(total_phones - covered_p),
        },
        "diphones": {
            "covered": len(covered_d),
            "total": len(total_diphones),
            "missing": sorted(diphone_name(d) for d in total_diphones - covered_d),
        },
        "corpus_only_phones": {p: in_sentences[p] for p in sorted(all_phones - total_phones)},
        "phone_counts": {p: script_counts[p] for p in sorted(script_counts, key=lambda p: (-script_counts[p], p))},
    }
    return Script(chosen, stats)
