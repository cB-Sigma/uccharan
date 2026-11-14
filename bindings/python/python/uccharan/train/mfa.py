r"""Pronunciation dictionaries for forced aligners.

>>> from uccharan.train import mfa_dictionary
>>> mfa_dictionary(["घर", "पानी"], "hi").splitlines()
['घर\tɡʰ ʌ ɾ', 'पानी\tp aː n i']

The Montreal Forced Aligner format: one line per pronunciation, the word, a
tab, and the phones separated by spaces.
"""

from __future__ import annotations

from collections.abc import Iterable, Sequence

import uccharan

from ..textnorm import WordList
from ._phones import foreign_scripts, split_phones, text_words

__all__ = ["mfa_dictionary"]


def mfa_dictionary(
    words_or_sentences: Iterable[str],
    lang: str = "en-us",
    *,
    improved: bool = False,
    wordlists: Sequence[WordList | str] = (),
    stress: bool = False,
    both_modes: bool = False,
    foreign: bool = False,
) -> str:
    """A dictionary for the words of a word list or corpus.

    Every line of the input is split into words (punctuation stripped,
    tokens without letters dropped; Latin-script words are lower-cased, as
    the aligner does). Each word is read on its own, so readings that
    depend on the neighbouring words (weak forms, sentence-final forms) are
    not in the dictionary. Phones are base symbols with their marks
    (``aː``, ``bʰ``, ``ẽ``); stress marks are dropped unless ``stress``.

    ``both_modes`` lists the compatible and the improved reading when they
    differ. Words in another script than the language's are left out unless
    ``foreign`` is set. Words that give no phones are left out."""
    seen: dict[str, None] = {}
    for line in words_or_sentences:
        for w in text_words(line):
            if any(c.isdigit() for c in w):
                continue
            w = w.lower()
            if not foreign and foreign_scripts(w, lang):
                continue
            seen.setdefault(w)
    words = sorted(seen)
    modes = (False, True) if both_modes else (improved,)
    readings: list[list[str]] = []
    for mode in modes:
        if wordlists:
            readings.append([str(uccharan.phonemize_wordlists(w, lang, wordlists=wordlists, improved=mode, punctuation=False)) for w in words])
        else:
            readings.append(uccharan.phonemize_batch(words, lang, "espeak-ipa", improved=mode, punctuation=False))
    lines: list[str] = []
    for i, w in enumerate(words):
        done: set[str] = set()
        for r in readings:
            phones = " ".join(p for c in split_phones(r[i], stress=stress) for p in c)
            if phones and phones not in done:
                done.add(phones)
                lines.append(f"{w}\t{phones}\n")
    return "".join(lines)
