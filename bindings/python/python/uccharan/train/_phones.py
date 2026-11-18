"""Phones, scripts and small text helpers shared by the training tools."""

from __future__ import annotations

import unicodedata
from collections.abc import Iterable, Iterator

STRESS = "ˈˌ"
# Marks that belong to the phone before them.
_MODIFIERS = frozenset("ːˑʰʱʲʷˠˤ˞ʼʴ")
# A tie bar joins the phone before it with the phone after it.
_TIES = frozenset("͜͡")
# Clause punctuation of the phoneme strings (a diphone never spans it).
BREAKS = frozenset(";:,.!?¡¿—…\"«»“”()[]-'।॥")


def nfd(text: str) -> str:
    return unicodedata.normalize("NFD", text)


def is_modifier(ch: str) -> bool:
    return ch in _MODIFIERS or unicodedata.combining(ch) != 0 or unicodedata.category(ch) == "Mn"


def split_phones(phonemes: str, *, stress: bool = False) -> list[list[str]]:
    """Clauses of phones for a phoneme string.

    A phone is a base symbol with the marks that follow it (length,
    aspiration, nasalisation and other diacritics); symbols joined by a tie
    bar are one phone. Stress marks are dropped unless ``stress`` is set,
    in which case a mark is put in front of the next phone. Words do not
    split a clause; punctuation does.

    >>> split_phones("bʰˈaːɾət meː, hɛː.")
    [['bʰ', 'aː', 'ɾ', 'ə', 't', 'm', 'eː'], ['h', 'ɛː']]
    """
    clauses: list[list[str]] = []
    cur: list[str] = []
    pending = ""
    tie = False
    text = nfd(phonemes)
    for at, ch in enumerate(text):
        if ch == "." and cur and cur[-1].endswith("r") and at + 1 < len(text) and not text[at + 1].isspace() and text[at + 1] not in BREAKS:
            # The reference's "r." (a retroflex flap) inside a word.
            cur[-1] += ch
            continue
        if ch in STRESS:
            if stress:
                pending += ch
            continue
        if ch.isspace():
            tie = False
            continue
        if ch in BREAKS:
            if cur:
                clauses.append(cur)
            cur, pending, tie = [], "", False
            continue
        if ch in _TIES:
            if cur:
                cur[-1] += ch
                tie = True
            continue
        if cur and (tie or is_modifier(ch)):
            cur[-1] += ch
            tie = False
            continue
        cur.append(pending + ch)
        pending = ""
    if cur:
        clauses.append(cur)
    return clauses


def diphones(clauses: Iterable[list[str]]) -> Iterator[tuple[str, str]]:
    """Pairs of neighbouring phones inside each clause."""
    for clause in clauses:
        yield from zip(clause, clause[1:])


def diphone_name(pair: tuple[str, str]) -> str:
    return f"{pair[0]}-{pair[1]}"


# The script each language is written in.
_SCRIPT = {
    "DEVANAGARI": ("hi", "mr", "ne", "bho", "mai", "mag", "kok", "sa", "doi", "brx"),
    "BENGALI": ("bn", "as"),
    "TELUGU": ("te",),
    "KANNADA": ("kn",),
    "TAMIL": ("ta",),
    "MALAYALAM": ("ml",),
    "GUJARATI": ("gu",),
    "GURMUKHI": ("pa",),
    "ORIYA": ("or",),
    "SINHALA": ("si",),
    "ARABIC": ("ur", "sd"),
    "OL": ("sat",),
    "MEETEI": ("mni",),
    "LATIN": ("en", "en-us", "en-gb", "es", "de", "nl", "fr", "it", "pt", "pt-br", "pl", "tr", "cs", "sv", "ca", "ro", "hu", "fi", "nb", "da", "fr-be", "fr-ch"),
}
_LANG_SCRIPT = {lang: script for script, langs in _SCRIPT.items() for lang in langs}
# Scripts read as the language's own besides the main one.
_ALSO = {"bho": ("KAITHI",), "mai": ("TIRHUTA",), "mni": ("BENGALI",)}


def script_of_language(lang: str) -> str | None:
    """The Unicode script name a language is written in, when known."""
    tag = lang.lower().replace("_", "-")
    return _LANG_SCRIPT.get(tag) or _LANG_SCRIPT.get(tag.split("-")[0])


_script_cache: dict[str, str] = {}


def script_of(ch: str) -> str:
    """Script of a letter: the first word of its Unicode name; ``""`` for
    anything that is not a letter or a mark."""
    s = _script_cache.get(ch)
    if s is None:
        s = ""
        if unicodedata.category(ch)[0] in "LM":
            name = unicodedata.name(ch, "")
            s = name.split(" ")[0] if name else "UNKNOWN"
            if s in ("CJK", "HIRAGANA", "KATAKANA", "HANGUL"):
                s = "CJK"
        _script_cache[ch] = s
    return s


def scripts(text: str) -> dict[str, int]:
    """Letters per script in ``text`` (combining marks follow their base)."""
    out: dict[str, int] = {}
    for ch in text:
        if unicodedata.category(ch)[0] != "L":
            continue
        s = script_of(ch)
        out[s] = out.get(s, 0) + 1
    return out


def foreign_scripts(text: str, lang: str) -> dict[str, int]:
    """Letters of ``text`` that are in another script than the language's."""
    own = script_of_language(lang)
    found = scripts(text)
    if own is None:
        # Unknown language: the script most of the text is in counts as its own.
        if len(found) < 2:
            return {}
        own = max(found, key=lambda k: found[k])
    also = _ALSO.get(lang.lower().split("-")[0], ())
    return {s: n for s, n in found.items() if s != own and s not in also}


_SENTENCE_END = frozenset(".!?।॥۔…")
_CLOSERS = frozenset("\"'”’»)]")


def split_sentences(text: str) -> list[str]:
    """Sentences of running text: split after ``. ! ? । ॥ ۔ …`` followed by
    white space, and at line ends. A full stop between digits or after a
    single letter (an initial) does not split."""
    out: list[str] = []
    for line in text.splitlines():
        start = 0
        i = 0
        n = len(line)
        while i < n:
            if line[i] in _SENTENCE_END:
                j = i + 1
                while j < n and (line[j] in _SENTENCE_END or line[j] in _CLOSERS):
                    j += 1
                initial = line[i] == "." and i >= 1 and line[i - 1].isalpha() and (i < 2 or not line[i - 2].isalpha()) and line[i - 1].isupper()
                if (j == n or line[j].isspace()) and not initial:
                    s = line[start:j].strip()
                    if s:
                        out.append(s)
                    start = j
                i = j
            else:
                i += 1
        s = line[start:].strip()
        if s:
            out.append(s)
    return out


def text_words(text: str) -> list[str]:
    """Words of a text: split at white space, punctuation and symbols
    stripped from both ends, tokens without a letter dropped."""
    out = []
    for tok in text.split():
        a, b = 0, len(tok)
        while a < b and unicodedata.category(tok[a])[0] in "PSZC":
            a += 1
        while b > a and unicodedata.category(tok[b - 1])[0] in "PSZC":
            b -= 1
        w = tok[a:b]
        if any(unicodedata.category(c)[0] == "L" for c in w):
            out.append(w)
    return out
