"""Sound-alike matching: correct a transcript against a vocabulary.

>>> import uccharan
>>> index = uccharan.SoundIndex(["Kothari", "Paytm"])
>>> fixed = index.apply("send it to Coat Harry", "en-us")
>>> fixed.text
'send it to Kothari'
>>> [(c.old, c.new) for c in fixed.changes]
[('Coat Harry', 'Kothari')]

Everything is compared as phonemes, so a vocabulary in Latin script also
matches a transcript in Devanagari and the other way round. Nothing is
rewritten silently: :meth:`SoundIndex.find` only reports candidates, and
:meth:`SoundIndex.apply` returns every change it made.

Offsets are code points (Python string indexes) unless ``byte_offsets=True``
is given; ``src_utf16`` gives UTF-16 offsets.
"""

from __future__ import annotations

import json
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, field

from . import _native
from .textnorm import WordList, _try_need

__all__ = ["Candidate", "Change", "Corrected", "SoundIndex", "match_terms"]


@dataclass(frozen=True)
class Candidate:
    """A place in the transcript that sounds like a term."""

    src: tuple[int, int]
    """Offsets of the span in the transcript."""
    text: str
    """The span as the transcript has it."""
    term: str
    index: int
    """Position of the term in the vocabulary."""
    score: float
    """Similarity, 0 to 1."""
    exact: bool
    """The transcript already has the term here (any case)."""
    kind: str
    """``unknown`` (an English transcript has a word here that is not in the
    lexicon), ``ordinary`` (every word is an ordinary word) or ``uncertain``."""
    words: tuple[int, int]
    """Whitespace-separated words the span covers (start, end)."""
    src_utf16: tuple[int, int]


@dataclass(frozen=True)
class Change:
    """One replacement made by :meth:`SoundIndex.apply`."""

    src: tuple[int, int]
    """Offsets of the replaced text in the transcript."""
    out: tuple[int, int]
    """Offsets of the replacement in the new text."""
    old: str
    new: str
    index: int
    score: float
    case_only: bool
    src_utf16: tuple[int, int]
    out_utf16: tuple[int, int]


@dataclass(frozen=True)
class Corrected:
    text: str
    changes: list[Change] = field(default_factory=list)
    candidates: list[Candidate] = field(default_factory=list)


def _rng(x: Mapping[str, list[int]], k: str) -> tuple[int, int]:
    return (x[k][0], x[k][1])


def _candidates(items: list[dict], unit: str) -> list[Candidate]:
    return [
        Candidate(
            src=_rng(c["src"], unit),
            text=c["text"],
            term=c["term"],
            index=c["index"],
            score=c["score"],
            exact=c["exact"],
            kind=c["kind"],
            words=(c["words"][0], c["words"][1]),
            src_utf16=_rng(c["src"], "utf16"),
        )
        for c in items
    ]


class SoundIndex:
    """A vocabulary prepared for matching. Build it once and reuse it.

    ``vocabulary`` is a :class:`uccharan.WordList` (its ``ipa`` and ``say``
    columns give the sound of an entry), word-list text (TSV, PLS or JSON),
    text with one term per line, or an iterable of terms: strings, or
    mappings with ``word`` and optional ``lang``, ``ipa`` and ``say``.

    ``langs`` are the languages a term without phonemes is read in
    (default ``("en-us", "hi")``). ``ordinary_words`` adds to the words the
    matcher takes for ordinary speech (frequent words of a language or
    domain that the built-in lists do not cover).
    """

    def __init__(
        self,
        vocabulary: WordList | str | Iterable[str | Mapping[str, object]],
        *,
        langs: Sequence[str] | None = None,
        ordinary_words: Iterable[str] = (),
    ) -> None:
        if isinstance(vocabulary, WordList):
            text = vocabulary.to_tsv()
        elif isinstance(vocabulary, str):
            text = vocabulary
        else:
            text = json.dumps([t if isinstance(t, str) else dict(t) for t in vocabulary], ensure_ascii=False)
        opts: dict[str, object] = {"ordinary_words": list(ordinary_words)}
        if langs is not None:
            opts["langs"] = list(langs)
        options = json.dumps(opts, ensure_ascii=False)
        for lang in langs if langs is not None else ("en-us", "hi"):
            _try_need(lang)
        self._native = _native.SoundIndex(text, options)

    def __len__(self) -> int:
        return len(self._native)

    def terms(self) -> list[str]:
        return self._native.terms()

    def sounds(self, index: int) -> list[str]:
        """The phoneme sequences stored for a term (the matcher's broad notation)."""
        return self._native.sounds(index)

    def skipped(self) -> list[tuple[int, str]]:
        """Terms without usable phonemes: ``(index, reason)``."""
        return self._native.skipped()

    def find(
        self,
        text: str,
        lang: str = "en-us",
        *,
        max_words: int = 3,
        min_score: float = 0.6,
        all: bool = False,
        byte_offsets: bool = False,
    ) -> list[Candidate]:
        """Candidates in ``text`` (a transcript in ``lang``), by position.

        Without ``all`` the candidates do not overlap (an exact occurrence
        wins, then the higher score); with it, every span and term above
        ``min_score`` is reported."""
        _try_need(lang)
        opts = json.dumps({"max_words": max_words, "min_score": min_score, "all": all})
        r = json.loads(self._native.find(text, lang, opts))
        return _candidates(r["candidates"], "byte" if byte_offsets else "char")

    def apply(
        self,
        text: str,
        lang: str = "en-us",
        *,
        threshold: float | None = None,
        uncertain_threshold: float | None = None,
        ordinary_threshold: float | None = None,
        confidence: Sequence[float] | None = None,
        max_confidence: float | None = None,
        fix_case: bool = True,
        max_words: int = 3,
        byte_offsets: bool = False,
    ) -> Corrected:
        """Replace what sounds like a term with the term.

        A candidate is applied when its score reaches ``threshold``; a span
        of the ``uncertain`` or ``ordinary`` kind also has to reach
        ``uncertain_threshold`` or ``ordinary_threshold``. ``confidence`` is
        the recogniser's confidence for each whitespace-separated word of
        ``text``; with it, a span is corrected only when the mean confidence
        of its words is at most ``max_confidence`` (default 0.9). The
        result lists every change."""
        _try_need(lang)
        opts: dict[str, object] = {"max_words": max_words, "fix_case": fix_case}
        if threshold is not None:
            opts["threshold"] = threshold
        if uncertain_threshold is not None:
            opts["uncertain_threshold"] = uncertain_threshold
        if ordinary_threshold is not None:
            opts["ordinary_threshold"] = ordinary_threshold
        if confidence is not None:
            opts["confidence"] = [float(x) for x in confidence]
        if max_confidence is not None:
            opts["max_confidence"] = max_confidence
        r = json.loads(self._native.apply(text, lang, json.dumps(opts)))
        unit = "byte" if byte_offsets else "char"
        changes = [
            Change(
                src=_rng(c["src"], unit),
                out=_rng(c["out"], unit),
                old=c["from"],
                new=c["to"],
                index=c["index"],
                score=c["score"],
                case_only=c["case_only"],
                src_utf16=_rng(c["src"], "utf16"),
                out_utf16=_rng(c["out"], "utf16"),
            )
            for c in r["changes"]
        ]
        return Corrected(r["text"], changes, _candidates(r["candidates"], unit))


def match_terms(
    text: str,
    vocabulary: SoundIndex | WordList | str | Iterable[str | Mapping[str, object]],
    lang: str = "en-us",
    *,
    max_words: int = 3,
    min_score: float = 0.6,
    all: bool = False,
    byte_offsets: bool = False,
) -> list[Candidate]:
    """Places in ``text`` that sound like a term of ``vocabulary``.

    Builds a :class:`SoundIndex` for the call; keep one yourself when the
    same vocabulary is used again."""
    index = vocabulary if isinstance(vocabulary, SoundIndex) else SoundIndex(vocabulary)
    return index.find(text, lang, max_words=max_words, min_score=min_score, all=all, byte_offsets=byte_offsets)
