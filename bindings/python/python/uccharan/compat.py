"""A small stand-in for ``phonemizer.backend.EspeakBackend``.

Covers the call shape that TTS code bases actually use::

    from uccharan.compat import EspeakBackend

    backend = EspeakBackend("en-us", preserve_punctuation=True, with_stress=True)
    backend.phonemize(["Hello, world."], strip=True)   # ['həlˈoʊ, wˈɜːld.']

Supported: ``language`` (en-us, en-gb, en-gb-x-rp, hi), ``preserve_punctuation``,
``with_stress``, ``tie``, ``language_switch`` (``keep-flags``, ``remove-flags``),
``words_mismatch`` (accepted, ignored) and, on :meth:`phonemize`, ``strip`` and
``njobs`` (accepted; batches always use all cores). Not supported: custom
``Separator`` objects with phone or syllable separators, ``remove-utterance``,
and any language uccharan has no front end for (a ``ValueError`` is raised).
Numbers are read the way the reference phonemizer reads them.
"""

from __future__ import annotations

import re
from collections.abc import Iterable
from typing import Union

from . import _native, _need

_VOWEL = "aeiouæɑɒɐɔəɚɛɜɪʊʌᵻ"
_STRESS = "ˈˌ"
# Multi-letter phonemes the reference joins with its tie character. The
# r-coloured vowels and the syllabic l are only single phonemes when no vowel
# follows (otherwise the r or l starts the next syllable).
_TIE_ALWAYS = ("tʃ", "dʒ", "aɪ", "aʊ", "eɪ", "oʊ", "əʊ", "ɔɪ", "iə", "eə", "ʊə")
_TIE_BEFORE_NON_VOWEL = {"en-us": ("ːɹ", "ɪɹ", "ʊɹ", "ɛɹ", "əl"), "en": ("əl",), "en-gb": ("əl",), "en-gb-x-rp": ("əl",), "hi": (), "bn": (), "mr": (), "te": (), "ne": (), "kn": (), "ta": (), "ml": (), "as": (), "gu": (), "bho": (), "mai": (), "pa": (), "or": (), "kok": (), "si": (), "sa": (), "ur": (), "sd": (), "mag": (), "doi": (), "brx": (), "sat": (), "es": (), "de": (), "nl": (), "fr": (), "it": (), "pt-br": (), "pt": (), "pl": (), "tr": (), "cs": (), "sv": (), "ca": (), "ro": (), "hu": (), "fi": (), "nb": (), "da": (), "fr-be": (), "fr-ch": ()}
_PUNCT = re.compile(r"[,.;:!?]")


def _tie(s: str, tie: str, language: str) -> str:
    units = [(u, False) for u in _TIE_ALWAYS] + [(u, True) for u in _TIE_BEFORE_NON_VOWEL.get(language, ())]
    out = []
    i = 0
    while i < len(s):
        for unit, needs_non_vowel in units:
            if s.startswith(unit, i):
                j = i + len(unit)
                k = j
                while k < len(s) and s[k] in _STRESS:
                    k += 1
                if needs_non_vowel and k < len(s) and s[k] in _VOWEL:
                    continue
                out.append(unit[0] + tie + unit[1:])
                i = j
                break
        else:
            out.append(s[i])
            i += 1
    return "".join(out)


class EspeakBackend:
    """Drop-in for the common uses of phonemizer's ``EspeakBackend``."""

    def __init__(
        self,
        language: str,
        punctuation_marks: Union[str, None] = None,
        preserve_punctuation: bool = False,
        with_stress: bool = False,
        tie: Union[bool, str] = False,
        language_switch: str = "keep-flags",
        words_mismatch: str = "ignore",
        logger: object = None,
    ) -> None:
        if language.lower().replace("_", "-") not in ("en-us", "en", "en-gb", "en-gb-x-rp", "hi", "bn", "mr", "te", "ne", "kn", "ta", "ml", "as", "gu", "bho", "mai", "pa", "or", "kok", "si", "sa", "ur", "sd", "mag", "doi", "brx", "sat", "es", "de", "nl", "fr", "it", "pt-br", "pt", "pl", "tr", "cs", "sv", "ca", "ro", "hu", "fi", "nb", "da", "fr-be", "fr-ch"):
            raise ValueError(f"language {language!r} is not supported by uccharan (en-us, en-gb, en-gb-x-rp, hi, bn, mr, te, ne, kn, ta, ml, as, gu)")
        if language_switch not in ("keep-flags", "remove-flags"):
            raise ValueError("language_switch must be 'keep-flags' or 'remove-flags'")
        self.language = language
        self.preserve_punctuation = preserve_punctuation
        self.with_stress = with_stress
        self.tie = "͡" if tie is True else (tie or "")
        self.language_switch = language_switch

    @staticmethod
    def name() -> str:
        return "uccharan"

    @staticmethod
    def is_available() -> bool:
        return True

    @staticmethod
    def version() -> tuple[int, ...]:
        return tuple(int(x) for x in _native.__version__.split("."))

    def phonemize(self, text: Iterable[str], separator: object = None, strip: bool = False, njobs: int = 1) -> list[str]:
        """Phonemize a list of utterances; one output string per input string."""
        if isinstance(text, str):
            raise TypeError("text must be a list of strings, as in phonemizer >= 3")
        out: list[str] = []
        for line in text:
            # phonemizer's punctuation set has no danda: it ends a clause but is not kept.
            pieces = re.split(r"[।॥]+", line) if self.preserve_punctuation else [line]
            _need(self.language, "", line)
            s = " ".join(
                p for p in (
                    _native.render(piece, self.language, "espeak-ipa", "espeak", False, False, self.preserve_punctuation,
                                   self.language_switch == "keep-flags")
                    for piece in pieces
                ) if p
            )
            if not self.with_stress:
                s = s.replace("ˈ", "").replace("ˌ", "")
            if self.tie:
                s = _tie(s, self.tie, self.language.lower().replace("_", "-"))
            # phonemizer ends every word with the word separator unless strip=True
            out.append(s if strip or not s else s + " ")
        return out
