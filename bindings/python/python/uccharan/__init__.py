"""uccharan: permissively licensed text-to-phoneme conversion for neural TTS.

>>> import uccharan
>>> uccharan.phonemize("Hello, world.")
'həlˈoʊ, wˈɜːld.'
>>> uccharan.phonemize("नमस्ते दुनिया", lang="hi", profile="sherpa-onnx")[0][:3]
[110, 601, 109]

Languages: ``en-us``, ``en-gb`` (RP) and ``hi`` (espeak voice names such as
``en-gb-x-rp`` are accepted). Profiles: ``espeak-ipa``, ``piper``, ``kokoro``,
``kitten``, ``matcha`` and ``sherpa-onnx``; see docs/profiles.md.
"""

from __future__ import annotations

import json
import os
import re
from collections.abc import Iterable, Mapping
from typing import Literal, NamedTuple, Union, overload

from . import _native
from . import packs as _packs
from .packs import PackError

__version__: str = _native.__version__

Lang = Literal["en-us", "en-gb", "hi"]
Profile = Literal["espeak-ipa", "piper", "kokoro", "kitten", "matcha", "sherpa-onnx"]
NormalizeMode = Literal["natural", "espeak", "off"]

LANGUAGES: tuple[str, ...] = ("en-us", "en-gb", "hi")
PROFILES: tuple[str, ...] = ("espeak-ipa", "piper", "kokoro", "kitten", "matcha", "sherpa-onnx")

__all__ = [
    "LANGUAGES",
    "PROFILES",
    "PackError",
    "Word",
    "__version__",
    "analyze",
    "codepoints",
    "has_misaki",
    "kitten_ids",
    "load_misaki",
    "kokoro_ids",
    "load_pack",
    "matcha_ids",
    "phonemize",
    "phonemize_batch",
    "piper_ids",
    "sentences",
    "unmapped",
]


_ready: set[tuple[str, str]] = set()


_LATIN = re.compile(r"[A-Za-z]")


def _need(lang: str, profile: str = "espeak-ipa", text: object = "") -> None:
    # en-gb and the Kokoro English data are packs, fetched on first use.
    # Hindi reads English words in Latin script with the en-gb data.
    key = (lang, "kokoro" if profile == "kokoro" else "")
    if key not in _ready:
        _packs.ensure(lang, profile)
        _ready.add(key)
    if ("en-gb", "") not in _ready and lang.lower() in ("hi", "hi-in", "h", "hin"):
        texts = [text] if isinstance(text, str) else text
        if any(_LATIN.search(t) for t in texts):  # type: ignore[union-attr]
            _need("en-gb")


def load_pack(name_or_path: Union[str, "os.PathLike[str]"]) -> str:
    """Load a language pack: a pack name (``en-gb``, ``kokoro-en-us``,
    ``kokoro-en-gb``; fetched and checked as described in
    :mod:`uccharan.packs`) or the path of a ``.ucd`` file (used as is).
    Returns the pack's language. Packs are also loaded on first use."""
    name = os.fspath(name_or_path)
    if name in _packs.PACKS:
        return _native.load_pack(_packs.fetch(name))
    with open(name, "rb") as fh:
        return _native.load_pack(fh.read())


class Word(NamedTuple):
    """One analysed word."""

    text: str
    ipa: str
    lang: str
    glued: bool


@overload
def phonemize(
    text: str,
    lang: str = ...,
    profile: Literal["espeak-ipa", "kokoro", "kitten", "matcha"] = ...,
    *,
    normalize: NormalizeMode = ...,
    improved: bool = ...,
    hinglish: bool = ...,
    punctuation: bool = ...,
    language_flags: bool = ...,
) -> str: ...
@overload
def phonemize(
    text: str,
    lang: str,
    profile: Literal["piper"],
    *,
    normalize: NormalizeMode = ...,
    improved: bool = ...,
    hinglish: bool = ...,
    punctuation: bool = ...,
    language_flags: bool = ...,
) -> list[str]: ...
@overload
def phonemize(
    text: str,
    lang: str,
    profile: Literal["sherpa-onnx"],
    *,
    normalize: NormalizeMode = ...,
    improved: bool = ...,
    hinglish: bool = ...,
    punctuation: bool = ...,
    language_flags: bool = ...,
) -> list[list[int]]: ...
def phonemize(
    text: str,
    lang: str = "en-us",
    profile: str = "espeak-ipa",
    *,
    normalize: str = "natural",
    improved: bool = False,
    hinglish: bool = False,
    punctuation: bool = True,
    language_flags: bool = False,
) -> Union[str, list[str], list[list[int]]]:
    """Convert text to phonemes.

    The return type depends on the profile:

    * ``espeak-ipa``: one IPA string, words separated by spaces. Clause
      punctuation is kept unless ``punctuation=False``.
    * ``piper``: a list with one NFD string per sentence, clause terminators
      included (feed each to :func:`piper_ids`).
    * ``sherpa-onnx``: a list with one list of Unicode codepoints per sentence,
      for ``GenerationConfig.phoneme_codepoints``.
    * ``kokoro``: the Kokoro token string (feed to :func:`kokoro_ids`).
    * ``kitten`` / ``matcha``: the phoneme string with punctuation (feed to
      :func:`kitten_ids` / :func:`matcha_ids`).

    ``normalize`` chooses how numbers and symbols are read (``natural``,
    ``espeak`` for the reference phonemizer's readings, ``off``).
    ``improved=True`` switches from reference-compatible output to the
    cleaner readings described in docs/improved-mode.md. ``hinglish=True``
    (Hindi only, experimental) detects romanised Hindi. ``language_flags=True``
    brackets words read by another language's rules with ``(en)``/``(hi)``
    markers in ``espeak-ipa`` output.
    """
    _need(lang, profile, text)
    if profile == "piper":
        return _native.sentences(text, lang, normalize, improved, hinglish)
    if profile == "sherpa-onnx":
        return _native.codepoints(text, lang, normalize, improved, hinglish)
    return _native.render(text, lang, profile, normalize, improved, hinglish, punctuation, language_flags)


def phonemize_batch(
    texts: Iterable[str],
    lang: str = "en-us",
    profile: str = "espeak-ipa",
    *,
    normalize: str = "natural",
    improved: bool = False,
    hinglish: bool = False,
    punctuation: bool = True,
) -> list[str]:
    """Phonemize many texts on all CPU cores (the GIL is released).

    Every result is a single string: for ``piper`` the sentences are joined by
    a space, for ``sherpa-onnx`` codepoints are written in decimal, separated
    by spaces, with `` | `` between sentences.
    """
    texts = list(texts)
    _need(lang, profile, texts)
    return _native.render_batch(texts, lang, profile, normalize, improved, hinglish, punctuation)


def sentences(text: str, lang: str = "en-us", *, normalize: str = "natural", improved: bool = False, hinglish: bool = False) -> list[str]:
    """Piper-shaped sentence strings (same as ``phonemize(profile="piper")``)."""
    _need(lang, "", text)
    return _native.sentences(text, lang, normalize, improved, hinglish)


def codepoints(text: str, lang: str = "en-us", *, normalize: str = "natural", improved: bool = False, hinglish: bool = False) -> list[list[int]]:
    """Codepoint lists per sentence (same as ``phonemize(profile="sherpa-onnx")``)."""
    _need(lang, "", text)
    return _native.codepoints(text, lang, normalize, improved, hinglish)


def analyze(text: str, lang: str = "en-us", *, normalize: str = "natural", improved: bool = False, hinglish: bool = False) -> list[list[tuple[list[Word], str]]]:
    """Sentences → clauses → ``(words, terminator)``."""
    _need(lang, "", text)
    raw = _native.analyze(text, lang, normalize, improved, hinglish)
    return [[([Word(*w) for w in words], term) for words, term in sentence] for sentence in raw]


def piper_ids(sentence: str, voice_config: Union[str, "os.PathLike[str]", Mapping[str, object]]) -> tuple[list[int], list[str]]:
    """Phoneme ids for one sentence string in Piper's framing.

    ``voice_config`` is the voice's ``*.onnx.json``: a path, the JSON text, or
    the parsed dict. Returns ``(ids, missing)`` where ``missing`` lists symbols
    that are not in the voice's ``phoneme_id_map`` (Piper skips those).
    """
    if isinstance(voice_config, Mapping):
        text = json.dumps({"phoneme_id_map": voice_config.get("phoneme_id_map", voice_config)})
    else:
        candidate = os.fspath(voice_config)
        if candidate.lstrip().startswith("{"):
            text = candidate
        else:
            with open(candidate, encoding="utf-8") as fh:
                text = fh.read()
    return _native.piper_ids(sentence, text)


def kokoro_ids(tokens: str) -> list[int]:
    """Kokoro-82M input ids ``[0, …, 0]`` for a Kokoro token string."""
    return _native.kokoro_ids(tokens)


def kitten_ids(phonemes: str) -> list[int]:
    """Kitten TTS 0.8 input ids ``[0, …, 10, 0]`` for a phoneme string."""
    return _native.kitten_ids(phonemes)


def matcha_ids(phonemes: str) -> list[int]:
    """Matcha-TTS input ids (interspersed with the blank id 0)."""
    return _native.matcha_ids(phonemes)


def unmapped(text: str, profile: str) -> list[str]:
    """Symbols of ``text`` that the profile's fixed symbol table cannot represent."""
    return _native.unmapped(text, profile)


def load_misaki(path: Union[str, "os.PathLike[str]", None] = None, lang: str = "en-us") -> None:
    """Replace the built-in copy of misaki's English dictionaries.

    uccharan ships misaki 0.9.4's dictionaries for the ``kokoro`` profile; use
    this to load another copy (for example a newer misaki release). ``path``
    is the directory holding ``us_gold.json`` / ``us_silver.json`` (``gb_*``
    for ``en-gb``); by default the ``data`` directory of an installed
    ``misaki`` package (nothing of it is imported). Can be called once per
    accent, before or after other calls.
    """
    if path is None:
        import importlib.util

        spec = importlib.util.find_spec("misaki")
        if spec is None or not spec.submodule_search_locations:
            raise FileNotFoundError("misaki is not installed; pass the directory that holds us_gold.json / us_silver.json")
        path = os.path.join(list(spec.submodule_search_locations)[0], "data")
    accent = "gb" if lang.lower().replace("_", "-") in ("en-gb", "en-gb-x-rp", "b") else "us"
    with open(os.path.join(path, f"{accent}_gold.json"), encoding="utf-8") as fh:
        gold = fh.read()
    with open(os.path.join(path, f"{accent}_silver.json"), encoding="utf-8") as fh:
        silver = fh.read()
    _native.load_misaki(lang, gold, silver)


def has_misaki(lang: str = "en-us") -> bool:
    """True when misaki's dictionaries are available for ``lang`` (built in or loaded)."""
    return _native.has_misaki(lang)

