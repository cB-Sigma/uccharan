"""Dataset labelling: text to training-ready phonemes.

>>> from uccharan.train import Utterance, label
>>> done = label([Utterance("u1", "नमस्ते दुनिया।", audio="u1.wav")], "hi")
>>> done.utterances[0].phonemes
'nəmˈʌsteː dˈʊnɪjˌaː.'
>>> done.utterances[0].warnings
[]

:func:`label` normalises and phonemises every utterance, collects the
symbol table and the warnings; :meth:`Labelled.write` writes the files for
Piper, Matcha-TTS and icefall-style recipes, and ``voice.json``.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
from collections import Counter
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import uccharan

from ..textnorm import WordList
from ._phones import foreign_scripts, nfd, split_phones
from .manifest import Utterance
from .symbols import SymbolTable

__all__ = ["Labelled", "LabelledUtterance", "UtteranceWarning", "label"]

PROFILES = ("piper", "sherpa-onnx", "espeak-ipa", "matcha")
_NUMBER_KINDS = frozenset(("number", "decimal", "ordinal", "year", "money", "percent", "time", "date", "phone", "digits", "range", "fraction", "measure"))
_DIGIT = re.compile(r"\d")
# Digits read one at a time: from this many on, the utterance is flagged.
_LONG_DIGITS = 8


@dataclass(frozen=True)
class UtteranceWarning:
    """Something to look at in one utterance."""

    code: str
    """``unknown-character``, ``unread-number``, ``long-number``,
    ``mixed-script``, ``outside-symbols``, ``unread-word``, ``no-phonemes``
    or ``reserved-character``."""
    detail: str

    def __str__(self) -> str:
        return f"{self.code}: {self.detail}"


@dataclass
class LabelledUtterance:
    id: str
    audio: str
    speaker: str
    lang: str
    text: str
    normalized: str
    """The spoken form of the text (numbers and symbols written out)."""
    phonemes: str
    """NFD; every code point is one symbol."""
    warnings: list[UtteranceWarning] = field(default_factory=list)
    duration: float | None = None
    line: int = 0

    def record(self) -> dict[str, object]:
        r: dict[str, object] = {
            "id": self.id,
            "audio": self.audio,
            "lang": self.lang,
            "text": self.text,
            "normalized_text": self.normalized,
            "phonemes": self.phonemes,
            "tokens": list(self.phonemes),
        }
        if self.speaker:
            r["speaker"] = self.speaker
        if self.duration is not None:
            r["duration"] = self.duration
        if self.warnings:
            r["warnings"] = [{"code": w.code, "detail": w.detail} for w in self.warnings]
        return r


@dataclass
class Labelled:
    """A labelled dataset."""

    utterances: list[LabelledUtterance]
    symbols: SymbolTable
    lang: str
    profile: str
    improved: bool
    normalize: str
    hinglish: bool
    stress: bool
    replace: dict[str, str]
    counts: Counter[str]
    """How often each symbol occurs."""
    outside: Counter[str]
    """Symbols that the target table does not have (after ``replace``)."""
    added: list[str]
    """Symbols given fresh ids in a table read from a file (``extend``)."""
    wordlists: list[dict[str, object]]

    # -- summaries ----------------------------------------------------------

    def warning_counts(self) -> dict[str, int]:
        c: Counter[str] = Counter(w.code for u in self.utterances for w in u.warnings)
        return dict(sorted(c.items()))

    def speakers(self) -> list[str]:
        return sorted({u.speaker for u in self.utterances if u.speaker})

    def languages(self) -> list[str]:
        return sorted({u.lang for u in self.utterances} | {self.lang})

    def voice(self) -> dict[str, Any]:
        """The ``voice.json`` stub: what a voice trained on these labels must
        record so that it is driven by the same phonemes later."""
        speakers = self.speakers()
        return {
            "language": self.lang,
            "languages": self.languages(),
            "phoneme_type": "text",
            "phonemizer": {
                "name": "uccharan",
                "version": uccharan.__version__,
                "mode": "improved" if self.improved else "compatible",
                "profile": self.profile,
                "normalize": self.normalize,
                "hinglish": self.hinglish,
                "stress": self.stress,
                "replace": self.replace,
                "word_lists": self.wordlists,
            },
            "symbols": {
                "count": len(self.symbols),
                "num_symbols": self.symbols.num_symbols,
                "source": self.symbols.source,
                "single_codepoint": not self.symbols.multi_codepoint(),
                "multi_codepoint": self.symbols.multi_codepoint(),
                "pad": self.symbols.pad,
                "bos": self.symbols.bos,
                "eos": self.symbols.eos,
            },
            "phoneme_id_map": {s: [i] for s, i in self.symbols.by_id()},
            "num_speakers": max(1, len(speakers)),
            "speakers": speakers,
            "dataset": {
                "utterances": len(self.utterances),
                "with_warnings": sum(1 for u in self.utterances if u.warnings),
                "warnings": self.warning_counts(),
                "outside_symbols": dict(sorted(self.outside.items())),
            },
        }

    def report(self) -> dict[str, Any]:
        return {
            "utterances": len(self.utterances),
            "symbols": {s: self.counts.get(s, 0) for s, _ in self.symbols.by_id()},
            "unused_symbols": [s for s, _ in self.symbols.by_id() if not self.counts.get(s) and s not in (self.symbols.pad, self.symbols.bos, self.symbols.eos)],
            "outside_symbols": {s: {"count": n, "codepoint": f"U+{ord(s):04X}"} for s, n in sorted(self.outside.items())},
            "added_symbols": self.added,
            "multi_codepoint_symbols": self.symbols.multi_codepoint(),
            "warnings": self.warning_counts(),
            "utterances_with_warnings": [{"id": u.id, "line": u.line, "warnings": [str(w) for w in u.warnings]} for u in self.utterances if u.warnings],
        }

    # -- files --------------------------------------------------------------

    def write(
        self,
        out_dir: str | os.PathLike[str],
        *,
        formats: Sequence[str] = ("piper", "matcha", "icefall"),
        validation: int = 0,
        skip_warnings: Sequence[str] = ("no-phonemes",),
    ) -> list[Path]:
        """Write the training files; returns the paths written.

        ``formats``: ``piper`` (``piper/metadata.csv`` for
        ``--data.phoneme_type text``, ``piper/metadata_phoneme_ids.csv`` for
        ``--data.data_type phoneme_ids``, ``piper/phonemes.json`` for
        ``--data.phonemes_path``), ``matcha`` (``matcha/filelist.txt``, and
        ``train.txt``/``val.txt`` when ``validation`` utterances are held
        out, ``matcha/tokens.txt``), ``icefall`` (``icefall/tokens.txt`` and
        ``icefall/utterances.jsonl`` with a ``tokens`` list per utterance).
        ``labels.jsonl``, ``voice.json`` and ``report.json`` are always
        written. Utterances with a warning listed in ``skip_warnings`` are
        left out of the training files (they stay in ``labels.jsonl``)."""
        out = Path(out_dir)
        out.mkdir(parents=True, exist_ok=True)
        written: list[Path] = []

        def put(rel: str, text: str) -> None:
            p = out / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text, encoding="utf-8", newline="\n")
            written.append(p)

        skip = set(skip_warnings)
        keep = [u for u in self.utterances if not any(w.code in skip for w in u.warnings)]
        multi = len(self.speakers()) > 1
        speaker_ids = {s: i for i, s in enumerate(self.speakers())}

        put("labels.jsonl", "".join(json.dumps(u.record(), ensure_ascii=False) + "\n" for u in self.utterances))
        voice = self.voice()
        voice["dataset"]["written"] = len(keep)
        put("voice.json", json.dumps(voice, ensure_ascii=False, indent=1) + "\n")
        put("report.json", json.dumps(self.report(), ensure_ascii=False, indent=1) + "\n")

        for fmt in formats:
            if fmt == "piper":
                if not self.symbols.bos or not self.symbols.eos:
                    raise ValueError("the Piper files need a symbol table with ^ and $ (sentence start and end)")
                rows, id_rows = [], []
                for u in keep:
                    name = u.audio or u.id
                    head = f"{name}|{u.speaker or 'speaker'}|" if multi else f"{name}|"
                    rows.append(f"{head}{u.phonemes}\n")
                    ids = " ".join(map(str, self.symbols.encode(u.phonemes, framing="piper")))
                    id_rows.append(f"{head}{_one_field(u.normalized)}|{ids}\n")
                put("piper/metadata.csv", "".join(rows))
                put("piper/metadata_phoneme_ids.csv", "".join(id_rows))
                put("piper/phonemes.json", self.symbols.to_piper_json())
            elif fmt == "matcha":
                lines = []
                for u in keep:
                    name = u.audio or u.id
                    spk = f"{speaker_ids.get(u.speaker, 0)}|" if multi else ""
                    lines.append(f"{name}|{spk}{u.phonemes}\n")
                put("matcha/filelist.txt", "".join(lines))
                if validation > 0:
                    order = sorted(range(len(keep)), key=lambda i: hashlib.sha1(keep[i].id.encode()).hexdigest())
                    held = set(order[: min(validation, max(0, len(keep) - 1))])
                    put("matcha/train.txt", "".join(line for i, line in enumerate(lines) if i not in held))
                    put("matcha/val.txt", "".join(line for i, line in enumerate(lines) if i in held))
                put("matcha/tokens.txt", self.symbols.to_tokens())
            elif fmt == "icefall":
                put("icefall/tokens.txt", self.symbols.to_tokens())
                recs = []
                for u in keep:
                    r = u.record()
                    r.pop("warnings", None)
                    r["token_ids"] = self.symbols.encode(u.phonemes, framing="plain")
                    recs.append(json.dumps(r, ensure_ascii=False) + "\n")
                put("icefall/utterances.jsonl", "".join(recs))
            else:
                raise ValueError(f"unknown format {fmt!r} (expected piper, matcha or icefall)")
        return written


def _one_field(text: str) -> str:
    """Text safe for one ``|``-separated field."""
    return " ".join(text.replace("|", " ").replace('"', "").split())


def _wordlist_info(lists: Sequence[WordList | str | os.PathLike[str]]) -> tuple[list[WordList], list[dict[str, object]]]:
    loaded: list[WordList] = []
    info: list[dict[str, object]] = []
    for w in lists:
        if isinstance(w, WordList):
            wl, name = w, ""
        elif isinstance(w, os.PathLike) or (isinstance(w, str) and "\n" not in w and os.path.exists(w)):
            wl, name = WordList.from_file(w), os.path.basename(os.fspath(w))
        else:
            wl, name = WordList(str(w)), ""
        loaded.append(wl)
        entry: dict[str, object] = {"entries": len(wl), "sha256": hashlib.sha256(wl.to_tsv().encode()).hexdigest()}
        if name:
            entry["file"] = name
        info.append(entry)
    return loaded, info


def _join(out: object) -> str:
    if isinstance(out, str):
        return out
    return " ".join(s.strip() for s in out if s.strip())  # type: ignore[attr-defined]


def label(
    utterances: Iterable[Utterance],
    lang: str = "en-us",
    *,
    profile: str = "piper",
    improved: bool = False,
    normalize: str = "natural",
    hinglish: bool = False,
    wordlists: Sequence[WordList | str | os.PathLike[str]] = (),
    symbols: SymbolTable | str | os.PathLike[str] | None = None,
    extend: bool = False,
    replace: Mapping[str, str] | None = None,
    stress: bool = True,
) -> Labelled:
    """Normalise and phonemise a dataset.

    ``lang`` is the language of rows that name none. ``profile`` chooses
    the phoneme string: ``piper`` / ``sherpa-onnx`` (sentences with their
    clause punctuation, as those voices are fed), ``espeak-ipa`` or
    ``matcha``. ``improved`` selects the mode; a voice has to be driven by
    the mode it was trained on. ``wordlists`` are pronunciation word lists
    (:class:`uccharan.WordList`, their text, or paths).

    ``symbols`` is the target symbol set: a :class:`SymbolTable`, a file
    (Piper voice config, JSON map or ``tokens.txt``), or ``None`` to build
    the table from the data. Symbols outside a given table are reported in
    ``outside`` and in the utterance warnings; with ``extend`` they get new
    ids instead. ``replace`` rewrites symbols first (``{"ʱ": "ɦ"}`` writes
    breathy stops the way a table without ``ʱ`` needs). ``stress=False``
    drops the stress marks."""
    if profile not in PROFILES:
        raise ValueError(f"unknown profile {profile!r} for labelling (expected one of {', '.join(PROFILES)})")
    rows = list(utterances)
    lists, list_info = _wordlist_info(wordlists)
    repl = {nfd(k): nfd(v) for k, v in (replace or {}).items()}
    for k in repl:
        if not k:
            raise ValueError("replace: empty symbol")

    by_lang: dict[str, list[int]] = {}
    for i, u in enumerate(rows):
        by_lang.setdefault(u.lang or lang, []).append(i)

    phonemes: list[str] = [""] * len(rows)
    for lg, idx in by_lang.items():
        if lists:
            for i in idx:
                phonemes[i] = _join(
                    uccharan.phonemize_wordlists(rows[i].text, lg, profile=profile, wordlists=lists, normalize=normalize, improved=improved, hinglish=hinglish)
                )
        else:
            got = uccharan.phonemize_batch([rows[i].text for i in idx], lg, profile, normalize=normalize, improved=improved, hinglish=hinglish)
            for i, p in zip(idx, got):
                phonemes[i] = p

    out: list[LabelledUtterance] = []
    counts: Counter[str] = Counter()
    for i, u in enumerate(rows):
        lg = u.lang or lang
        warnings: list[UtteranceWarning] = []
        p = nfd(" ".join(phonemes[i].split()))
        for k, v in repl.items():
            p = p.replace(k, v)
        if not stress:
            p = p.replace("ˈ", "").replace("ˌ", "")
        if "|" in p or '"' in p:
            warnings.append(UtteranceWarning("reserved-character", "| or \" removed from the phonemes"))
            p = " ".join(p.replace("|", " ").replace('"', " ").split())
        norm = uccharan.normalize(u.text, lg, improved=improved, wordlists=lists)
        # With normalisation off, numbers and symbols are not read at all.
        spans = norm.spans if normalize != "off" else []
        spoken = norm.text.strip() if normalize != "off" else u.text
        dropped = [s.text for s in spans if s.kind == "dropped" and s.text.strip()]
        if dropped:
            warnings.append(UtteranceWarning("unknown-character", " ".join(dict.fromkeys(dropped))))
        unread = [s.text for s in spans if s.kind in _NUMBER_KINDS and (not s.spoken.strip() or _DIGIT.search(s.spoken))]
        if not unread and _DIGIT.search(spoken):
            unread = re.findall(r"\d+", spoken)
        if unread:
            warnings.append(UtteranceWarning("unread-number", " ".join(dict.fromkeys(unread))))
        long_digits = [s.text for s in spans if s.kind in ("phone", "digits", "number") and sum(c.isdigit() for c in s.text) >= _LONG_DIGITS]
        if long_digits:
            warnings.append(UtteranceWarning("long-number", " ".join(dict.fromkeys(long_digits))))
        foreign = foreign_scripts(u.text, lg)
        if foreign:
            warnings.append(UtteranceWarning("mixed-script", ", ".join(f"{s.capitalize()} ({n} letters)" for s, n in sorted(foreign.items()))))
        if not any(split_phones(p)):
            warnings.append(UtteranceWarning("no-phonemes", "nothing to say"))
        counts.update(p)
        out.append(LabelledUtterance(id=u.id, audio=u.audio, speaker=u.speaker, lang=lg, text=u.text, normalized=spoken, phonemes=p, warnings=warnings, duration=u.duration, line=u.line))

    added: list[str] = []
    if symbols is None:
        table = SymbolTable.from_symbols(counts)
    else:
        table = symbols if isinstance(symbols, SymbolTable) else SymbolTable.from_file(symbols)
        if extend:
            added = table.extend(c for c in counts if c not in table)
    outside: Counter[str] = Counter({c: n for c, n in counts.items() if c not in table})
    if outside:
        for lu in out:
            bad = dict.fromkeys(c for c in lu.phonemes if c in outside)
            if bad:
                lu.warnings.append(UtteranceWarning("outside-symbols", " ".join(f"{c} (U+{ord(c):04X})" for c in bad)))
    return Labelled(
        utterances=out,
        symbols=table,
        lang=lang,
        profile=profile,
        improved=improved,
        normalize=normalize,
        hinglish=hinglish,
        stress=stress,
        replace=dict(repl),
        counts=counts,
        outside=outside,
        added=added,
        wordlists=list_info,
    )
