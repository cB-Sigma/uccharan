"""Coverage report for a dataset: which phones and diphones it has and how
often, what is missing, how long each phone lasts (given alignments), and
which utterances stand out."""

from __future__ import annotations

import json
import os
import re
import statistics
from collections import Counter
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import uccharan

from .. import _native
from ._phones import (
    diphone_name,
    diphones,
    foreign_scripts,
    scripts,
    split_phones,
    text_words,
)
from .label import Labelled, LabelledUtterance, label
from .manifest import Utterance
from .symbols import SymbolTable

__all__ = ["CoverageReport", "Outlier", "coverage", "read_alignments"]

# Labels that aligners use for silence and noise.
_NON_SPEECH = frozenset(("", "sil", "sp", "spn", "<eps>", "SIL", "<unk>", "_", "pau", "#"))


@dataclass(frozen=True)
class Outlier:
    id: str
    reason: str
    """``too-long``, ``too-short``, ``out-of-vocabulary``, ``wrong-script``,
    ``speaking-rate`` or ``no-phonemes``."""
    detail: str
    value: float


@dataclass
class CoverageReport:
    utterances: int
    phones: dict[str, int]
    diphones: dict[str, int]
    missing_phones: list[str]
    """Phones of the reference (or symbols of the target table) that the
    dataset never has."""
    missing_diphones: list[tuple[str, int]]
    """Diphones of the reference that the dataset lacks, with their count
    in the reference, most frequent first."""
    rare_phones: dict[str, int]
    """Phones seen fewer than ``min_count`` times."""
    durations: dict[str, dict[str, float]]
    """Per phone label of the alignments: ``count``, ``total``, ``mean``,
    ``median``, ``min``, ``max`` (seconds)."""
    outliers: list[Outlier]
    totals: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, object]:
        return {
            "totals": self.totals,
            "phones": self.phones,
            "diphones": self.diphones,
            "missing_phones": self.missing_phones,
            "missing_diphones": [{"diphone": d, "reference_count": n} for d, n in self.missing_diphones],
            "rare_phones": self.rare_phones,
            "durations": self.durations,
            "outliers": [{"id": o.id, "reason": o.reason, "detail": o.detail, "value": o.value} for o in self.outliers],
        }

    def to_json(self) -> str:
        return json.dumps(self.to_dict(), ensure_ascii=False, indent=1) + "\n"

    def summary(self, limit: int = 15) -> str:
        """A short text report."""
        t = self.totals
        lines = [
            f"utterances        {self.utterances}",
            f"phones            {t['phone_tokens']} tokens, {len(self.phones)} different",
            f"diphones          {len(self.diphones)} different"
            + (f"; {t['reference_diphones'] - len(self.missing_diphones)} of the {t['reference_diphones']} in the reference ({t['diphone_coverage']:.1%})" if "reference_diphones" in t else ""),
        ]
        if "hours" in t:
            lines.append(f"audio             {t['hours']:.2f} h")
        if self.missing_phones:
            lines.append(f"missing phones    {' '.join(self.missing_phones)}")
        if self.rare_phones:
            lines.append("rare phones       " + " ".join(f"{p}×{n}" for p, n in list(self.rare_phones.items())[:limit]))
        if self.missing_diphones:
            lines.append(f"missing diphones  {len(self.missing_diphones)}; most frequent in the reference: " + " ".join(f"{d}×{n}" for d, n in self.missing_diphones[:limit]))
        if self.durations:
            longest = sorted(self.durations.items(), key=lambda kv: -kv[1]["mean"])[:limit]
            lines.append("mean duration     " + " ".join(f"{p} {v['mean'] * 1000:.0f}ms" for p, v in longest))
        by_reason = Counter(o.reason for o in self.outliers)
        lines.append("outliers          " + (", ".join(f"{r} {n}" for r, n in sorted(by_reason.items())) or "none"))
        for o in self.outliers[:limit]:
            lines.append(f"  {o.id}: {o.reason} ({o.detail})")
        return "\n".join(lines) + "\n"


# -- alignments ---------------------------------------------------------------

_TG_INTERVAL = re.compile(r'xmin\s*=\s*([\d.eE+-]+)\s*xmax\s*=\s*([\d.eE+-]+)\s*text\s*=\s*"((?:[^"]|"")*)"')
_TG_TIER = re.compile(r'name\s*=\s*"((?:[^"]|"")*)"')


def _textgrid(text: str, tier: str) -> list[tuple[str, float, float]]:
    """Intervals of the phone tier of a Praat TextGrid (long format)."""
    out: list[tuple[str, float, float]] = []
    parts = re.split(r"item\s*\[\d+\]\s*:", text)
    chosen = None
    for part in parts[1:]:
        m = _TG_TIER.search(part)
        if m and m.group(1).lower() == tier.lower():
            chosen = part
            break
    if chosen is None:
        cand = [p for p in parts[1:] if (m := _TG_TIER.search(p)) and "phone" in m.group(1).lower()]
        chosen = cand[0] if cand else (parts[-1] if len(parts) > 1 else "")
    for m in _TG_INTERVAL.finditer(chosen):
        out.append((m.group(3).replace('""', '"').strip(), float(m.group(1)), float(m.group(2))))
    return out


def read_alignments(path: str | os.PathLike[str], *, tier: str = "phones") -> dict[str, list[tuple[str, float, float]]]:
    """Phone alignments as ``{utterance id: [(phone, start, end), …]}``.

    ``path`` is a directory of Praat TextGrid files (long format, as the
    Montreal Forced Aligner writes; the file stem is the utterance id; the
    ``phones`` tier is read), a JSON-lines file with ``id`` and ``phones``
    (objects with ``phone``, ``start`` and ``end``, or ``duration``), or a
    CTM file (``id channel start duration phone``)."""
    p = Path(path)
    out: dict[str, list[tuple[str, float, float]]] = {}
    if p.is_dir():
        for f in sorted(p.rglob("*.TextGrid")):
            out[f.stem] = _textgrid(f.read_text(encoding="utf-8-sig"), tier)
        return out
    text = p.read_text(encoding="utf-8-sig")
    if p.suffix.lower() == ".textgrid":
        return {p.stem: _textgrid(text, tier)}
    if text.lstrip().startswith("{"):
        for line in text.splitlines():
            if not line.strip():
                continue
            row = json.loads(line)
            items = []
            for ph in row.get("phones", []):
                start = float(ph.get("start", 0.0))
                end = float(ph["end"]) if "end" in ph else start + float(ph.get("duration", 0.0))
                items.append((str(ph.get("phone", ph.get("label", ""))), start, end))
            out[str(row["id"])] = items
        return out
    for n, line in enumerate(text.splitlines(), start=1):
        cols = line.split()
        if not cols or cols[0].startswith(";;"):
            continue
        if len(cols) < 5:
            raise ValueError(f"{p}:{n}: expected 'id channel start duration phone'")
        start, dur = float(cols[2]), float(cols[3])
        out.setdefault(cols[0], []).append((cols[4], start, start + dur))
    return out


# -- the report ---------------------------------------------------------------


def _fences(values: Sequence[float], k: float = 3.0) -> tuple[float, float]:
    if len(values) < 8:
        return (float("-inf"), float("inf"))
    q = statistics.quantiles(values, n=4)
    iqr = q[2] - q[0]
    return (q[0] - k * iqr, q[2] + k * iqr)


def coverage(
    dataset: Labelled | Iterable[Utterance] | Iterable[LabelledUtterance],
    lang: str = "en-us",
    *,
    improved: bool = False,
    reference: Iterable[str] | None = None,
    symbols: SymbolTable | None = None,
    alignments: Mapping[str, Sequence[tuple[str, float, float]]] | None = None,
    vocabulary: Iterable[str] | None = None,
    min_count: int = 5,
    max_phones: int | None = None,
    max_seconds: float | None = None,
    max_oov: float = 0.3,
    min_script: float = 0.5,
    stress: bool = False,
) -> CoverageReport:
    """Report what a dataset covers.

    ``dataset`` is the result of :func:`label`, labelled utterances, or
    plain utterances (phonemised here in ``lang`` and the given mode).

    Missing phones and diphones are judged against ``reference``: sentences
    of the language (a text corpus much larger than the dataset), read the
    same way. Without it, only phones can be missing, and only against the
    ``symbols`` table. ``alignments`` (see :func:`read_alignments`) give the
    duration per phone. ``vocabulary`` is a list of known words; a word
    outside it is out of vocabulary. Without it, English words are checked
    against the built-in lexicon and words of other languages only by
    their script.

    Outliers: more phones than ``max_phones`` or longer than ``max_seconds``
    (by default beyond three interquartile ranges above the third
    quartile), far too short, more than ``max_oov`` of the words out of
    vocabulary, fewer than ``min_script`` of the letters in the language's
    script, and a speaking rate (phones per second) far from the rest."""
    if isinstance(dataset, Labelled):
        items = dataset.utterances
    else:
        rows = list(dataset)
        if rows and isinstance(rows[0], Utterance):
            items = label(rows, lang, profile="espeak-ipa", improved=improved).utterances  # type: ignore[arg-type]
        else:
            items = rows  # type: ignore[assignment]

    phones: Counter[str] = Counter()
    pairs: Counter[tuple[str, str]] = Counter()
    n_phones: list[int] = []
    for u in items:
        cl = split_phones(u.phonemes, stress=stress)
        phones.update(p for c in cl for p in c)
        pairs.update(diphones(cl))
        n_phones.append(sum(len(c) for c in cl))

    totals: dict[str, Any] = {
        "utterances": len(items),
        "phone_tokens": sum(phones.values()),
        "phones": len(phones),
        "diphones": len(pairs),
        "uccharan": uccharan.__version__,
    }
    seconds = [u.duration for u in items if u.duration is not None]
    if seconds:
        totals["hours"] = sum(seconds) / 3600.0

    missing_phones: list[str] = []
    missing_diphones: list[tuple[str, int]] = []
    if reference is not None:
        ref_texts = [t for t in reference if t.strip()]
        ref_p: Counter[str] = Counter()
        ref_d: Counter[tuple[str, str]] = Counter()
        for p in uccharan.phonemize_batch(ref_texts, lang, "espeak-ipa", improved=improved):
            cl = split_phones(p, stress=stress)
            ref_p.update(x for c in cl for x in c)
            ref_d.update(diphones(cl))
        missing_phones = sorted((p for p in ref_p if p not in phones), key=lambda p: (-ref_p[p], p))
        missing_diphones = sorted(((diphone_name(d), n) for d, n in ref_d.items() if d not in pairs), key=lambda x: (-x[1], x[0]))
        totals["reference_sentences"] = len(ref_texts)
        totals["reference_phones"] = len(ref_p)
        totals["reference_diphones"] = len(ref_d)
        totals["diphone_coverage"] = (len(ref_d) - len(missing_diphones)) / len(ref_d) if ref_d else 1.0
        tokens = sum(ref_d.values())
        totals["diphone_coverage_by_frequency"] = (tokens - sum(n for _, n in missing_diphones)) / tokens if tokens else 1.0
    elif symbols is not None:
        seen = {c for u in items for c in u.phonemes}
        skip = {symbols.pad, symbols.bos, symbols.eos, " "}
        missing_phones = [s for s, _ in symbols.by_id() if s not in seen and s not in skip]
    rare = {p: n for p, n in sorted(phones.items(), key=lambda kv: (kv[1], kv[0])) if n < min_count}

    durations: dict[str, dict[str, float]] = {}
    if alignments:
        per: dict[str, list[float]] = {}
        for seq in alignments.values():
            for ph, start, end in seq:
                if ph in _NON_SPEECH:
                    continue
                per.setdefault(ph, []).append(max(0.0, end - start))
        for ph in sorted(per, key=lambda k: (-round(sum(per[k]), 4), k)):
            v = per[ph]
            durations[ph] = {
                "count": len(v),
                "total": round(sum(v), 4),
                "mean": round(statistics.fmean(v), 4),
                "median": round(statistics.median(v), 4),
                "min": round(min(v), 4),
                "max": round(max(v), 4),
            }
        totals["aligned_utterances"] = len(alignments)
        totals["aligned_seconds"] = round(sum(x["total"] for x in durations.values()), 3)

    # -- outliers
    vocab = {w.lower() for w in vocabulary} if vocabulary is not None else None
    outliers: list[Outlier] = []
    hi_phones = float(max_phones) if max_phones is not None else _fences([float(n) for n in n_phones])[1]
    hi_seconds = max_seconds if max_seconds is not None else _fences([float(s) for s in seconds])[1]
    rates = [n / u.duration for u, n in zip(items, n_phones) if u.duration]
    lo_rate, hi_rate = _fences(rates)
    for u, n in zip(items, n_phones):
        if n == 0:
            outliers.append(Outlier(u.id, "no-phonemes", "nothing to say", 0.0))
            continue
        if n > hi_phones:
            outliers.append(Outlier(u.id, "too-long", f"{n} phones", float(n)))
        elif u.duration is not None and u.duration > hi_seconds:
            outliers.append(Outlier(u.id, "too-long", f"{u.duration:.1f} s", u.duration))
        if n < 3:
            outliers.append(Outlier(u.id, "too-short", f"{n} phones", float(n)))
        if u.duration:
            rate = n / u.duration
            if rate < lo_rate or rate > hi_rate:
                outliers.append(Outlier(u.id, "speaking-rate", f"{rate:.1f} phones per second", rate))
        found = scripts(u.text)
        letters = sum(found.values())
        foreign = sum(foreign_scripts(u.text, u.lang).values())
        if letters and (letters - foreign) / letters < min_script:
            outliers.append(Outlier(u.id, "wrong-script", ", ".join(sorted(s.capitalize() for s in foreign_scripts(u.text, u.lang))), (letters - foreign) / letters))
        words = text_words(u.normalized or u.text)
        if words:
            oov = []
            for w in words:
                if vocab is not None:
                    known = w.lower() in vocab
                elif foreign_scripts(w, u.lang):
                    known = False
                else:
                    known = _native.in_lexicon(w, u.lang) is not False
                if not known:
                    oov.append(w)
            if len(oov) / len(words) > max_oov:
                outliers.append(Outlier(u.id, "out-of-vocabulary", " ".join(dict.fromkeys(oov)), len(oov) / len(words)))

    return CoverageReport(
        utterances=len(items),
        phones=dict(sorted(phones.items(), key=lambda kv: (-kv[1], kv[0]))),
        diphones={diphone_name(d): n for d, n in sorted(pairs.items(), key=lambda kv: (-kv[1], kv[0]))},
        missing_phones=missing_phones,
        missing_diphones=missing_diphones,
        rare_phones=rare,
        durations=durations,
        outliers=outliers,
        totals=totals,
    )
