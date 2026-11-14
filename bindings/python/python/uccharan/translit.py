"""Romanised text into Indian scripts.

>>> import uccharan
>>> uccharan.load_pack("xlit-hi")            # CC BY-SA 4.0, see below
'xlit-hi'
>>> uccharan.transliterate("mujhe kal office jana hai", "hi").text
'मुझे कल office जाना है'
>>> uccharan.transliterate("mujhe kal office jana hai", "hi", english="native").text
'मुझे कल ऑफ़िस जाना है'

The work is done with a language pack: ``xlit-hi``, ``xlit-mr``, ``xlit-bn``,
``xlit-ta``, ``xlit-te``, ``xlit-gu``, ``xlit-pa``, ``xlit-kn`` and
``xlit-ml``. The packs are **CC BY-SA 4.0**, not Apache-2.0 (they are trained
on the Dakshina and Aksharantar word pairs), so they are never fetched on
their own: call ``uccharan.load_pack("xlit-hi")`` if that licence suits your
use. Without a pack a plain rule converter answers (for Hindi, the small
built-in list of common romanised words and its rules), which is much less
accurate; ``Transliterated.pack`` says which of the two was used.

Offsets are code points (Python string indexes) unless ``byte_offsets=True``
is given; ``Span.src_utf16`` and ``Span.out_utf16`` give UTF-16 offsets.
"""

from __future__ import annotations

import json
from collections.abc import Mapping
from dataclasses import dataclass, field

from . import _native

__all__ = ["Alternative", "TranslitSpan", "Transliterated", "has_translit", "transliterate"]


@dataclass(frozen=True)
class Alternative:
    text: str
    score: float
    """Share of belief among the word's candidates, 0 to 1."""


@dataclass(frozen=True)
class TranslitSpan:
    kind: str
    """``indic`` (a romanised word, written in the script), ``english`` or
    ``other`` (numbers, punctuation, spaces, text already in a native script)."""
    src: tuple[int, int]
    """Offsets in the input (code points, or bytes with byte_offsets)."""
    out: tuple[int, int]
    """Offsets in the output text."""
    text: str
    written: str
    src_utf16: tuple[int, int]
    out_utf16: tuple[int, int]
    alternatives: list[Alternative] = field(default_factory=list)
    """Native candidates of a Latin word, the chosen one first."""


@dataclass(frozen=True)
class Transliterated:
    text: str
    spans: list[TranslitSpan]
    pack: bool
    """True when the language's transliteration pack did the work."""


def has_translit(lang: str) -> bool:
    """True when the transliteration pack of ``lang`` is loaded."""
    return _native.has_translit(lang)


def transliterate(
    text: str,
    lang: str = "hi",
    *,
    english: str = "keep",
    alternatives: int = 3,
    context: bool = True,
    byte_offsets: bool = False,
) -> Transliterated:
    """Write the romanised words of ``text`` in the script of ``lang``.

    ``english``: ``keep`` (English words stay in Latin letters), ``native``
    (they are respelled in the script) or ``convert`` (no language decision:
    every Latin word is taken for a romanised word). ``alternatives``: how
    many candidates to list per word. ``context``: let neighbouring words
    decide between candidates and between English and the language."""
    from .textnorm import _try_need

    _try_need(lang, text)
    opts = json.dumps({"english": english, "alternatives": alternatives, "context": context})
    r = json.loads(_native.transliterate(text, lang, opts))
    unit = "byte" if byte_offsets else "char"

    def rng(x: Mapping[str, list[int]], k: str) -> tuple[int, int]:
        return (x[k][0], x[k][1])

    spans = [
        TranslitSpan(
            kind=s["kind"],
            src=rng(s["src"], unit),
            out=rng(s["out"], unit),
            text=s["text"],
            written=s["written"],
            src_utf16=rng(s["src"], "utf16"),
            out_utf16=rng(s["out"], "utf16"),
            alternatives=[Alternative(a["text"], a["score"]) for a in s.get("alternatives", [])],
        )
        for s in r["spans"]
    ]
    return Transliterated(r["text"], spans, _native.has_translit(lang))
