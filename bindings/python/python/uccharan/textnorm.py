"""Normalise-only mode and pronunciation word lists.

>>> import uccharan
>>> n = uccharan.normalize("Pay ₹500 by 10:30.", "en-us")
>>> n.text
'Pay five hundred rupees by ten thirty.'
>>> [(s.kind, s.text) for s in n.spans if s.kind != "punct"][1]
('money', '₹500')

Offsets are code points (Python string indexes) unless ``byte_offsets=True``
is given; ``Span.src_utf16`` and ``Span.out_utf16`` give UTF-16 offsets.
"""

from __future__ import annotations

import json
import os
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, field

from . import _native

__all__ = [
    "Normalized",
    "Respelling",
    "Span",
    "WordList",
    "normalize",
    "phonemize_wordlists",
    "respell",
]


class WordList:
    """A pronunciation word list: TSV (``word lang ipa say match note``),
    W3C PLS 1.0 XML, or a JSON list of entry objects. Parsing errors raise
    ``ValueError`` with the line number."""

    def __init__(self, text: str) -> None:
        self._native = _native.WordList(text)

    @classmethod
    def from_file(cls, path: str | os.PathLike[str]) -> WordList:
        with open(path, encoding="utf-8") as f:
            return cls(f.read())

    @classmethod
    def from_entries(cls, entries: Iterable[Mapping[str, object]]) -> WordList:
        """Entries as mappings with ``word`` and optional ``lang``, ``ipa``,
        ``say``, ``match`` and ``note``."""
        return cls(json.dumps(list(map(dict, entries)), ensure_ascii=False))

    def __len__(self) -> int:
        return len(self._native)

    def entries(self) -> list[dict[str, object]]:
        """The entries as dictionaries (``lang`` is ``"*"`` for any language)."""
        out: list[dict[str, object]] = json.loads(self._native.to_json())
        return out

    def to_tsv(self) -> str:
        return self._native.to_tsv()

    def to_pls(self, lang: str | None = None) -> str:
        """One PLS document for ``lang`` (plus the entries for any language)."""
        return self._native.to_pls(lang)

    def languages(self) -> list[str]:
        return self._native.languages()

    def warnings(self) -> list[tuple[int, str]]:
        return self._native.warnings()


@dataclass(frozen=True)
class Span:
    kind: str
    """text, punct, number, decimal, ordinal, year, money, percent, time,
    date, phone, digits, range, fraction, measure, abbreviation, acronym,
    symbol, url, email, latin, transliterated, wordlist or dropped."""
    lang: str
    src: tuple[int, int]
    """Offsets in the input (code points, or bytes with byte_offsets)."""
    out: tuple[int, int]
    """Offsets in the output text."""
    text: str
    spoken: str
    src_utf16: tuple[int, int]
    out_utf16: tuple[int, int]
    entry: tuple[int, int] | None = None
    """(list index, entry index) for word-list spans."""
    value: str | None = None


@dataclass(frozen=True)
class Normalized:
    text: str
    sentences: list[tuple[int, int]]
    spans: list[Span] = field(default_factory=list)


def _try_need(lang: str) -> None:
    # Normalising needs no language pack (English words fall back to the other
    # accent's built-in data); load the request language's pack when it can be had.
    from . import _need
    from .packs import PackError

    try:
        _need(lang)
    except PackError:
        pass


def _lists(wordlists: Sequence[WordList | str]) -> list[_native.WordList]:
    return [
        w._native if isinstance(w, WordList) else WordList(w)._native for w in wordlists
    ]


def normalize(
    text: str,
    lang: str = "en-us",
    *,
    improved: bool = False,
    digits: str = "context",
    en_scale: str = "auto",
    latin: str = "keep",
    urls: str = "read",
    respell: str = "native",
    wordlists: Sequence[WordList | str] = (),
    byte_offsets: bool = False,
) -> Normalized:
    """The spoken form of ``text`` for a voice that reads text.

    ``digits``: ``context`` (request language; English next to Latin words
    in Indic text), ``request``, or a language tag. ``en_scale``: ``auto``,
    ``indian`` (lakh, crore) or ``international``. ``latin``: ``keep``,
    ``transliterate`` or ``spell`` (Latin words in other scripts). ``urls``:
    ``read``, ``keep`` or ``drop``. ``respell``: ``native``,
    ``latin-hyphen`` or ``latin-plain`` (word-list entries with IPA only).
    Word lists are applied first, the first list winning."""
    _try_need(lang)
    opts = json.dumps(
        {
            "mode": "improved" if improved else "compatible",
            "digits": digits,
            "en_scale": en_scale,
            "latin": latin,
            "urls": urls,
            "respell": respell,
        }
    )
    r = json.loads(_native.normalize(text, lang, opts, _lists(wordlists)))
    unit = "byte" if byte_offsets else "char"

    def rng(x: Mapping[str, list[int]], k: str) -> tuple[int, int]:
        return (x[k][0], x[k][1])

    spans = [
        Span(
            kind=s["kind"],
            lang=s["lang"],
            src=rng(s["src"], unit),
            out=rng(s["out"], unit),
            text=s["text"],
            spoken=s["spoken"],
            src_utf16=rng(s["src"], "utf16"),
            out_utf16=rng(s["out"], "utf16"),
            entry=(s["entry"][0], s["entry"][1]) if "entry" in s else None,
            value=s.get("value"),
        )
        for s in r["spans"]
    ]
    return Normalized(r["text"], [rng(x, unit) for x in r["sentences"]], spans)


def phonemize_wordlists(
    text: str,
    lang: str = "en-us",
    *,
    profile: str = "espeak-ipa",
    wordlists: Sequence[WordList | str] = (),
    normalize: str = "natural",
    improved: bool = False,
    hinglish: bool = False,
    punctuation: bool = True,
    language_flags: bool = False,
) -> str | list[str] | list[list[int]]:
    """Phonemise with word lists: an entry's IPA (converted to the profile's
    phone set) replaces the reading of the words it matches; an entry with
    only a spoken form is phonemised from that form. The result has the
    shape of :func:`uccharan.phonemize` for the profile."""
    from . import _need

    _need(lang, profile, text)
    native_profile = "sherpa-onnx" if profile == "piper" else profile
    out = _native.render_wordlists(
        text,
        lang,
        native_profile,
        normalize,
        improved,
        hinglish,
        punctuation,
        language_flags,
        _lists(wordlists),
    )
    if profile not in ("piper", "sherpa-onnx"):
        return out
    sentences = [[int(c) for c in s.split()] for s in out.split(" | ") if s.strip()]
    if profile == "sherpa-onnx":
        return sentences
    return ["".join(map(chr, s)) for s in sentences]


@dataclass(frozen=True)
class Respelling:
    text: str
    readback: str
    """uccharan's own reading of ``text``."""
    distance: float
    """Phoneme error rate of the reading against the IPA (0 = exact)."""


def respell(
    ipa: str, lang: str, *, source: str | None = None, style: str = "native"
) -> Respelling | None:
    """Spell IPA so that a text voice reading ``lang`` says it: the
    candidate whose reading by uccharan is closest to the IPA."""
    _try_need(lang)
    r = _native.respell(ipa, lang, source, style)
    return None if r is None else Respelling(*r)
