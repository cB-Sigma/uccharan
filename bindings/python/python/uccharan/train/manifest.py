"""Dataset manifests: LJSpeech-style ``metadata.csv``, JSON lines, and rows
shaped like Hugging Face ``datasets`` records."""

from __future__ import annotations

import json
import os
from collections.abc import Iterable, Mapping
from dataclasses import dataclass
from pathlib import PurePath

__all__ = ["Utterance", "from_rows", "read_manifest", "read_texts"]


@dataclass(frozen=True)
class Utterance:
    """One row of a dataset."""

    id: str
    text: str
    audio: str = ""
    """Audio path as the manifest gives it."""
    lang: str = ""
    """Language of the row; empty for the dataset's language."""
    speaker: str = ""
    duration: float | None = None
    """Seconds, when the manifest gives it."""
    line: int = 0
    """Line (or row) number in the manifest, from 1."""


_AUDIO_KEYS = ("audio_filepath", "audio", "audio_path", "path", "file", "file_name", "wav", "filename")
_TEXT_KEYS = ("text", "sentence", "transcript", "transcription", "raw_text", "normalized_text")
_LANG_KEYS = ("lang", "language", "locale")
_SPEAKER_KEYS = ("speaker", "speaker_id", "spk", "client_id")
_ID_KEYS = ("id", "utt_id", "utterance_id", "key")
_DURATION_KEYS = ("duration", "duration_s", "audio_duration")


def _first(row: Mapping[str, object], keys: Iterable[str]) -> object:
    for k in keys:
        if k in row and row[k] is not None:
            return row[k]
    return None


def from_rows(rows: Iterable[Mapping[str, object]], *, text_key: str | None = None, audio_key: str | None = None) -> list[Utterance]:
    """Utterances from mappings (JSON-lines records, or rows of a Hugging
    Face dataset). The audio is ``audio_filepath``, ``audio`` (a path, or a
    mapping with ``path``), ``path``, ``file`` …; the text is ``text``,
    ``sentence``, ``transcript`` …; ``lang``/``language``, ``speaker``,
    ``id`` and ``duration`` are read when present. Audio is never opened."""
    out: list[Utterance] = []
    for n, row in enumerate(rows, start=1):
        text = row.get(text_key) if text_key else _first(row, _TEXT_KEYS)
        if text is None:
            raise ValueError(f"row {n}: no text column (looked for {', '.join(_TEXT_KEYS)})")
        audio = row.get(audio_key) if audio_key else _first(row, _AUDIO_KEYS)
        if isinstance(audio, Mapping):
            audio = audio.get("path") or ""
        audio = "" if audio is None else str(audio)
        ident = _first(row, _ID_KEYS)
        if ident is None:
            ident = PurePath(audio).stem if audio else f"{n:06d}"
        duration = _first(row, _DURATION_KEYS)
        lang = _first(row, _LANG_KEYS)
        speaker = _first(row, _SPEAKER_KEYS)
        out.append(
            Utterance(
                id=str(ident),
                text=str(text),
                audio=audio,
                lang="" if lang is None else str(lang),
                speaker="" if speaker is None else str(speaker),
                duration=float(duration) if isinstance(duration, (int, float, str)) and str(duration).strip() else None,
                line=n,
            )
        )
    return out


def _read_csv(text: str, layout: str, text_column: int | None) -> list[Utterance]:
    out: list[Utterance] = []
    for n, line in enumerate(text.splitlines(), start=1):
        if not line.strip():
            continue
        cols = line.split("|")
        if len(cols) < 2:
            raise ValueError(f"line {n}: expected at least two columns separated by '|'")
        speaker = ""
        if text_column is not None:
            if not 0 < text_column < len(cols):
                raise ValueError(f"line {n}: no column {text_column}")
            body = cols[text_column]
        elif layout == "piper":
            # file|text or file|speaker|text
            body = cols[-1]
        else:
            # LJSpeech: id|text|normalised text. The raw text is used.
            body = cols[1]
        if layout == "piper" and len(cols) >= 3:
            speaker = cols[1]
        first = cols[0].strip()
        stem = PurePath(first).stem if PurePath(first).suffix.lower() in (".wav", ".flac", ".mp3", ".ogg", ".opus", ".m4a") else first
        out.append(Utterance(id=stem, text=body.strip(), audio=first, speaker=speaker.strip(), line=n))
    return out


def read_manifest(
    path: str | os.PathLike[str],
    *,
    layout: str | None = None,
    text_column: int | None = None,
) -> list[Utterance]:
    """Read a manifest file.

    ``layout``: ``ljspeech`` (``id|text`` or ``id|text|normalised text``; the
    second column is used), ``piper`` (``file|text`` or
    ``file|speaker|text``), ``jsonl`` (one JSON object per line) or ``json``
    (a list of objects). By default the file name decides: ``.jsonl`` and
    ``.json`` are JSON, anything else is ``ljspeech``. ``text_column``
    (from 0) overrides the text column of the ``|`` layouts."""
    name = os.fspath(path)
    with open(name, encoding="utf-8-sig") as fh:
        text = fh.read()
    if layout is None:
        low = name.lower()
        layout = "jsonl" if low.endswith((".jsonl", ".ndjson")) else "json" if low.endswith(".json") else "ljspeech"
    if layout == "jsonl":
        rows = []
        for n, line in enumerate(text.splitlines(), start=1):
            if line.strip():
                try:
                    rows.append(json.loads(line))
                except json.JSONDecodeError as e:
                    raise ValueError(f"line {n}: {e}") from e
        return from_rows(rows)
    if layout == "json":
        data = json.loads(text)
        if isinstance(data, Mapping):
            data = data.get("data") or data.get("rows") or data.get("utterances") or []
        return from_rows(data)
    if layout in ("ljspeech", "piper"):
        return _read_csv(text, layout, text_column)
    raise ValueError(f"unknown manifest layout {layout!r} (expected ljspeech, piper, jsonl or json)")


def read_texts(path: str | os.PathLike[str]) -> list[str]:
    """Lines of a plain text file (blank lines dropped)."""
    with open(path, encoding="utf-8-sig") as fh:
        return [line.strip() for line in fh if line.strip()]
