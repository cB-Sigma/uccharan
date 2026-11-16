"""Command line: ``uccharan [options] [TEXT ...]`` (reads stdin when no text is given)."""

from __future__ import annotations

import argparse
import codecs
import json
import sys
from collections.abc import Iterator, Sequence
from typing import BinaryIO

from . import LANGUAGES, PROFILES, __version__, phonemize


# Byte order marks, longest first (the UTF-32 LE mark starts with the UTF-16 LE one).
_BOMS = (
    (codecs.BOM_UTF32_LE, "utf-32-le"),
    (codecs.BOM_UTF32_BE, "utf-32-be"),
    (codecs.BOM_UTF8, "utf-8"),
    (codecs.BOM_UTF16_LE, "utf-16-le"),
    (codecs.BOM_UTF16_BE, "utf-16-be"),
)


class InputEncodingError(Exception):
    """Standard input is not text in an encoding the command line reads."""


def _decode(data: bytes, encoding: str, where: str, line: int) -> str:
    try:
        return data.decode(encoding)
    except UnicodeDecodeError as exc:
        line += data.count(b"\n", 0, exc.start) if encoding == "utf-8" else 0
        at = f", line {line}" if encoding == "utf-8" else ""
        raise InputEncodingError(
            f"{where} is not valid {encoding.upper()} (byte 0x{data[exc.start]:02x}{at}); "
            "save it as UTF-8, or as UTF-16 with a byte order mark"
        ) from None


def _split(text: str) -> list[str]:
    lines = text.split("\n")
    if lines[-1] == "":
        lines.pop()
    return [line.removesuffix("\r") for line in lines]


def read_lines(stream: BinaryIO, where: str = "standard input") -> Iterator[str]:
    """The lines of ``stream`` without their line endings.

    Text with a UTF-8, UTF-16 or UTF-32 byte order mark is decoded accordingly (the mark is
    dropped); anything else is read as UTF-8. UTF-8 is read line by line, so a pipe is answered
    as it is written; UTF-16 and UTF-32 are read whole. Raises :class:`InputEncodingError`
    when the bytes do not decode.
    """
    head = b""
    while len(head) < 4:
        chunk = stream.read(4 - len(head))
        if not chunk:
            break
        head += chunk
    for bom, encoding in _BOMS:
        if head.startswith(bom):
            break
    else:
        bom, encoding = b"", "utf-8"
    if encoding != "utf-8":
        yield from _split(_decode(head[len(bom) :] + stream.read(), encoding, where, 1))
        return
    pending, number = head[len(bom) :], 1
    while True:
        raw = stream.readline()
        pending += raw
        if raw and not raw.endswith(b"\n"):
            continue
        if pending:
            lines = _split(_decode(pending, "utf-8", where, number))
            number += len(lines)
            yield from lines
            pending = b""
        if not raw:
            return


def main(argv: Sequence[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="uccharan", description="Text to phonemes for neural TTS.")
    ap.add_argument("text", nargs="*", help="text to phonemize; standard input is read line by line when omitted")
    ap.add_argument("-l", "--lang", default="en-us", help=f"language: {', '.join(LANGUAGES)} (default en-us)")
    ap.add_argument("-p", "--profile", default="espeak-ipa", choices=PROFILES)
    ap.add_argument("--normalize", default="natural", choices=["natural", "espeak", "off"])
    ap.add_argument("--improved", action="store_true", help="cleaner, not reference-compatible output")
    ap.add_argument("--hinglish", action="store_true", help="detect romanised Hindi (experimental, lang hi)")
    ap.add_argument("--no-punctuation", action="store_true", help="drop clause punctuation (espeak-ipa)")
    ap.add_argument("--json", action="store_true", help="print one JSON value per input line")
    ap.add_argument("-V", "--version", action="version", version=f"uccharan {__version__}")
    args = ap.parse_args(argv)
    # Phonemes are not in the legacy Windows code pages: always read and write UTF-8.
    for stream in (sys.stdin, sys.stdout, sys.stderr):
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is not None:
            reconfigure(encoding="utf-8")

    def emit(line: str) -> None:
        try:
            out = phonemize(  # type: ignore[call-overload]
                line,
                args.lang,
                args.profile,
                normalize=args.normalize,
                improved=args.improved,
                hinglish=args.hinglish,
                punctuation=not args.no_punctuation,
            )
        except ValueError as exc:
            ap.error(str(exc))
        if args.json or not isinstance(out, str):
            print(json.dumps(out, ensure_ascii=False))
        else:
            print(out)

    if args.text:
        for t in args.text:
            emit(t)
    else:
        stdin = getattr(sys.stdin, "buffer", None)
        try:
            if stdin is None:  # replaced by a text stream, as in some embedding hosts
                for line in sys.stdin:
                    emit(line.rstrip("\n"))
            else:
                for line in read_lines(stdin):
                    emit(line)
        except InputEncodingError as exc:
            sys.stdout.flush()
            print(f"uccharan: {exc}", file=sys.stderr)
            return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
