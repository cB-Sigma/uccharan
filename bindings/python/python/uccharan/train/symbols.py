"""Symbol tables: the set of symbols a voice is trained on and their ids."""

from __future__ import annotations

import json
import os
from collections.abc import Iterable, Mapping

from .. import _native
from ._phones import nfd

__all__ = ["SymbolTable"]

PAD, BOS, EOS = "_", "^", "$"


class SymbolTable:
    """Symbols and their ids.

    Every symbol a voice sees must be one Unicode code point: Piper's
    ``phoneme_type: text`` splits the phoneme text into code points, and
    sherpa-onnx takes phonemes as code points. Entries longer than that
    (such as the vowel clusters in some Piper configs) are kept, listed by
    :meth:`multi_codepoint`, and never produced by the labelling."""

    def __init__(self, ids: Mapping[str, int], *, source: str = "", pad: str = PAD, bos: str = BOS, eos: str = EOS) -> None:
        self.ids: dict[str, int] = dict(ids)
        self.source = source
        self.pad, self.bos, self.eos = pad, bos, eos

    # -- construction -------------------------------------------------------

    @classmethod
    def from_symbols(cls, symbols: Iterable[str], *, source: str = "dataset") -> SymbolTable:
        """Pad ``_``, sentence start ``^``, end ``$`` and the space, then the
        symbols in code point order (the layout Piper and icefall expect)."""
        ids = {PAD: 0, BOS: 1, EOS: 2, " ": 3}
        for s in sorted(set(symbols)):
            if s not in ids:
                ids[s] = len(ids)
        return cls(ids, source=source)

    @classmethod
    def matcha(cls) -> SymbolTable:
        """The fixed symbol table of upstream Matcha-TTS (``matcha/text/symbols.py``)."""
        ids = {PAD: 0}
        for cp in range(0x20, 0x2C80):
            ch = chr(cp)
            got = _native.matcha_ids(ch)
            if len(got) == 3:
                ids[ch] = got[1]
        return cls(ids, source="matcha", bos="", eos="")

    @classmethod
    def from_file(cls, path: str | os.PathLike[str]) -> SymbolTable:
        """Read a Piper voice config (``phoneme_id_map`` inside), a JSON map
        of symbol to id (or list of ids), or a ``tokens.txt`` with one
        ``symbol id`` per line (a line with only an id is the space)."""
        name = os.fspath(path)
        with open(name, encoding="utf-8-sig") as fh:
            text = fh.read()
        ids: dict[str, int] = {}
        if text.lstrip().startswith("{"):
            data = json.loads(text)
            data = data.get("phoneme_id_map", data)
            for sym, value in data.items():
                if isinstance(value, list):
                    if not value:
                        continue
                    value = value[0]
                ids[str(sym)] = int(value)
        else:
            for n, line in enumerate(text.splitlines(), start=1):
                if not line.strip():
                    continue
                parts = line.rstrip("\r\n").split()
                try:
                    if len(parts) == 1:
                        ids[" "] = int(parts[0])
                    elif len(parts) == 2:
                        ids[parts[0]] = int(parts[1])
                    else:
                        raise ValueError("expected 'symbol id'")
                except ValueError as e:
                    raise ValueError(f"{name}:{n}: {e}") from e
        if not ids:
            raise ValueError(f"{name}: no symbols found")
        return cls(ids, source=os.path.basename(name), bos=BOS if BOS in ids else "", eos=EOS if EOS in ids else "")

    # -- queries ------------------------------------------------------------

    def __len__(self) -> int:
        return len(self.ids)

    def __contains__(self, symbol: object) -> bool:
        return symbol in self.ids

    @property
    def num_symbols(self) -> int:
        """One more than the largest id (what a model's embedding needs)."""
        return max(self.ids.values()) + 1 if self.ids else 0

    def multi_codepoint(self) -> list[str]:
        """Entries that are more than one code point after NFD."""
        return [s for s in self.ids if len(nfd(s)) != 1]

    def outside(self, phonemes: str) -> list[str]:
        """Code points of ``phonemes`` (NFD) that the table does not have."""
        return [c for c in nfd(phonemes) if c not in self.ids]

    def extend(self, symbols: Iterable[str]) -> list[str]:
        """Give new ids to the symbols the table lacks; returns those added."""
        added = []
        for s in sorted(set(symbols)):
            if s not in self.ids:
                self.ids[s] = self.num_symbols
                added.append(s)
        return added

    def encode(self, phonemes: str, *, framing: str = "piper") -> list[int]:
        """Ids for a phoneme string; symbols outside the table are skipped.

        ``piper``: ``[BOS, PAD, s, PAD, …, EOS]`` (what Piper builds from
        ``phoneme_type: text``, and what its ``phoneme_ids`` data type
        expects to be given). ``blank``: ``[PAD, s, PAD, …]`` (Matcha-TTS and
        the icefall recipes with interspersed blanks). ``plain``: the ids."""
        seq = [self.ids[c] for c in nfd(phonemes) if c in self.ids]
        if framing == "plain":
            return seq
        pad = self.ids[self.pad]
        out: list[int] = []
        if framing == "piper":
            out.append(self.ids[self.bos])
            out.append(pad)
            for i in seq:
                out.extend((i, pad))
            out.append(self.ids[self.eos])
            return out
        if framing == "blank":
            out.append(pad)
            for i in seq:
                out.extend((i, pad))
            return out
        raise ValueError(f"unknown framing {framing!r} (expected piper, blank or plain)")

    # -- output -------------------------------------------------------------

    def by_id(self) -> list[tuple[str, int]]:
        return sorted(self.ids.items(), key=lambda kv: (kv[1], kv[0]))

    def to_piper_json(self) -> str:
        """``{symbol: [id]}`` for Piper's ``--data.phonemes_path`` (and the
        ``phoneme_id_map`` of a voice config)."""
        return json.dumps({s: [i] for s, i in self.by_id()}, ensure_ascii=False, indent=1) + "\n"

    def to_tokens(self) -> str:
        """``tokens.txt`` as icefall and sherpa-onnx read it: ``symbol id``
        per line, in id order."""
        return "".join(f"{s} {i}\n" for s, i in self.by_id())
